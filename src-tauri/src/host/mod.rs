use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
};

use serde_json::{json, Value};
use tokio::{runtime::Handle, task::JoinHandle};
use utils::{get_location, Error};

use crate::{
    app::AppState, cache::CacheState, live_scraper::LiveScraperState, log_parser::LogParserState,
    wf_inventory::WFInventoryState,
};

#[cfg(feature = "tauri")]
mod tauri_host;
#[cfg(feature = "tauri")]
pub use tauri_host::TauriHost;

pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
pub const PACKAGE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PACKAGE_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const PACKAGE_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/// The front end that receives backend events and answers OS-level requests.
pub trait Host: Send + Sync {
    /// `event` is `"message"` or `"message_update"` for UI events (see `send_event!`),
    /// or a named event such as `"play_sound"` or `"add_trade"`.
    fn emit(&self, event: &str, payload: Value);
    fn notify(&self, title: &str, body: &str);
    fn play_sound(&self, file: &str, volume: f32) {
        self.emit("play_sound", json!({ "file_name": file, "volume": volume }));
    }
    /// Blocks the calling thread until the user picks a target path or cancels.
    /// `suggested` is the directory and file name the dialog opens with.
    fn save_file(
        &self,
        _filter_name: &str,
        _extensions: &[&str],
        _suggested: Option<&Path>,
    ) -> Option<PathBuf> {
        None
    }
    /// Blocks the calling thread until the user picks a file or cancels.
    fn pick_file(&self, _title: &str, _filter_name: &str, _extensions: &[&str]) -> Option<PathBuf> {
        None
    }
    fn show_trade(&self, payload: Value) {
        self.emit("add_trade", payload);
    }
    fn exit(&self, code: i32) {
        std::process::exit(code);
    }
}

static HOST: OnceLock<Arc<dyn Host>> = OnceLock::new();
static OFFLINE: AtomicBool = AtomicBool::new(false);

/// Fixture mode. Requests to warframe.market, the Quantframe API, and webhooks are refused.
pub fn set_offline(offline: bool) {
    OFFLINE.store(offline, Ordering::Relaxed);
}

pub fn is_offline() -> bool {
    OFFLINE.load(Ordering::Relaxed)
}

/// Returns an error in fixture mode. Call before any request to warframe.market.
pub fn ensure_online(component: &str) -> Result<(), Error> {
    if is_offline() {
        return Err(Error::new(
            component,
            "Fixture mode makes no network requests",
            get_location!(),
        ));
    }
    Ok(())
}
static RUNTIME: OnceLock<Handle> = OnceLock::new();

/// Installs the host and the runtime used by `spawn`. Later calls are ignored.
pub fn install(host: Arc<dyn Host>, runtime: Handle) {
    HOST.get_or_init(|| host);
    RUNTIME.get_or_init(|| runtime);
}

pub fn host() -> Option<&'static Arc<dyn Host>> {
    HOST.get()
}

pub fn emit(event: &str, payload: Value) -> Result<(), Error> {
    match HOST.get() {
        Some(host) => {
            host.emit(event, payload);
            Ok(())
        }
        None => Err(Error::new(
            "Host:Emit",
            format!("No host installed, dropped event {event}"),
            get_location!(),
        )),
    }
}

pub fn notify(title: &str, body: &str) {
    if let Some(host) = HOST.get() {
        host.notify(title, body);
    }
}

pub fn play_sound(file: &str, volume: f32) {
    if let Some(host) = HOST.get() {
        host.play_sound(file, volume);
    }
}

pub fn save_file(
    filter_name: &str,
    extensions: &[&str],
    suggested: Option<&Path>,
) -> Option<PathBuf> {
    HOST.get()?.save_file(filter_name, extensions, suggested)
}

pub fn pick_file(title: &str, filter_name: &str, extensions: &[&str]) -> Option<PathBuf> {
    HOST.get()?.pick_file(title, filter_name, extensions)
}

pub fn show_trade(payload: Value) {
    if let Some(host) = HOST.get() {
        host.show_trade(payload);
    }
}

pub fn exit(code: i32) {
    match HOST.get() {
        Some(host) => host.exit(code),
        None => std::process::exit(code),
    }
}

fn runtime() -> Handle {
    RUNTIME
        .get()
        .cloned()
        .or_else(|| Handle::try_current().ok())
        .expect("No tokio runtime installed")
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime().spawn(future)
}

pub fn spawn_blocking<F, R>(func: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    runtime().spawn_blocking(func)
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    runtime().block_on(future)
}

/// A value stored in one process-wide slot, filled once at startup.
pub trait Managed: Sized + Send + Sync + 'static {
    fn slot() -> &'static OnceLock<Self>;
}

macro_rules! managed {
    ($($ty:ty),* $(,)?) => {$(
        impl Managed for $ty {
            fn slot() -> &'static OnceLock<Self> {
                static SLOT: OnceLock<$ty> = OnceLock::new();
                &SLOT
            }
        }
    )*};
}

managed!(
    Mutex<AppState>,
    Mutex<CacheState>,
    Arc<LiveScraperState>,
    Mutex<Arc<LogParserState>>,
    Mutex<Arc<WFInventoryState>>,
);

/// Stores `value` unless the slot is already filled, and returns the stored value.
pub fn manage<T: Managed>(value: T) -> &'static T {
    T::slot().get_or_init(|| value)
}

pub fn managed<T: Managed>() -> Result<&'static T, Error> {
    T::slot().get().ok_or_else(|| {
        Error::new(
            "Host:State",
            format!("{} is not initialized", std::any::type_name::<T>()),
            get_location!(),
        )
    })
}

/// Command argument for managed state. The Tauri build uses `tauri::State`.
#[cfg(feature = "tauri")]
pub type State<'a, T> = tauri::State<'a, &'static T>;

/// Command argument for managed state. Build one with `State::get()`.
#[cfg(not(feature = "tauri"))]
pub struct State<'a, T: 'static>(&'a T);

#[cfg(not(feature = "tauri"))]
impl<T: Managed> State<'static, T> {
    /// Panics if `init_headless` hasn't filled the slot.
    pub fn get() -> Self {
        State(managed::<T>().expect("State is not initialized"))
    }
}

#[cfg(not(feature = "tauri"))]
impl<'a, T> State<'a, T> {
    pub fn inner(&self) -> &'a T {
        self.0
    }
}

#[cfg(not(feature = "tauri"))]
impl<'a, T> From<&'a T> for State<'a, T> {
    fn from(value: &'a T) -> Self {
        State(value)
    }
}

#[cfg(not(feature = "tauri"))]
impl<T> Clone for State<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(not(feature = "tauri"))]
impl<T> Copy for State<'_, T> {}

#[cfg(not(feature = "tauri"))]
impl<T> std::ops::Deref for State<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}
