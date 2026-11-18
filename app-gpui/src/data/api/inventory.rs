//! The in-game rivens with every field the Inventory screen shows, and the
//! call that copies one into stock.

use std::future::Future;

use qf_app_lib::{
    State,
    commands::{stock_riven, wf_inventory},
};
use serde_json::{Value, json};

use super::dto;
use crate::{
    data::{DataError, DataResult, DataSource, convert},
    model::{InventoryRiven, StockRiven},
};

/// Every riven of `wf_inventory_get_rivens`, as the backend serialises it.
async fn raw_rivens() -> DataResult<Vec<Value>> {
    let query = dto(json!({ "page": 1, "limit": -1 }))?;
    let value = wf_inventory::wf_inventory_get_rivens(query, State::get()).await?;
    Ok(value["results"].as_array().cloned().unwrap_or_default())
}

/// `WFInvItemRiven` flattens `ItemRivenBase` and its properties into the
/// riven's own JSON object, so every field sits at the top level.
fn riven(value: &Value) -> InventoryRiven {
    let text = |key: &str| value[key].as_str().unwrap_or_default().to_owned();
    InventoryRiven {
        uuid: text("uuid"),
        name: text("name"),
        wfm_url: text("wfm_url"),
        mod_name: text("mod_name"),
        mastery_rank: value["mastery_rank"].as_i64().unwrap_or(0),
        re_rolls: value["re_rolls"].as_i64().unwrap_or(0),
        rank: value["sub_type"]["rank"].as_i64().unwrap_or(0),
        polarity: text("polarity"),
        riven_type: text("riven_type"),
        is_in_stock: value["is_in_stock"].as_bool().unwrap_or(false),
    }
}

impl DataSource {
    /// Every riven in the in-game inventory.
    pub fn wf_inventory_list_rivens(
        &self,
    ) -> impl Future<Output = DataResult<Vec<InventoryRiven>>> + Send + 'static {
        self.call(async move { Ok(raw_rivens().await?.iter().map(riven).collect()) })
    }

    /// Adds the inventory riven with this uuid to the stock rivens, bought
    /// for `bought` platinum.
    pub fn wf_inventory_stock_riven(
        &self,
        uuid: String,
        bought: i64,
    ) -> impl Future<Output = DataResult<StockRiven>> + Send + 'static {
        self.call(async move {
            let rivens = raw_rivens().await?;
            let Some(riven) = rivens.iter().find(|riven| riven["uuid"] == uuid.as_str()) else {
                return Err(DataError::new(
                    "DataSource:Inventory",
                    "The riven is no longer in the inventory",
                ));
            };
            let input = dto(json!({
                "raw": riven["name"],
                "mod_name": riven["mod_name"],
                "mastery_rank": riven["mastery_rank"].as_i64().unwrap_or(0),
                "re_rolls": riven["re_rolls"].as_i64().unwrap_or(0),
                "rank": riven["sub_type"]["rank"].as_i64().unwrap_or(0),
                "polarity": riven["polarity"],
                "attributes": riven["attributes"],
                "bought": bought,
            }))?;
            Ok(convert::stock_riven(
                stock_riven::stock_riven_create(input).await?,
            ))
        })
    }
}
