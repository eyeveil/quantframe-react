//! The data layer: one `DataSource` over the Rust backend in `src-tauri`, in
//! live mode or the backend's own fixture mode.
//!
//! The backend runs on a tokio runtime that this module owns. Every method
//! spawns the backend command on that runtime and returns a future that waits
//! on its `JoinHandle`. A tokio `JoinHandle` can be awaited from any executor,
//! so views await these futures in `cx.spawn` tasks on gpui's executor, and
//! tests await them with `futures::executor::block_on`. The UI thread never
//! runs database or network I/O.

mod api;
mod backend;
mod convert;
mod events;
mod host;

use std::{
    future::Future,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use qf_app_lib::{HeadlessOptions, init_headless};
use tokio::runtime::{Handle, Runtime};

pub use backend::{Backend, BackendStatus};
pub use events::{BackendEvent, DataKind, Trade};
use host::GpuiHost;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataError {
    pub component: String,
    pub message: String,
    /// The underlying error, such as a refused or failed request.
    pub cause: String,
}

impl DataError {
    fn new(component: &str, message: impl Into<String>) -> Self {
        Self {
            component: component.to_owned(),
            message: message.into(),
            cause: String::new(),
        }
    }
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.component, self.message)?;
        if !self.cause.is_empty() {
            write!(f, ": {}", self.cause)?;
        }
        Ok(())
    }
}

impl std::error::Error for DataError {}

impl From<qf_app_lib::Error> for DataError {
    fn from(error: qf_app_lib::Error) -> Self {
        Self {
            component: error.component,
            message: error.message,
            cause: error.cause,
        }
    }
}

impl From<serde_json::Error> for DataError {
    fn from(error: serde_json::Error) -> Self {
        Self::new("DataSource:Decode", error.to_string())
    }
}

pub type DataResult<T> = Result<T, DataError>;

/// Where the backend reads its data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StartOptions {
    /// A fixture directory such as `src-tauri/fixtures`. `None` is live mode.
    pub fixtures: Option<PathBuf>,
    pub storage_dir: Option<PathBuf>,
}

/// The fixture directory debug builds use when no other source is named.
const DEBUG_FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src-tauri/fixtures");

impl StartOptions {
    /// `--fixtures <dir>` wins, then `QF_FIXTURES` and `QF_STORAGE_DIR`. Debug
    /// builds fall back to `src-tauri/fixtures` unless `--live` is passed.
    pub fn from_env() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let flag = args
            .iter()
            .position(|arg| arg == "--fixtures")
            .and_then(|index| args.get(index + 1))
            .map(PathBuf::from);
        let env = HeadlessOptions::from_env();
        let live = args.iter().any(|arg| arg == "--live");
        let fixtures = flag
            .or(env.fixtures)
            .or_else(|| (cfg!(debug_assertions) && !live).then(|| PathBuf::from(DEBUG_FIXTURES)));
        Self {
            fixtures,
            storage_dir: env.storage_dir,
        }
    }

    pub fn fixtures(dir: impl Into<PathBuf>) -> Self {
        Self {
            fixtures: Some(dir.into()),
            storage_dir: None,
        }
    }
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static HOST: OnceLock<Arc<GpuiHost>> = OnceLock::new();

/// Typed access to every backend command. Cheap to clone.
#[derive(Clone)]
pub struct DataSource {
    runtime: Handle,
    host: Arc<GpuiHost>,
    fixture_mode: bool,
    export_lock: Arc<tokio::sync::Mutex<()>>,
}

impl DataSource {
    /// Starts the backend on its own runtime and returns at once. The backend
    /// sends `BackendEvent::Ready` or `BackendEvent::StartupFailed` on the
    /// returned channel. The backend keeps process-wide state, so a second call
    /// returns an error.
    pub fn start(options: StartOptions) -> DataResult<(Self, UnboundedReceiver<BackendEvent>)> {
        let (sender, receiver) = unbounded();
        let host = Arc::new(GpuiHost::new(sender));
        if HOST.set(host.clone()).is_err() {
            return Err(DataError::new(
                "DataSource:Start",
                "The backend is already running",
            ));
        }
        let runtime = RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("qf-backend")
                .enable_all()
                .build()
                .expect("tokio runtime")
        });
        let fixture_mode = options.fixtures.is_some();
        let headless = HeadlessOptions {
            fixtures: options.fixtures,
            storage_dir: options.storage_dir,
            use_temp_db: false,
        };
        let backend_host: Arc<dyn qf_app_lib::Host> = host.clone();
        let startup_host = host.clone();
        runtime.spawn(async move {
            if let Err(error) = init_headless(headless, backend_host).await {
                startup_host.send(BackendEvent::StartupFailed(
                    DataError::from(error).to_string(),
                ));
            }
        });
        Ok((
            Self {
                runtime: runtime.handle().clone(),
                host,
                fixture_mode,
                export_lock: Arc::default(),
            },
            receiver,
        ))
    }

    /// The backend runs on fixtures and refuses network requests.
    pub fn fixture_mode(&self) -> bool {
        self.fixture_mode
    }

    fn call<T, F>(&self, future: F) -> impl Future<Output = DataResult<T>> + Send + 'static
    where
        T: Send + 'static,
        F: Future<Output = DataResult<T>> + Send + 'static,
    {
        let task = self.runtime.spawn(async move {
            if qf_app_lib::HAS_STARTED.get().is_none() {
                return Err(DataError::new(
                    "DataSource",
                    "The backend is still starting",
                ));
            }
            future.await
        });
        async move {
            task.await
                .map_err(|error| DataError::new("DataSource:Task", error.to_string()))?
        }
    }

    /// Runs an export command, which asks the host for a target path, and
    /// answers that request with `path`.
    fn export<F>(
        &self,
        path: PathBuf,
        future: F,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static
    where
        F: Future<Output = Result<String, qf_app_lib::Error>> + Send + 'static,
    {
        let host = self.host.clone();
        let lock = self.export_lock.clone();
        self.call(async move {
            let _guard = lock.lock().await;
            if let Ok(mut slot) = host.export_path.lock() {
                *slot = Some(path.clone());
            }
            let result = future.await;
            if let Ok(mut slot) = host.export_path.lock() {
                slot.take();
            }
            result?;
            Ok(path)
        })
    }
}
