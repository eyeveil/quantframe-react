//! Trading analytics: Quantframe price lookups and the market activity chart.

use super::SubType;

/// Mirrors `qf_api::types::ItemPrice`.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemPrice {
    pub wfm_id: String,
    pub wfm_url: String,
    pub name: String,
    pub sub_type: Option<SubType>,
    pub avg_price: f64,
    pub closed_price: f64,
    pub volume: f64,
    pub supply: f64,
    pub demand: f64,
}

/// Mirrors `qf_api::types::RivenPrice`.
#[derive(Clone, Debug, PartialEq)]
pub struct RivenPrice {
    pub wfm_url: String,
    pub name: String,
    pub volume: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,
}

/// Mirrors `qf_api::types::SyndicateItemPrice`.
#[derive(Clone, Debug, PartialEq)]
pub struct SyndicateItemPrice {
    pub wfm_id: String,
    pub name: String,
    pub syndicate: String,
    pub standing_cost: i64,
    pub min_price: f64,
    pub max_price: f64,
    pub volume: f64,
}

/// Mirrors `qf_api::types::UserActivityDto`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserActivity {
    pub labels: Vec<String>,
    pub registered_users: Vec<f64>,
    pub total_users: Vec<f64>,
}

/// Query of the three price lookups. Dates are `YYYY-MM-DD`. The item and
/// riven lookups need them, and the syndicate lookup ignores them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriceLookupParams {
    pub page: i64,
    pub limit: i64,
    pub query: Option<String>,
    pub from_date: String,
    pub to_date: String,
}

impl PriceLookupParams {
    pub fn new(from_date: impl Into<String>, to_date: impl Into<String>) -> Self {
        Self {
            page: 1,
            limit: 25,
            query: None,
            from_date: from_date.into(),
            to_date: to_date.into(),
        }
    }
}
