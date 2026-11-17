use crate::State;
use qf_api::{enums::app_events::ApplicationEvent as EventType, types::*};
use std::sync::Mutex;
use utils::{get_location, info, Error, LoggerOptions};

use crate::{app::AppState, track_event, utils::ErrorFromExt};

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn riven_prices_lookup(
    query: RivenPricePaginationQueryDto,
    app: State<'_, Mutex<AppState>>,
) -> Result<Paginated<RivenPrice>, Error> {
    let app_state = app.lock().unwrap().clone();
    match app_state.qf_client.riven().get_prices(query).await {
        Ok(data) => {
            track_event!(
                EventType::RivenPriceLookup,
                [
                    ("success", "true".to_string()),
                    ("count", data.results.len().to_string()),
                ]
            );
            return Ok(data);
        }
        Err(e) => {
            let error_type = e.error_type().to_string();
            let error = Error::from_qf(
                "RivenPricesLookup",
                "Failed to lookup riven prices: {}",
                e,
                get_location!(),
            )
            .log("riven_prices_lookup.log");
            track_event!(
                EventType::RivenPriceLookup,
                [
                    ("success", "false".to_string()),
                    ("error_type", error_type),
                ]
            );
            return Err(error);
        }
    };
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn export_riven_price_data(
    mut query: RivenPricePaginationQueryDto,
    app: State<'_, Mutex<AppState>>,
) -> Result<String, Error> {
    let app_state = app.lock().unwrap().clone();
    query.pagination.limit = -1;
    let trace_event_error = |error_type: &str| {
        track_event!(
            EventType::RivenPriceExport,
            [
                ("success", "false".to_string()),
                ("error_type", error_type.to_string()),
            ]
        );
    };

    let data = match app_state.qf_client.riven().get_prices(query).await {
        Ok(data) => data,
        Err(e) => {
            let error_type = e.error_type().to_string();
            let error = Error::from_qf(
                "RivenPricesLookup",
                "Failed to lookup riven prices: {}",
                e,
                get_location!(),
            )
            .log("riven_prices_lookup.log");
            track_event!(
                EventType::RivenPriceExport,
                [
                    ("success", "false".to_string()),
                    ("error_type", error_type),
                ]
            );
            return Err(error);
        }
    };

    let Some(file_path) = crate::host::save_file("Quantframe_RivenPrices", &["json"], None)
    else {
        trace_event_error("cancelled");
        return Ok(String::new());
    };

    let json = serde_json::to_string_pretty(&data.results).map_err(|e| {
        trace_event_error("serialization_error");
        Error::new(
            "Command::ExportRivenPriceData",
            format!("Failed to serialize riven prices to JSON: {}", e),
            get_location!(),
        )
    })?;

    std::fs::write(&file_path, json).map_err(|e| {
        trace_event_error("file_write_error");
        Error::new(
            "Command::ExportRivenPriceData",
            format!("Failed to write riven prices to file: {}", e),
            get_location!(),
        )
    })?;

    info(
        "Command::ExportRivenPriceData",
        format!("Exported riven prices to JSON file: {}", file_path.display()),
        &LoggerOptions::default(),
    );

    track_event!(
        EventType::RivenPriceExport,
        [
            ("success", "true".to_string()),
            ("count", data.results.len().to_string()),
        ]
    );

    Ok(file_path.display().to_string())
}
