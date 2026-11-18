// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![allow(non_snake_case)]
#![allow(deprecated)]
use ::utils::clear_logs;
use ::utils::get_location;
use ::utils::info;
use ::utils::init_logger;
use ::utils::set_base_path;
use ::utils::warning;
pub use ::utils::Error;
use ::utils::LoggerOptions;
use app::AppState;
use migration::{Migrator, MigratorTrait};
use service::sea_orm::{Database, DatabaseConnection};

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use crate::cache::client::CacheState;
use crate::live_scraper::LiveScraperState;
use crate::log_parser::init_detections;
use crate::log_parser::LogParserState;
use crate::wf_inventory::WFInventoryState;

pub mod app;
pub mod cache;
pub mod commands;
pub mod handlers;
pub mod http_server;
mod macros;
pub mod utils;
pub mod wf_inventory;

pub mod enums;
mod fixtures;
pub mod helper;
pub mod host;
pub mod live_scraper;
pub mod log_parser;
pub mod types;

pub use ::entity;
pub use ::qf_api;
pub use ::service;
pub use ::utils as qf_utils;
pub use ::wf_market;
pub use host::{Host, State};

pub(crate) static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
pub static DATABASE: OnceLock<DatabaseConnection> = OnceLock::new();
pub static HAS_STARTED: OnceLock<bool> = OnceLock::new();
pub static APP_ERROR: OnceLock<Mutex<Option<Error>>> = OnceLock::new();
pub static SENSITIVE_FIELDS: &[&str] = &[
    "email",
    "password",
    "authorization",
    "check_code",
    "qf_token",
    "wfm_token",
    "webhook",
    "slug",
];

// If use_debug is true the debug database will be used and all data will be lost on restart
async fn init_database(use_debug: bool) -> Result<(), Error> {
    // Create the database connection and store it
    let storage_path = helper::get_app_storage_path();

    let mut file_name = "quantframeV2.sqlite";
    let debug_file_name = "quantframeV2_debug.sqlite";

    // Create the path to the database file
    let file_path = format!("{}/{}", storage_path.to_str().unwrap(), file_name);

    // Create the path to the database file
    let file_path_backup = format!("{}/{}_backup", storage_path.to_str().unwrap(), file_name);
    info(
        "Setup:Database",
        "Creating a backup of the database file",
        &LoggerOptions::default(),
    );
    if std::path::Path::new(&file_path).exists() {
        std::fs::copy(&file_path, &file_path_backup)
            .expect("Failed to create a backup of the database file");
    }

    if use_debug {
        let db_debug_file_path_backup = file_path.replace(file_name, debug_file_name);
        file_name = debug_file_name;
        warning(
            "Setup:Database",
            "Debug mode is enabled, using the debug database file no data wil be saved",
            &LoggerOptions::default(),
        );
        if std::path::Path::new(&file_path).exists() {
            std::fs::copy(&file_path, &db_debug_file_path_backup)
                .expect("Failed to create a backup of the database file");
        }
    }

    // Create the database connection URL
    let db_url = format!(
        "sqlite://{}/{}?mode=rwc",
        storage_path.to_str().unwrap(),
        file_name,
    );

    // Create the database connection and store it and run the migrations
    let conn = Database::connect(db_url)
        .await
        .expect("Database connection failed");
    match Migrator::up(&conn, None).await {
        Ok(_) => info(
            "Setup:Database",
            "Database migrations applied successfully",
            &LoggerOptions::default(),
        ),
        Err(e) => {
            return Err(Error::new(
                "InitDatabase",
                format!("Failed to apply database migrations: {}", e),
                get_location!(),
            ));
        }
    }
    DATABASE.get_or_init(|| conn);
    Ok(())
}

async fn setup_manages(use_temp_db: bool, offline: bool) -> Result<(), Error> {
    init_detections();
    // Clear the logs older then 7 days
    clear_logs(7)?;

    // Clone the fields needed for CacheState before moving app_state
    let app_state = AppState::new(use_temp_db, false, offline).await?;
    let qf_client = app_state.qf_client.clone();
    let settings = app_state.settings.clone();
    let user = app_state.user.clone();
    host::manage(Mutex::new(app_state));

    let cache_state = if offline {
        CacheState::new_offline(&settings.lang)?
    } else {
        CacheState::new(&qf_client, &user, &settings.lang).await?
    };
    host::manage(Mutex::new(cache_state));

    host::manage(LiveScraperState::new());
    host::manage(Mutex::new(LogParserState::new(&settings.log_settings)));
    host::manage(Mutex::new(WFInventoryState::new(&settings)));
    Ok(())
}

#[cfg(feature = "tauri")]
fn manage_in_tauri(app: &tauri::AppHandle) {
    use tauri::Manager;
    fn manage<T: host::Managed>(app: &tauri::AppHandle) {
        if let Ok(state) = host::managed::<T>() {
            app.manage(state);
        }
    }
    manage::<Mutex<AppState>>(app);
    manage::<Mutex<CacheState>>(app);
    manage::<Arc<LiveScraperState>>(app);
    manage::<Mutex<Arc<LogParserState>>>(app);
    manage::<Mutex<Arc<WFInventoryState>>>(app);
}

/// Options for `init_headless`.
#[derive(Clone, Debug, Default)]
pub struct HeadlessOptions {
    /// A fixture directory such as `src-tauri/fixtures`. Seeds the cache, the user,
    /// the database, and the warframe.market caches from it, and makes no request
    /// to warframe.market or the Quantframe API.
    pub fixtures: Option<PathBuf>,
    /// Replaces the OS data directory. Fixture mode defaults to a fresh temp directory.
    pub storage_dir: Option<PathBuf>,
    pub use_temp_db: bool,
}

impl HeadlessOptions {
    /// Reads `QF_FIXTURES`, `QF_STORAGE_DIR`, and `QF_USE_TEMP_DB=1`.
    pub fn from_env() -> Self {
        HeadlessOptions {
            fixtures: std::env::var_os("QF_FIXTURES").map(PathBuf::from),
            storage_dir: std::env::var_os("QF_STORAGE_DIR").map(PathBuf::from),
            use_temp_db: std::env::var("QF_USE_TEMP_DB").is_ok_and(|v| v == "1"),
        }
    }
}

/// Starts the backend without Tauri. Call once, inside a tokio runtime, before any command.
pub async fn init_headless(opts: HeadlessOptions, host: Arc<dyn Host>) -> Result<(), Error> {
    host::install(host, tokio::runtime::Handle::current());
    host::set_offline(opts.fixtures.is_some());
    let storage_dir = match (&opts.storage_dir, &opts.fixtures) {
        (Some(dir), _) => Some(dir.clone()),
        (None, Some(_)) => Some(fixtures::temp_storage_dir()?),
        (None, None) => None,
    };
    if let Some(dir) = storage_dir {
        helper::STORAGE_PATH.get_or_init(|| dir);
    }
    init_logger();
    set_base_path(helper::get_app_storage_path().to_str().unwrap());
    if let Some(dir) = &opts.fixtures {
        fixtures::prepare_storage(dir)?;
    }
    let offline = opts.fixtures.is_some();
    init_database(opts.use_temp_db).await?;
    setup_manages(opts.use_temp_db, offline).await?;
    if let Some(dir) = &opts.fixtures {
        fixtures::seed(dir).await?;
    }
    HAS_STARTED.get_or_init(|| true);
    host::emit("app:ready", serde_json::Value::Null)?;
    Ok(())
}

#[cfg(feature = "tauri")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use ::utils::{critical, error};
    use std::panic;

    let use_temp_db: bool = std::env::args().any(|a| a == "--use-temp-db");

    println!(
        "Starting Quantframe...{:?}",
        std::env::args().collect::<Vec<String>>()
    );

    // Initialize the logger for elapsed time tracking
    panic::set_hook(Box::new(|panic_info| {
        eprintln!("Panic: {:?}", panic_info);
        critical(
            "Panic",
            format!("Panic: {:?}", panic_info).as_str(),
            &LoggerOptions::default().set_file("panic.log"),
        );
        emit_error!(Error::new(
            "Panic",
            format!("Panic: {:?}", panic_info),
            get_location!(),
        ));
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            use tauri::{Emitter, Manager};
            host::install(
                Arc::new(host::TauriHost(app.handle().clone())),
                tauri::async_runtime::handle().inner().clone(),
            );
            // Set the window title with the current package version so it doesn't need manual updating
            if let Some(window) = app.get_webview_window("main") {
                let version = app.package_info().version.to_string();
                let name = app.package_info().name.as_str();
                let _ = window.set_title(&format!("{name} v{version}"));
            }
            // Clone the handle for async task
            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let mut err = None;
                if let Err(e) = init_database(use_temp_db).await {
                    err = Some(e.clone());
                    e.log("init_database_error.log");
                    emit_error!(e);
                }
                let setup = setup_manages(use_temp_db, false).await;
                manage_in_tauri(&app_handle);
                if let Err(e) = setup {
                    err = Some(e.clone());
                    e.log("setup_error.log");
                    emit_error!(e);
                }
                if let Err(e) = app_handle.emit("app:ready", ()) {
                    error(
                        "Emit",
                        &format!("Failed to emit app:ready event: {:?}", e),
                        &LoggerOptions::default().set_file("emit_error.log"),
                    );
                }
                HAS_STARTED.set(true).unwrap();
                if let Some(e) = err {
                    emit_error!(e);
                }
            });
            init_logger();
            set_base_path(helper::get_app_storage_path().to_str().unwrap());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Base commands
            commands::dashboard::dashboard_summary,
            commands::app::initialized,
            commands::app::app_get_app_info,
            commands::app::app_get_settings,
            commands::app::app_update_settings,
            commands::app::app_exit,
            commands::app::app_accept_tos,
            commands::app::app_notify_reset,
            commands::app::app_get_default_settings,
            // Auth commands
            commands::auth::auth_me,
            commands::auth::auth_login,
            commands::auth::auth_logout,
            // User commands
            commands::user::user_set_status,
            // Analytics commands
            commands::analytics::track_event,
            // Alert commands
            commands::alert::alert_get_alerts,
            // Cache commands
            commands::cache::cache_get_tradable_items,
            commands::cache::cache_get_riven_attributes,
            commands::cache::cache_get_riven_weapons,
            commands::cache::cache_get_theme_presets,
            commands::cache::cache_create_theme,
            commands::cache::cache_open_theme_folder,
            commands::cache::cache_get_chat_link,
            commands::cache::cache_get_chat_icons,
            commands::cache::cache_get_syndicates,
            // Log commands
            commands::logs::log_export,
            commands::logs::log,
            // Live Scraper commands
            commands::live_scraper::live_scraper_get_state,
            commands::live_scraper::live_scraper_toggle,
            commands::live_scraper::live_scraper_get_interesting_wtb_items,
            // Stock Item commands
            commands::stock_item::get_stock_item_pagination,
            commands::stock_item::get_stock_item_financial_report,
            commands::stock_item::get_stock_item_status_counts,
            commands::stock_item::stock_item_create,
            commands::stock_item::stock_item_delete,
            commands::stock_item::stock_item_sell,
            commands::stock_item::stock_item_update,
            commands::stock_item::stock_item_get_by_id,
            commands::stock_item::stock_item_update_multiple,
            commands::stock_item::stock_item_delete_multiple,
            commands::stock_item::export_stock_item_json,
            // Stock Riven commands
            commands::stock_riven::get_stock_riven_pagination,
            commands::stock_riven::get_stock_riven_financial_report,
            commands::stock_riven::get_stock_riven_status_counts,
            commands::stock_riven::stock_riven_create,
            commands::stock_riven::stock_riven_delete,
            commands::stock_riven::stock_riven_sell,
            commands::stock_riven::stock_riven_update,
            commands::stock_riven::stock_riven_get_by_id,
            commands::stock_riven::export_stock_riven_json,
            commands::stock_riven::stock_riven_update_multiple,
            commands::stock_riven::stock_riven_delete_multiple,
            // Debug commands
            commands::debug::debug_get_wfm_state,
            commands::debug::debug_get_ee_logs,
            commands::debug::debug_export_ee_logs,
            commands::debug::debug_test,
            // Order commands
            commands::order::get_wfm_orders_pagination,
            commands::order::get_wfm_orders_status_counts,
            commands::order::order_refresh,
            commands::order::order_delete_all,
            commands::order::order_delete_by_id,
            commands::order::get_wfm_order_by_id,
            // Auction commands
            commands::auction::auction_refresh,
            commands::auction::get_wfm_auctions_pagination,
            commands::auction::get_wfm_auctions_overview,
            commands::auction::auction_delete_all,
            commands::auction::auction_delete_by_id,
            commands::auction::get_wfm_auction_by_id,
            commands::auction::auction_import_by_id,
            // Wish List commands
            commands::wish_list::get_wish_list_pagination,
            commands::wish_list::get_wish_list_financial_report,
            commands::wish_list::get_wish_list_status_counts,
            commands::wish_list::wish_list_create,
            commands::wish_list::wish_list_bought,
            commands::wish_list::wish_list_delete,
            commands::wish_list::wish_list_update,
            commands::wish_list::wish_list_get_by_id,
            commands::wish_list::export_wish_list_json,
            commands::wish_list::wish_list_update_multiple,
            commands::wish_list::wish_list_delete_multiple,
            // Chat commands
            commands::chat::chat_refresh,
            commands::chat::get_chat_pagination,
            commands::chat::chat_get_messages_by_id,
            commands::chat::chat_delete,
            commands::chat::chat_set_active,
            commands::chat::chat_send_message,
            // Transaction commands
            commands::transaction::get_transaction_pagination,
            commands::transaction::get_transaction_financial_report,
            commands::transaction::transaction_update,
            commands::transaction::transaction_delete,
            commands::transaction::transaction_delete_bulk,
            commands::transaction::export_transaction_json,
            commands::transaction::transaction_calculate_tax,
            // Item commands
            commands::item::item_prices_lookup,
            commands::item::item_price_lookup,
            commands::item::export_item_price_data,
            // Syndicate Item commands
            commands::syndicate_item::syndicate_item_import_items,
            commands::syndicate_item::get_syndicate_item_pagination,
            commands::syndicate_item::get_syndicate_item_financial_report,
            commands::syndicate_item::get_syndicate_item_status_counts,
            commands::syndicate_item::get_syndicate_item_syndicate_counts,
            commands::syndicate_item::syndicate_item_create,
            commands::syndicate_item::syndicate_item_delete,
            commands::syndicate_item::syndicate_item_sell,
            commands::syndicate_item::syndicate_item_update,
            commands::syndicate_item::syndicate_item_get_by_id,
            commands::syndicate_item::syndicate_item_update_multiple,
            commands::syndicate_item::syndicate_item_delete_multiple,
            commands::syndicate_item::export_syndicate_item_json,
            // Syndicate Item Price commands
            commands::syndicate_price::syndicate_item_prices_lookup,
            commands::syndicate_price::export_syndicate_item_price_data,
            // Riven commands
            commands::riven::riven_prices_lookup,
            commands::riven::export_riven_price_data,
            // Riven price prediction commands
            commands::riven_pricer::riven_pricer_available,
            commands::riven_pricer::riven_price_predict,
            commands::riven_pricer::riven_pricer_reload,
            commands::riven_pricer::get_known_riven_weapons,
            // Market commands
            commands::market::get_user_activity,
            // Trade Entry commands
            commands::trade_entry::get_trade_entry_pagination,
            commands::trade_entry::trade_entry_get_by_id,
            commands::trade_entry::trade_entry_create,
            commands::trade_entry::trade_entry_create_multiple,
            commands::trade_entry::trade_entry_delete,
            commands::trade_entry::trade_entry_delete_multiple,
            commands::trade_entry::trade_entry_update,
            commands::trade_entry::trade_entry_update_multiple,
            commands::trade_entry::export_trade_entry_json,
            // Warframe GDPR commands
            commands::warframe_gdpr::wfgdpr_get_state,
            commands::warframe_gdpr::wfgdpr_load,
            commands::warframe_gdpr::wfgdpr_get_accounts,
            // Sound commands
            commands::sound::sound_get_custom_sounds,
            commands::sound::sound_add_custom_sound,
            commands::sound::sound_delete_custom_sound,
            commands::sound::sound_get_custom_sounds_path,
            // Handlers
            commands::handlers::handles_handle_items,
            // WFInventory commands
            commands::wf_inventory::wf_inventory_get_rivens,
            commands::wf_inventory::wf_inventory_get_syndicates,
            commands::wf_inventory::wf_inventory_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                track_app_exit();
            }
        });
}

#[cfg(feature = "tauri")]
/// Sends the `app_exit` analytics event and flushes the queue before the
/// process terminates. Safe to call more than once.
fn track_app_exit() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static TRACKED: AtomicBool = AtomicBool::new(false);
    if TRACKED.swap(true, Ordering::SeqCst) {
        return;
    }

    let Ok(app) = crate::utils::modules::states::app_state() else {
        return;
    };

    app.analytics.track_event(
        qf_api::enums::ApplicationEvent::AppExit,
        [(
            "session_duration",
            ::utils::get_duration_since_start().as_secs().to_string(),
        )],
    );

    // Report how many Warframe Market API calls were made per route this
    // session (one event per route, so it's easy to aggregate in ClickHouse).
    let routes: Vec<(String, String)> = match app.wfm_client.get_tracking().lock() {
        Ok(tracking) => tracking
            .iter()
            .map(|(route, count)| (route.clone(), count.to_string()))
            .collect(),
        Err(_) => Vec::new(),
    };
    for (route, count) in routes {
        app.analytics.track_event(
            qf_api::enums::ApplicationEvent::WfmApiTracking,
            [("route", route), ("count", count)],
        );
    }

    app.analytics.flush();
}
