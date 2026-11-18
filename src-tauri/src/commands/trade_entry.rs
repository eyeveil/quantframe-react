
use entity::{dto::*, trade_entry::*};
use qf_api::enums::app_events::ApplicationEvent as EventType;
use service::{TradeEntryMutation, TradeEntryQuery};
use utils::{get_location, info, Error, LoggerOptions};

use crate::{
    send_event_update, track_event, types::UIOperationEvent, utils::CreateTradeEntryExt, DATABASE,
};

fn trade_entries_changed(operation: UIOperationEvent, ids: Vec<i64>) {
    send_event_update!(UIEvent::RefreshTradeEntries, operation, serde_json::json!(ids));
}

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn get_trade_entry_pagination(
    query: TradeEntryPaginationQueryDto,
) -> Result<PaginatedResult<Model>, Error> {
    let conn = DATABASE.get().unwrap();
    match TradeEntryQuery::get_all(conn, query).await {
        Ok(data) => return Ok(data),
        Err(e) => return Err(e.with_location(get_location!())),
    };
}

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn trade_entry_create(mut input: CreateTradeEntry) -> Result<Model, Error> {
    let conn = DATABASE.get().unwrap();
    input.validate().map_err(|e| {
        let err = e.clone();
        track_event!(
            EventType::TradeEntryCreate,
            [
                ("success", "false".to_string()),
                ("error_type", "validation_error".to_string()),
            ]
        );
        err.with_location(get_location!())
            .log("trade_entry_create.log");
        e
    })?;

    let model = input.to_model();
    match TradeEntryMutation::create_or_update(conn, input.override_existing, &model).await {
        Ok(item) => {
            track_event!(
                EventType::TradeEntryCreate,
                [("success", "true".to_string())]
            );
            trade_entries_changed(UIOperationEvent::CreateOrUpdate, vec![item.id]);
            Ok(item)
        }
        Err(e) => {
            track_event!(
                EventType::TradeEntryCreate,
                [
                    ("success", "false".to_string()),
                    ("error_type", "create_failed".to_string()),
                ]
            );
            return Err(e.with_location(get_location!()));
        }
    }
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn trade_entry_create_multiple(mut inputs: Vec<CreateTradeEntry>) -> Result<i64, Error> {
    let conn = DATABASE.get().unwrap();
    let mut ids = Vec::new();
    for input in inputs.iter_mut() {
        input.validate().map_err(|e| {
            let err = e.clone();
            track_event!(
                EventType::TradeEntryCreate,
                [
                    ("success", "false".to_string()),
                    ("error_type", "validation_error".to_string()),
                ]
            );
            err.with_location(get_location!())
                .log("trade_entry_create_multiple.log");
            e
        })?;
        let model = input.to_model();
        match TradeEntryMutation::create_or_update(conn, input.override_existing, &model).await {
            Ok(item) => ids.push(item.id),
            Err(e) => {
                track_event!(
                    EventType::TradeEntryCreate,
                    [
                        ("success", "false".to_string()),
                        ("error_type", "create_failed".to_string()),
                    ]
                );
                return Err(e.with_location(get_location!()));
            }
        }
    }
    track_event!(
        EventType::TradeEntryCreate,
        [
            ("success", "true".to_string()),
            ("count", ids.len().to_string()),
        ]
    );
    let total = ids.len() as i64;
    trade_entries_changed(UIOperationEvent::CreateOrUpdate, ids);
    Ok(total)
}

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn trade_entry_delete(id: i64) -> Result<Model, Error> {
    let conn = DATABASE.get().unwrap();

    let item = TradeEntryQuery::get_by_id(conn, id)
        .await
        .map_err(|e| {
            track_event!(
                EventType::TradeEntryDelete,
                [
                    ("success", "false".to_string()),
                    ("error_type", "query_failed".to_string()),
                ]
            );
            e.with_location(get_location!())
        })?;
    if item.is_none() {
        let err = Error::new(
            "Command::TradeEntryDelete",
            format!("Trade entry with ID {} not found", id),
            get_location!(),
        );
        track_event!(
            EventType::TradeEntryDelete,
            [
                ("success", "false".to_string()),
                ("error_type", "entry_not_found".to_string()),
            ]
        );
        return Err(err);
    }
    let item = item.unwrap();

    match TradeEntryMutation::delete_by_id(conn, id).await {
        Ok(_) => {}
        Err(e) => {
            track_event!(
                EventType::TradeEntryDelete,
                [
                    ("success", "false".to_string()),
                    ("error_type", "delete_failed".to_string()),
                ]
            );
            return Err(e.with_location(get_location!()));
        }
    }

    track_event!(
        EventType::TradeEntryDelete,
        [("success", "true".to_string())]
    );
    trade_entries_changed(UIOperationEvent::Delete, vec![id]);
    Ok(item)
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn trade_entry_delete_multiple(ids: Vec<i64>) -> Result<i64, Error> {
    let conn = DATABASE.get().unwrap();
    let mut deleted = Vec::new();

    for id in ids {
        match TradeEntryMutation::delete_by_id(conn, id).await {
            Ok(_) => deleted.push(id),
            Err(e) => {
                track_event!(
                    EventType::TradeEntryDelete,
                    [
                        ("success", "false".to_string()),
                        ("error_type", "delete_failed".to_string()),
                    ]
                );
                return Err(e.with_location(get_location!()));
            }
        }
    }
    track_event!(
        EventType::TradeEntryDelete,
        [
            ("success", "true".to_string()),
            ("count", deleted.len().to_string()),
        ]
    );
    let deleted_count = deleted.len() as i64;
    trade_entries_changed(UIOperationEvent::Delete, deleted);
    Ok(deleted_count)
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn trade_entry_update(input: UpdateTradeEntry) -> Result<Model, Error> {
    let conn = DATABASE.get().unwrap();

    match TradeEntryMutation::update_by_id(conn, input).await {
        Ok(item) => {
            track_event!(
                EventType::TradeEntryUpdate,
                [("success", "true".to_string())]
            );
            trade_entries_changed(UIOperationEvent::CreateOrUpdate, vec![item.id]);
            Ok(item)
        }
        Err(e) => {
            track_event!(
                EventType::TradeEntryUpdate,
                [
                    ("success", "false".to_string()),
                    ("error_type", "update_failed".to_string()),
                ]
            );
            return Err(e.with_location(get_location!()));
        }
    }
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn trade_entry_get_by_id(id: i64) -> Result<Model, Error> {
    let conn = DATABASE.get().unwrap();

    match TradeEntryQuery::get_by_id(conn, id).await {
        Ok(item) => {
            if let Some(trade_entry) = item {
                Ok(trade_entry)
            } else {
                Err(Error::new(
                    "Command::TradeEntryGetById",
                    format!("Trade entry with ID {} not found", id),
                    get_location!(),
                ))
            }
        }
        Err(e) => return Err(e.with_location(get_location!())),
    }
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn trade_entry_update_multiple(
    ids: Vec<i64>,
    input: UpdateTradeEntry,
) -> Result<Vec<Model>, Error> {
    let conn = DATABASE.get().unwrap();
    let mut updated_items = Vec::new();

    for id in ids {
        let mut update_input = input.clone();
        update_input.id = id;
        match TradeEntryMutation::update_by_id(conn, update_input).await {
            Ok(trade_entry) => updated_items.push(trade_entry),
            Err(e) => {
                track_event!(
                    EventType::TradeEntryUpdate,
                    [
                        ("success", "false".to_string()),
                        ("error_type", "update_failed".to_string()),
                    ]
                );
                return Err(e.with_location(get_location!()));
            }
        }
    }
    track_event!(
        EventType::TradeEntryUpdate,
        [
            ("success", "true".to_string()),
            ("count", updated_items.len().to_string()),
        ]
    );
    trade_entries_changed(
        UIOperationEvent::CreateOrUpdate,
        updated_items.iter().map(|item| item.id).collect(),
    );
    Ok(updated_items)
}

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn export_trade_entry_json(
    mut query: TradeEntryPaginationQueryDto,
) -> Result<String, Error> {
    let conn = DATABASE.get().unwrap();
    query.pagination.limit = -1; // fetch all
    match TradeEntryQuery::get_all(conn, query).await {
        Ok(trade_entry) => {
            let file_path = crate::host::save_file("quantframe_trade_entry", &["json"], None);
            if let Some(file_path) = file_path {
                let json = serde_json::to_string_pretty(&trade_entry.results).map_err(|e| {
                    let err = Error::new(
                        "Command::ExportTradeEntryJson",
                        format!("Failed to serialize trade entry to JSON: {}", e),
                        get_location!(),
                    );
                    track_event!(
                        EventType::TradeEntryExport,
                        [
                            ("success", "false".to_string()),
                            ("error_type", "serialization_error".to_string()),
                        ]
                    );
                    err
                })?;
                std::fs::write(&file_path, json).map_err(|e| {
                    let err = Error::new(
                        "Command::ExportTradeEntryJson",
                        format!("Failed to write trade entry to file: {}", e),
                        get_location!(),
                    );
                    track_event!(
                        EventType::TradeEntryExport,
                        [
                            ("success", "false".to_string()),
                            ("error_type", "file_write_error".to_string()),
                        ]
                    );
                    err
                })?;
                info(
                    "Command::ExportTradeEntryJson",
                    format!("Exported trade entry to JSON file: {}", file_path.display()),
                    &LoggerOptions::default(),
                );
                track_event!(
                    EventType::TradeEntryExport,
                    [
                        ("success", "true".to_string()),
                        ("count", trade_entry.results.len().to_string()),
                    ]
                );
                return Ok(file_path.display().to_string());
            }
            // do something with the optional file path here
            // the file path is `None` if the user closed the dialog
            track_event!(
                EventType::TradeEntryExport,
                [
                    ("success", "false".to_string()),
                    ("error_type", "cancelled".to_string()),
                ]
            );
            return Ok("".to_string());
        }
        Err(e) => {
            track_event!(
                EventType::TradeEntryExport,
                [
                    ("success", "false".to_string()),
                    ("error_type", "query_failed".to_string()),
                ]
            );
            return Err(e.with_location(get_location!()));
        }
    }
}
