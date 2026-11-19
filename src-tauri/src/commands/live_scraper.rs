use crate::State;
use std::sync::{atomic::Ordering, Arc, Mutex};

use serde_json::{json, Value};
use utils::Error;

use crate::{
    app::ItemSettings,
    cache::{CacheState, ItemPriceInfo},
    live_scraper::{
        self,
        writes::{self, IntendedOrder},
        LiveScraperState,
    },
    send_event,
    types::*,
};

#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn live_scraper_toggle(
    live_scraper: State<'_, Arc<LiveScraperState>>,
) -> Result<(), Error> {
    if live_scraper.is_running.load(Ordering::SeqCst) {
        live_scraper.stop();
    } else {
        live_scraper.start();
    }
    send_event!(
        UIEvent::UpdateLiveScraperRunningState,
        json!(live_scraper.is_running.load(Ordering::SeqCst))
    );
    Ok(())
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn live_scraper_get_state(
    live_scraper: State<'_, Arc<LiveScraperState>>,
) -> Result<Value, Error> {
    Ok(json!({
        "is_running": live_scraper.is_running.load(Ordering::SeqCst)
    }))
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn live_scraper_get_intended_orders() -> Result<Vec<IntendedOrder>, Error> {
    Ok(writes::intended_orders())
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn live_scraper_clear_intended_orders() -> Result<(), Error> {
    writes::clear_intended_orders();
    Ok(())
}
#[cfg_attr(feature = "tauri", tauri::command)]
pub async fn live_scraper_get_interesting_wtb_items(
    settings: ItemSettings,
    cache: State<'_, Mutex<CacheState>>,
) -> Result<Vec<ItemPriceInfo>, Error> {
    let mut items = live_scraper::helpers::get_interesting_items(&settings);
    let cache = cache.lock()?;
    for item in &mut items {
        if let Ok(info) = cache.tradable_item().get_by(&item.wfm_id) {
            item.properties.set_property_value("name", info.name);
        }
    }
    Ok(items)
}
