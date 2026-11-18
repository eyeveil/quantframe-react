use std::{future::Future, path::PathBuf};

use qf_app_lib::{
    State,
    commands::{dashboard, item, market, riven, syndicate_price, trade_entry, transaction},
    entity::{enums::FieldChange, transaction::UpdateTransaction},
    qf_api::{enums::TimeGroup, types::UserActiveHistoryQueryDto},
};
use serde_json::{Map, Value, json};

use super::{changes, dto, list, to_backend};
use crate::{
    data::{DataResult, DataSource, convert},
    model::*,
};

fn change<T>(value: Option<T>) -> FieldChange<T> {
    value.map_or(FieldChange::Ignore, FieldChange::Value)
}

fn trade_entry_query(params: &TradeEntryListParams) -> Value {
    let mut query = list(&params.list);
    if let Some(group) = &params.group {
        query.insert("group".into(), json!(group));
    }
    if !params.tags.is_empty() {
        query.insert("tags".into(), json!(params.tags));
    }
    query.into()
}

fn transaction_query(params: &TransactionListParams) -> Value {
    let mut query = list(&params.list);
    if let Some(kind) = params.transaction_type {
        query.insert(
            "transaction_type".into(),
            json!(convert::backend_transaction_type(kind)),
        );
    }
    if let Some(kind) = params.item_type {
        query.insert("item_type".into(), json!(convert::backend_item_type(kind)));
    }
    if let Some(from) = &params.from_date {
        query.insert("from_date".into(), json!(from));
    }
    if let Some(to) = &params.to_date {
        query.insert("to_date".into(), json!(to));
    }
    query.into()
}

fn price_query(params: &PriceLookupParams) -> Value {
    let mut query = Map::new();
    query.insert("page".into(), json!(params.page));
    query.insert("limit".into(), json!(params.limit));
    query.insert("from_date".into(), json!(params.from_date));
    query.insert("to_date".into(), json!(params.to_date));
    if let Some(text) = params.query.as_ref().filter(|text| !text.trim().is_empty()) {
        query.insert("query".into(), json!(text));
    }
    query.into()
}

impl DataSource {
    pub fn get_trade_entry_pagination(
        &self,
        params: &TradeEntryListParams,
    ) -> impl Future<Output = DataResult<Paginated<TradeEntry>>> + Send + 'static {
        let query = trade_entry_query(params);
        self.call(async move {
            let page = trade_entry::get_trade_entry_pagination(dto(query)?).await?;
            Ok(convert::paginated(page, convert::trade_entry))
        })
    }

    pub fn trade_entry_get_by_id(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<TradeEntry>> + Send + 'static {
        self.call(async move {
            Ok(convert::trade_entry(
                trade_entry::trade_entry_get_by_id(id).await?,
            ))
        })
    }

    pub fn trade_entry_create(
        &self,
        input: NewTradeEntry,
    ) -> impl Future<Output = DataResult<TradeEntry>> + Send + 'static {
        self.call(async move {
            let input = dto(json!({
                "raw": input.raw,
                "sub_type": convert::backend_sub_type(input.sub_type),
                "price": input.price,
                "group": input.group.as_str(),
                "tags": input.tags,
                "override_existing": input.override_existing,
                "properties": convert::backend_trade_properties(&input.properties),
            }))?;
            Ok(convert::trade_entry(
                trade_entry::trade_entry_create(input).await?,
            ))
        })
    }

    pub fn trade_entry_update(
        &self,
        id: i64,
        update: TradeEntryUpdate,
    ) -> impl Future<Output = DataResult<TradeEntry>> + Send + 'static {
        self.call(async move {
            let input = dto(changes(
                id,
                &[
                    ("price", update.price.map(|v| json!(v))),
                    ("tags", update.tags.map(|v| json!(v))),
                ],
            ))?;
            Ok(convert::trade_entry(
                trade_entry::trade_entry_update(input).await?,
            ))
        })
    }

    pub fn trade_entry_delete(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<TradeEntry>> + Send + 'static {
        self.call(async move {
            Ok(convert::trade_entry(
                trade_entry::trade_entry_delete(id).await?,
            ))
        })
    }

    pub fn trade_entry_delete_multiple(
        &self,
        ids: Vec<i64>,
    ) -> impl Future<Output = DataResult<i64>> + Send + 'static {
        self.call(async move { Ok(trade_entry::trade_entry_delete_multiple(ids).await?) })
    }

    pub fn export_trade_entry_json(
        &self,
        params: &TradeEntryListParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = trade_entry_query(params);
        self.export(path, async move {
            trade_entry::export_trade_entry_json(dto(query).map_err(to_backend)?).await
        })
    }

    pub fn get_transaction_pagination(
        &self,
        params: &TransactionListParams,
    ) -> impl Future<Output = DataResult<Paginated<Transaction>>> + Send + 'static {
        let query = transaction_query(params);
        self.call(async move {
            let page = transaction::get_transaction_pagination(dto(query)?).await?;
            Ok(convert::paginated(page, convert::transaction))
        })
    }

    pub fn get_transaction_financial_report(
        &self,
        params: &TransactionListParams,
    ) -> impl Future<Output = DataResult<FinancialReport>> + Send + 'static {
        let query = transaction_query(params);
        self.call(async move {
            Ok(convert::report(
                transaction::get_transaction_financial_report(dto(query)?).await?,
            ))
        })
    }

    pub fn transaction_update(
        &self,
        id: i64,
        update: TransactionUpdate,
    ) -> impl Future<Output = DataResult<Transaction>> + Send + 'static {
        self.call(async move {
            let mut input = UpdateTransaction::new(id);
            input.price = change(update.price);
            input.quantity = change(update.quantity);
            input.user_name = change(update.user_name);
            input.credits = change(update.credits);
            Ok(convert::transaction(
                transaction::transaction_update(input).await?,
            ))
        })
    }

    pub fn transaction_delete(
        &self,
        id: i64,
    ) -> impl Future<Output = DataResult<Transaction>> + Send + 'static {
        self.call(async move {
            Ok(convert::transaction(
                transaction::transaction_delete(id).await?,
            ))
        })
    }

    pub fn transaction_delete_bulk(
        &self,
        ids: Vec<i64>,
    ) -> impl Future<Output = DataResult<u64>> + Send + 'static {
        self.call(async move { Ok(transaction::transaction_delete_bulk(ids).await?) })
    }

    pub fn transaction_calculate_tax(
        &self,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(transaction::transaction_calculate_tax(State::get()).await?) })
    }

    pub fn export_transaction_json(
        &self,
        params: &TransactionListParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = transaction_query(params);
        self.export(path, async move {
            transaction::export_transaction_json(dto(query).map_err(to_backend)?).await
        })
    }

    pub fn dashboard_summary(
        &self,
    ) -> impl Future<Output = DataResult<DashboardSummary>> + Send + 'static {
        self.call(async move {
            let value = dashboard::dashboard_summary(State::get()).await?;
            Ok(convert::dashboard(value)?)
        })
    }

    pub fn item_prices_lookup(
        &self,
        params: &PriceLookupParams,
    ) -> impl Future<Output = DataResult<Paginated<ItemPrice>>> + Send + 'static {
        let query = price_query(params);
        self.call(async move {
            let page = item::item_prices_lookup(dto(query)?, State::get()).await?;
            Ok(convert::qf_paginated(page, convert::item_price))
        })
    }

    pub fn riven_prices_lookup(
        &self,
        params: &PriceLookupParams,
    ) -> impl Future<Output = DataResult<Paginated<RivenPrice>>> + Send + 'static {
        let query = price_query(params);
        self.call(async move {
            let page = riven::riven_prices_lookup(dto(query)?, State::get()).await?;
            Ok(convert::qf_paginated(page, convert::riven_price))
        })
    }

    pub fn syndicate_item_prices_lookup(
        &self,
        params: &PriceLookupParams,
    ) -> impl Future<Output = DataResult<Paginated<SyndicateItemPrice>>> + Send + 'static {
        let query = price_query(params);
        self.call(async move {
            let page =
                syndicate_price::syndicate_item_prices_lookup(dto(query)?, State::get()).await?;
            Ok(convert::qf_paginated(page, convert::syndicate_price))
        })
    }

    pub fn export_item_price_data(
        &self,
        params: &PriceLookupParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = price_query(params);
        self.export(path, async move {
            item::export_item_price_data(dto(query).map_err(to_backend)?, State::get()).await
        })
    }

    pub fn export_riven_price_data(
        &self,
        params: &PriceLookupParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = price_query(params);
        self.export(path, async move {
            riven::export_riven_price_data(dto(query).map_err(to_backend)?, State::get()).await
        })
    }

    pub fn export_syndicate_item_price_data(
        &self,
        params: &PriceLookupParams,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<PathBuf>> + Send + 'static {
        let query = price_query(params);
        self.export(path, async move {
            syndicate_price::export_syndicate_item_price_data(
                dto(query).map_err(to_backend)?,
                State::get(),
            )
            .await
        })
    }

    pub fn get_user_activity(
        &self,
        from_date: String,
        to_date: String,
    ) -> impl Future<Output = DataResult<UserActivity>> + Send + 'static {
        self.call(async move {
            let query = UserActiveHistoryQueryDto::new(from_date, to_date, TimeGroup::Day);
            let activity = market::get_user_activity(query, State::get()).await?;
            Ok(UserActivity {
                labels: activity.labels,
                registered_users: activity.registered_users_chart,
                total_users: activity.total_users_chart,
            })
        })
    }
}
