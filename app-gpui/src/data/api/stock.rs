use std::{collections::BTreeMap, future::Future, path::PathBuf};

use qf_app_lib::{
    State,
    commands::{stock_item, stock_riven, syndicate_item, wish_list},
    entity::stock_riven::RivenAttribute as BackendAttribute,
};
use serde_json::{Value, json};

use super::{
    all, dto, limit, status_counts, stock_list, syndicate_list, to_backend, with_properties,
};
use crate::{
    data::{DataResult, DataSource, convert},
    model::*,
};

fn attributes(attributes: &[RivenAttribute]) -> Vec<BackendAttribute> {
    attributes
        .iter()
        .map(|attribute| {
            BackendAttribute::new(
                attribute.positive,
                attribute.value,
                attribute.wfm_url.clone(),
                String::new(),
            )
        })
        .collect()
}

fn sub_type_json(sub_type: &Option<SubType>) -> Value {
    json!(convert::backend_sub_type(sub_type.clone()))
}

impl DataSource {
    pub fn get_stock_item_pagination(
        &self,
        params: &StockItemListParams,
    ) -> impl Future<Output = DataResult<Paginated<StockItem>>> + Send + 'static {
        let query = stock_list(params);
        self.call(async move {
            let page = stock_item::get_stock_item_pagination(dto(query.into())?).await?;
            Ok(convert::paginated(page, convert::stock_item))
        })
    }

    pub fn get_stock_item_status_counts(
        &self,
    ) -> impl Future<Output = DataResult<BTreeMap<StockStatus, usize>>> + Send + 'static {
        self.call(async move {
            Ok(status_counts(
                stock_item::get_stock_item_status_counts(dto(all())?).await?,
            ))
        })
    }

    pub fn get_stock_item_financial_report(
        &self,
    ) -> impl Future<Output = DataResult<FinancialReport>> + Send + 'static {
        self.call(async move {
            Ok(convert::report(
                stock_item::get_stock_item_financial_report(dto(all())?).await?,
            ))
        })
    }

    pub fn stock_item_get_by_id(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<StockItem>> + Send + 'static {
        self.call(async move {
            let model =
                stock_item::stock_item_get_by_id(id, None, State::get(), State::get()).await?;
            Ok(convert::stock_item(model))
        })
    }

    pub fn stock_item_create(
        &self,
        input: NewStockItem,
    ) -> impl Future<Output = DataResult<StockItem>> + Send + 'static {
        self.call(async move {
            let input = dto(json!({
                "raw": input.raw,
                "quantity": input.quantity,
                "bought": input.bought,
                "sub_type": sub_type_json(&input.sub_type),
            }))?;
            Ok(convert::stock_item(
                stock_item::stock_item_create(input).await?,
            ))
        })
    }

    pub fn stock_item_update(
        &self,
        id: i64,
        update: StockItemUpdate,
    ) -> impl Future<Output = DataResult<StockItem>> + Send + 'static {
        self.call(async move {
            let input = dto(stock_item_changes(id, &update))?;
            Ok(convert::stock_item(
                stock_item::stock_item_update(input).await?,
            ))
        })
    }

    pub fn stock_item_update_multiple(
        &self,
        ids: Vec<i64>,
        update: StockItemUpdate,
    ) -> impl Future<Output = DataResult<Vec<StockItem>>> + Send + 'static {
        self.call(async move {
            let input = dto(stock_item_changes(0, &update))?;
            let models = stock_item::stock_item_update_multiple(ids, input).await?;
            Ok(models.into_iter().map(convert::stock_item).collect())
        })
    }

    pub fn stock_item_sell(
        &self,
        wfm_url: String,
        sub_type: Option<SubType>,
        quantity: i64,
        price: i64,
    ) -> impl Future<Output = DataResult<StockItem>> + Send + 'static {
        self.call(async move {
            let model = stock_item::stock_item_sell(
                wfm_url,
                convert::backend_sub_type(sub_type),
                quantity,
                price,
            )
            .await?;
            Ok(convert::stock_item(model))
        })
    }

    pub fn stock_item_delete(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<StockItem>> + Send + 'static {
        self.call(async move {
            Ok(convert::stock_item(
                stock_item::stock_item_delete(id).await?,
            ))
        })
    }

    pub fn stock_item_delete_multiple(
        &self,
        ids: Vec<i64>,
    ) -> impl Future<Output = DataResult<i64>> + Send + 'static {
        self.call(async move { Ok(stock_item::stock_item_delete_multiple(ids).await?) })
    }

    pub fn export_stock_item_json(
        &self,
        params: &StockItemListParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = stock_list(params);
        self.export(path, async move {
            stock_item::export_stock_item_json(dto(query.into()).map_err(to_backend)?).await
        })
    }

    pub fn get_stock_riven_pagination(
        &self,
        params: &StockItemListParams,
    ) -> impl Future<Output = DataResult<Paginated<StockRiven>>> + Send + 'static {
        let query = stock_list(params);
        self.call(async move {
            let page = stock_riven::get_stock_riven_pagination(dto(query.into())?).await?;
            Ok(convert::paginated(page, convert::stock_riven))
        })
    }

    pub fn get_stock_riven_status_counts(
        &self,
    ) -> impl Future<Output = DataResult<BTreeMap<StockStatus, usize>>> + Send + 'static {
        self.call(async move {
            Ok(status_counts(
                stock_riven::get_stock_riven_status_counts(dto(all())?).await?,
            ))
        })
    }

    pub fn get_stock_riven_financial_report(
        &self,
    ) -> impl Future<Output = DataResult<FinancialReport>> + Send + 'static {
        self.call(async move {
            Ok(convert::report(
                stock_riven::get_stock_riven_financial_report(dto(all())?).await?,
            ))
        })
    }

    pub fn stock_riven_get_by_id(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<StockRiven>> + Send + 'static {
        self.call(async move {
            let model =
                stock_riven::stock_riven_get_by_id(id, None, State::get(), State::get()).await?;
            Ok(convert::stock_riven(model))
        })
    }

    pub fn stock_riven_create(
        &self,
        input: NewStockRiven,
    ) -> impl Future<Output = DataResult<StockRiven>> + Send + 'static {
        self.call(async move {
            let input = dto(json!({
                "raw": input.wfm_weapon_url,
                "mod_name": input.mod_name,
                "mastery_rank": input.mastery_rank,
                "re_rolls": input.re_rolls,
                "rank": input.rank,
                "polarity": input.polarity,
                "attributes": attributes(&input.attributes),
                "bought": input.bought,
            }))?;
            Ok(convert::stock_riven(
                stock_riven::stock_riven_create(input).await?,
            ))
        })
    }

    pub fn stock_riven_update(
        &self,
        id: i64,
        update: StockRivenUpdate,
    ) -> impl Future<Output = DataResult<StockRiven>> + Send + 'static {
        self.call(async move {
            let input = dto(stock_riven_changes(id, &update))?;
            Ok(convert::stock_riven(
                stock_riven::stock_riven_update(input).await?,
            ))
        })
    }

    pub fn stock_riven_update_multiple(
        &self,
        ids: Vec<i64>,
        update: StockRivenUpdate,
    ) -> impl Future<Output = DataResult<Vec<StockRiven>>> + Send + 'static {
        self.call(async move {
            let input = dto(stock_riven_changes(0, &update))?;
            let models = stock_riven::stock_riven_update_multiple(ids, input).await?;
            Ok(models.into_iter().map(convert::stock_riven).collect())
        })
    }

    pub fn stock_riven_sell(
        &self,
        riven: &StockRiven,
        price: i64,
    ) -> impl Future<Output = DataResult<StockRiven>> + Send + 'static {
        let riven = riven.clone();
        self.call(async move {
            let model = stock_riven::stock_riven_sell(
                riven.wfm_weapon_url,
                riven.mod_name,
                riven.mastery_rank,
                riven.rank.unwrap_or(0),
                riven.re_rolls,
                riven.polarity,
                attributes(&riven.attributes),
                price,
            )
            .await?;
            Ok(convert::stock_riven(model))
        })
    }

    pub fn stock_riven_delete(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<StockRiven>> + Send + 'static {
        self.call(async move {
            Ok(convert::stock_riven(
                stock_riven::stock_riven_delete(id, State::get()).await?,
            ))
        })
    }

    pub fn stock_riven_delete_multiple(
        &self,
        ids: Vec<i64>,
    ) -> impl Future<Output = DataResult<i64>> + Send + 'static {
        self.call(async move { Ok(stock_riven::stock_riven_delete_multiple(ids).await?) })
    }

    pub fn export_stock_riven_json(
        &self,
        params: &StockItemListParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = stock_list(params);
        self.export(path, async move {
            stock_riven::export_stock_riven_json(dto(query.into()).map_err(to_backend)?).await
        })
    }

    pub fn get_wish_list_pagination(
        &self,
        params: &StockItemListParams,
    ) -> impl Future<Output = DataResult<Paginated<WishListItem>>> + Send + 'static {
        let query = stock_list(params);
        self.call(async move {
            let page = wish_list::get_wish_list_pagination(dto(query.into())?).await?;
            Ok(convert::paginated(page, convert::wish_list_item))
        })
    }

    pub fn get_wish_list_status_counts(
        &self,
    ) -> impl Future<Output = DataResult<BTreeMap<StockStatus, usize>>> + Send + 'static {
        self.call(async move {
            Ok(status_counts(
                wish_list::get_wish_list_status_counts(dto(all())?).await?,
            ))
        })
    }

    pub fn get_wish_list_financial_report(
        &self,
    ) -> impl Future<Output = DataResult<FinancialReport>> + Send + 'static {
        self.call(async move {
            Ok(convert::report(
                wish_list::get_wish_list_financial_report(dto(all())?).await?,
            ))
        })
    }

    pub fn wish_list_create(
        &self,
        input: NewStockItem,
    ) -> impl Future<Output = DataResult<WishListItem>> + Send + 'static {
        self.call(async move {
            let input = dto(json!({
                "raw": input.raw,
                "quantity": input.quantity,
                "bought": input.bought,
                "sub_type": sub_type_json(&input.sub_type),
            }))?;
            Ok(convert::wish_list_item(
                wish_list::wish_list_create(input).await?,
            ))
        })
    }

    pub fn wish_list_bought(
        &self,
        wfm_url: String,
        sub_type: Option<SubType>,
        quantity: i64,
        price: i64,
    ) -> impl Future<Output = DataResult<WishListItem>> + Send + 'static {
        self.call(async move {
            let model = wish_list::wish_list_bought(
                wfm_url,
                convert::backend_sub_type(sub_type),
                quantity,
                price,
            )
            .await?;
            Ok(convert::wish_list_item(model))
        })
    }

    pub fn wish_list_update(
        &self,
        id: i64,
        update: WishListUpdate,
    ) -> impl Future<Output = DataResult<WishListItem>> + Send + 'static {
        self.call(async move {
            let input = dto(wish_list_changes(id, &update))?;
            Ok(convert::wish_list_item(
                wish_list::wish_list_update(input).await?,
            ))
        })
    }

    pub fn wish_list_update_multiple(
        &self,
        ids: Vec<i64>,
        update: WishListUpdate,
    ) -> impl Future<Output = DataResult<Vec<WishListItem>>> + Send + 'static {
        self.call(async move {
            let input = dto(wish_list_changes(0, &update))?;
            let models = wish_list::wish_list_update_multiple(ids, input).await?;
            Ok(models.into_iter().map(convert::wish_list_item).collect())
        })
    }

    pub fn wish_list_delete(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<WishListItem>> + Send + 'static {
        self.call(async move {
            Ok(convert::wish_list_item(
                wish_list::wish_list_delete(id).await?,
            ))
        })
    }

    pub fn wish_list_delete_multiple(
        &self,
        ids: Vec<i64>,
    ) -> impl Future<Output = DataResult<i64>> + Send + 'static {
        self.call(async move { Ok(wish_list::wish_list_delete_multiple(ids).await?) })
    }

    pub fn export_wish_list_json(
        &self,
        params: &StockItemListParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = stock_list(params);
        self.export(path, async move {
            wish_list::export_wish_list_json(dto(query.into()).map_err(to_backend)?).await
        })
    }

    pub fn get_syndicate_item_pagination(
        &self,
        params: &SyndicateItemListParams,
    ) -> impl Future<Output = DataResult<Paginated<SyndicateItem>>> + Send + 'static {
        let query = syndicate_list(params);
        self.call(async move {
            let page = syndicate_item::get_syndicate_item_pagination(dto(query.into())?).await?;
            Ok(convert::paginated(page, convert::syndicate_item))
        })
    }

    pub fn get_syndicate_item_status_counts(
        &self,
    ) -> impl Future<Output = DataResult<BTreeMap<StockStatus, usize>>> + Send + 'static {
        self.call(async move {
            Ok(status_counts(
                syndicate_item::get_syndicate_item_status_counts(dto(all())?).await?,
            ))
        })
    }

    pub fn get_syndicate_item_syndicate_counts(
        &self,
    ) -> impl Future<Output = DataResult<BTreeMap<String, usize>>> + Send + 'static {
        self.call(async move {
            let counts = syndicate_item::get_syndicate_item_syndicate_counts(dto(all())?).await?;
            Ok(counts.into_iter().collect())
        })
    }

    pub fn get_syndicate_item_financial_report(
        &self,
    ) -> impl Future<Output = DataResult<FinancialReport>> + Send + 'static {
        self.call(async move {
            Ok(convert::report(
                syndicate_item::get_syndicate_item_financial_report(dto(all())?).await?,
            ))
        })
    }

    pub fn syndicate_item_create(
        &self,
        input: NewSyndicateItem,
    ) -> impl Future<Output = DataResult<SyndicateItem>> + Send + 'static {
        self.call(async move {
            let input = dto(json!({
                "raw": input.raw,
                "sub_type": sub_type_json(&input.sub_type),
                "raw_syndicate": input.syndicate_unique_name,
            }))?;
            Ok(convert::syndicate_item(
                syndicate_item::syndicate_item_create(input).await?,
            ))
        })
    }

    pub fn syndicate_item_sell(
        &self,
        wfm_url: String,
        sub_type: Option<SubType>,
        syndicate_unique_name: String,
        price: i64,
    ) -> impl Future<Output = DataResult<SyndicateItem>> + Send + 'static {
        self.call(async move {
            let model = syndicate_item::syndicate_item_sell(
                wfm_url,
                convert::backend_sub_type(sub_type),
                syndicate_unique_name,
                price,
            )
            .await?;
            Ok(convert::syndicate_item(model))
        })
    }

    pub fn syndicate_item_update(
        &self,
        id: i64,
        update: SyndicateItemUpdate,
    ) -> impl Future<Output = DataResult<SyndicateItem>> + Send + 'static {
        self.call(async move {
            let input = dto(with_properties(
                id,
                &[
                    ("list_price", update.list_price.map(|v| json!(v))),
                    ("standing_cost", update.standing_cost.map(|v| json!(v))),
                ],
                &[("min_price", limit(update.min_price))],
            ))?;
            Ok(convert::syndicate_item(
                syndicate_item::syndicate_item_update(input).await?,
            ))
        })
    }

    pub fn syndicate_item_delete(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<SyndicateItem>> + Send + 'static {
        self.call(async move {
            Ok(convert::syndicate_item(
                syndicate_item::syndicate_item_delete(id).await?,
            ))
        })
    }

    pub fn syndicate_item_delete_multiple(
        &self,
        ids: Vec<i64>,
    ) -> impl Future<Output = DataResult<i64>> + Send + 'static {
        self.call(async move { Ok(syndicate_item::syndicate_item_delete_multiple(ids).await?) })
    }

    pub fn syndicate_item_import_items(
        &self,
    ) -> impl Future<Output = DataResult<i64>> + Send + 'static {
        self.call(
            async move { Ok(syndicate_item::syndicate_item_import_items(State::get()).await?) },
        )
    }

    pub fn export_syndicate_item_json(
        &self,
        params: &SyndicateItemListParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = syndicate_list(params);
        self.export(path, async move {
            syndicate_item::export_syndicate_item_json(dto(query.into()).map_err(to_backend)?).await
        })
    }
}

fn stock_item_changes(id: i64, update: &StockItemUpdate) -> Value {
    with_properties(
        id,
        &[
            ("owned", update.owned.map(|v| json!(v))),
            ("bought", update.bought.map(|v| json!(v))),
            ("list_price", update.list_price.map(|v| json!(v))),
            ("is_hidden", update.is_hidden.map(|v| json!(v))),
        ],
        &[
            ("min_price", limit(update.min_price)),
            ("min_sma", limit(update.min_sma)),
            ("min_profit", limit(update.min_profit)),
            ("is_bulk", update.is_bulk.map(|v| json!(v))),
        ],
    )
}

fn stock_riven_changes(id: i64, update: &StockRivenUpdate) -> Value {
    with_properties(
        id,
        &[
            ("bought", update.bought.map(|v| json!(v))),
            ("list_price", update.list_price.map(|v| json!(v))),
            ("re_rolls", update.re_rolls.map(|v| json!(v))),
            ("mastery_rank", update.mastery_rank.map(|v| json!(v))),
            ("is_hidden", update.is_hidden.map(|v| json!(v))),
            (
                "filter",
                update.filter.as_ref().map(convert::backend_riven_filter),
            ),
        ],
        &[("min_price", limit(update.min_price))],
    )
}

fn wish_list_changes(id: i64, update: &WishListUpdate) -> Value {
    with_properties(
        id,
        &[
            ("quantity", update.quantity.map(|v| json!(v))),
            ("list_price", update.list_price.map(|v| json!(v))),
            ("is_hidden", update.is_hidden.map(|v| json!(v))),
        ],
        &[
            ("min_price", limit(update.min_price)),
            ("max_price", limit(update.max_price)),
            ("is_bulk", update.is_bulk.map(|v| json!(v))),
        ],
    )
}
