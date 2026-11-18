mod app;
mod cache;
mod inventory;
mod market;
mod stock;
mod trades;

use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use super::{DataError, DataResult};
use crate::model::{
    BulkEdit, Limit, ListParams, SortDirection, StockItemListParams, StockStatus,
    SyndicateItemListParams,
};

fn dto<T: DeserializeOwned>(value: Value) -> DataResult<T> {
    serde_json::from_value(value)
        .map_err(|error| DataError::new("DataSource:Encode", error.to_string()))
}

fn to_backend(error: DataError) -> qf_app_lib::Error {
    qf_app_lib::Error::new(&error.component, error.to_string(), String::new())
}

fn direction(direction: SortDirection) -> &'static str {
    match direction {
        SortDirection::Asc => "asc",
        SortDirection::Desc => "desc",
    }
}

fn list_query(
    page: i64,
    limit: i64,
    sort_by: &Option<String>,
    sort: SortDirection,
    query: &Option<String>,
) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("page".into(), json!(page));
    map.insert("limit".into(), json!(limit));
    if let Some(sort_by) = sort_by {
        map.insert("sort_by".into(), json!(sort_by));
        map.insert("sort_direction".into(), json!(direction(sort)));
    }
    if let Some(query) = query.as_ref().filter(|query| !query.trim().is_empty()) {
        map.insert("query".into(), json!(query));
    }
    map
}

fn list(params: &ListParams) -> Map<String, Value> {
    list_query(
        params.page,
        params.limit,
        &params.sort_by,
        params.sort_direction,
        &params.query,
    )
}

fn stock_list(params: &StockItemListParams) -> Map<String, Value> {
    let mut map = list_query(
        params.page,
        params.limit,
        &params.sort_by,
        params.sort_direction,
        &params.query,
    );
    if let Some(status) = params.status {
        map.insert("status".into(), json!(status.as_str()));
    }
    map
}

fn syndicate_list(params: &SyndicateItemListParams) -> Map<String, Value> {
    let mut map = stock_list(&params.stock);
    if let Some(syndicate) = &params.syndicate {
        map.insert("syndicate".into(), json!(syndicate));
    }
    map
}

fn all() -> Value {
    json!({ "page": 1, "limit": -1 })
}

fn status_counts(
    counts: std::collections::HashMap<String, usize>,
) -> std::collections::BTreeMap<StockStatus, usize> {
    let mut out = std::collections::BTreeMap::new();
    for (status, count) in counts {
        *out.entry(StockStatus::parse(&status)).or_insert(0) += count;
    }
    out
}

fn changes(id: i64, fields: &[(&str, Option<Value>)]) -> Value {
    with_properties(id, fields, &[])
}

fn limit(edit: BulkEdit<Limit>) -> Option<Value> {
    match edit {
        BulkEdit::Keep => None,
        BulkEdit::Set(Limit::Unset) => Some(json!(0)),
        BulkEdit::Set(Limit::Value(value)) => Some(json!(value)),
    }
}

fn with_properties(
    id: i64,
    fields: &[(&str, Option<Value>)],
    properties: &[(&str, Option<Value>)],
) -> Value {
    let pick = |fields: &[(&str, Option<Value>)]| {
        fields
            .iter()
            .filter_map(|(key, value)| Some(((*key).to_owned(), value.clone()?)))
            .collect::<Map<String, Value>>()
    };
    let mut map = pick(fields);
    map.insert("id".into(), json!(id));
    let properties = pick(properties);
    if !properties.is_empty() {
        map.insert("properties".into(), Value::Object(properties));
    }
    Value::Object(map)
}
