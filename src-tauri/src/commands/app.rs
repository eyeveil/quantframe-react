use crate::State;
use std::sync::{Arc, Mutex};

use crate::{
    app::{AppState, Settings},
    host::{self, PACKAGE_AUTHORS, PACKAGE_DESCRIPTION, PACKAGE_NAME, PACKAGE_VERSION},
    log_parser::LogParserState,
    wf_inventory::{InventorySource, WFInventoryState},
    HAS_STARTED,
};
use serde_json::{json, Value};
use utils::Error;

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn initialized() -> Result<bool, Error> {
    let started = HAS_STARTED.get().cloned().unwrap_or(false);
    return Ok(started);
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn app_get_app_info(app: State<'_, Mutex<AppState>>) -> Result<Value, Error> {
    let app = app.lock()?;
    Ok(json!({
        "version": PACKAGE_VERSION,
        "name": PACKAGE_NAME,
        "description": PACKAGE_DESCRIPTION,
        "authors": PACKAGE_AUTHORS,
        "is_dev": app.is_development,
        "use_temp_db": app.use_temp_db,
        "tos_uuid": app.settings.tos_uuid.clone(),
        "is_pre_release": app.is_pre_release,
    }))
}

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn app_get_settings(app: State<'_, Mutex<AppState>>) -> Result<Settings, Error> {
    let app = app.lock()?;
    Ok(app.settings.clone())
}

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn app_get_default_settings() -> Result<Settings, Error> {
    Ok(Settings::default())
}

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn app_update_settings(
    mut settings: Settings,
    app: State<'_, Mutex<AppState>>,
    log_parser: State<'_, Mutex<Arc<LogParserState>>>,
    wf_inventory: State<'_, Mutex<Arc<WFInventoryState>>>,
) -> Result<Settings, Error> {
    let mut app = app.lock()?;
    settings.notifications.custom_sounds = app.settings.notifications.custom_sounds.clone();
    let log_parser = log_parser.lock()?;
    log_parser.set_path(&settings.log_settings.ee_log_path)?;

    let current_http_server = &app.settings.advanced_settings.http_server;
    let new_http_server = &settings.advanced_settings.http_server;

    if new_http_server.uuid() != current_http_server.uuid() {
        let operation = app
            .http_server
            .set_host(&new_http_server.host, new_http_server.port);
        match (new_http_server.enable, operation.as_str()) {
            (true, "NO_CHANGE") => app.http_server.start(),
            (false, "NO_CHANGE") => app.http_server.stop(),
            (true, "CHANGED") => app.http_server.restart(),
            (false, "CHANGED") => app.http_server.stop(),
            _ => {}
        }
    }

    settings.wf_inventory.source.validate()?;

    let current_wf_source = app.settings.wf_inventory.source.clone();
    if settings.wf_inventory.source != current_wf_source {
        let wf_inventory = wf_inventory.lock()?;
        wf_inventory.set_source(settings.wf_inventory.source.clone());
    }

    app.update_settings(settings.clone())?;
    Ok(settings.clone())
}

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn app_exit() -> Result<(), Error> {
    host::exit(0);
    Ok(())
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn app_accept_tos(id: String, app: State<'_, Mutex<AppState>>) -> Result<(), Error> {
    let mut app = app.lock()?;
    let mut settings = app.settings.clone();
    settings.tos_uuid = id.clone();
    app.update_settings(settings)?;
    Ok(())
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn app_notify_reset(id: String) -> Result<Value, Error> {
    let value = json!(crate::app::NotificationsSetting::default());
    if value[id.clone()].is_object() {
        return Ok(value[id.clone()].clone());
    }
    Ok(json!({}))
}
