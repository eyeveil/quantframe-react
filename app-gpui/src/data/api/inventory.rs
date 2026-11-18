use std::future::Future;

use qf_app_lib::{
    State,
    commands::{stock_riven, wf_inventory},
    entity::dto::PaginatedResult,
    wf_inventory::types::WFInvItemRiven,
};
use serde_json::json;

use super::dto;
use crate::{
    data::{DataError, DataResult, DataSource, convert},
    model::{InventoryRiven, StockRiven},
};

async fn raw_rivens() -> DataResult<Vec<WFInvItemRiven>> {
    let query = dto(json!({ "page": 1, "limit": -1 }))?;
    let value = wf_inventory::wf_inventory_get_rivens(query, State::get()).await?;
    let page: PaginatedResult<WFInvItemRiven> = serde_json::from_value(value)?;
    Ok(page.results)
}

impl DataSource {
    pub fn wf_inventory_list_rivens(
        &self,
    ) -> impl Future<Output = DataResult<Vec<InventoryRiven>>> + Send + 'static {
        self.call(async move {
            Ok(raw_rivens()
                .await?
                .into_iter()
                .map(convert::inventory_riven)
                .collect())
        })
    }

    pub fn wf_inventory_stock_riven(
        &self,
        uuid: String,
        bought: i64,
    ) -> impl Future<Output = DataResult<StockRiven>> + Send + 'static {
        self.call(async move {
            let rivens = raw_rivens().await?;
            let Some(riven) = rivens.into_iter().find(|riven| riven.base.uuid == uuid) else {
                return Err(DataError::new(
                    "DataSource:Inventory",
                    "The riven is no longer in the inventory",
                ));
            };
            let base = riven.base;
            let input = dto(json!({
                "raw": base.name,
                "mod_name": base.mod_name,
                "mastery_rank": base.mastery_rank,
                "re_rolls": base.re_rolls,
                "rank": base.sub_type.and_then(|sub_type| sub_type.rank).unwrap_or(0),
                "polarity": base.polarity,
                "attributes": base.attributes,
                "bought": bought,
            }))?;
            Ok(convert::stock_riven(
                stock_riven::stock_riven_create(input).await?,
            ))
        })
    }
}
