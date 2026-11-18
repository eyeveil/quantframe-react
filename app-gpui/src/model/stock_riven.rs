use super::{PriceHistory, StockStatus};

/// Mirrors `entity::stock_riven::Model`.
#[derive(Clone, Debug, PartialEq)]
pub struct StockRiven {
    pub id: i64,
    pub uuid: String,
    pub wfm_weapon_id: String,
    pub wfm_weapon_url: String,
    pub weapon_name: String,
    pub weapon_type: String,
    pub weapon_unique_name: String,
    pub mod_name: String,
    pub rank: Option<i64>,
    pub mastery_rank: i64,
    pub re_rolls: i64,
    pub polarity: String,
    pub attributes: Vec<RivenAttribute>,
    pub bought: i64,
    pub list_price: Option<i64>,
    pub is_hidden: bool,
    pub comment: String,
    pub status: StockStatus,
    pub price_history: Vec<PriceHistory>,
    pub updated_at: String,
    pub created_at: String,
    pub properties: StockRivenProperties,
    pub filter: RivenFilter,
}

/// The keys the backend allows in a stock riven's `properties`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StockRivenProperties {
    pub min_price: Option<i64>,
    /// The list price came from the price model, not from live auctions.
    pub is_predicted: bool,
}

/// Mirrors `entity::stock_riven::RivenAttribute`.
#[derive(Clone, Debug, PartialEq)]
pub struct RivenAttribute {
    pub wfm_url: String,
    pub positive: bool,
    pub value: f64,
    /// The cache's display label, such as "Critical Chance". The data layer
    /// fills it, and input ignores it.
    pub label: String,
}

impl RivenAttribute {
    pub fn new(wfm_url: impl Into<String>, positive: bool, value: f64) -> Self {
        Self {
            wfm_url: wfm_url.into(),
            positive,
            value,
            label: String::new(),
        }
    }
}

/// Mirrors `entity::stock_riven::MatchRivenStruct`: the auctions the live
/// scraper compares a riven with when it prices it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RivenFilter {
    pub enabled: bool,
    pub mastery_rank: Option<MinMax>,
    pub re_rolls: Option<MinMax>,
    pub polarity: Option<String>,
    /// The least similarity in percent.
    pub similarity: Option<f64>,
    pub required_negative: bool,
    pub attributes: Vec<RivenFilterAttribute>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinMax {
    pub min: i64,
    pub max: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RivenFilterAttribute {
    pub wfm_url: String,
    pub positive: bool,
    pub is_required: bool,
}

impl StockRiven {
    pub fn title(&self) -> String {
        format!("{} {}", self.weapon_name, self.mod_name)
    }
}

/// Input of `stock_riven_create`.
#[derive(Clone, Debug, PartialEq)]
pub struct NewStockRiven {
    pub wfm_weapon_url: String,
    pub mod_name: String,
    pub mastery_rank: i64,
    pub re_rolls: i64,
    pub rank: i64,
    pub polarity: String,
    pub attributes: Vec<RivenAttribute>,
    pub bought: i64,
}

/// Input of `stock_riven_update`. `None` leaves a field unchanged. The
/// backend clears a minimum price that is set to 0.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StockRivenUpdate {
    pub bought: Option<i64>,
    pub list_price: Option<i64>,
    pub re_rolls: Option<i64>,
    pub mastery_rank: Option<i64>,
    pub is_hidden: Option<bool>,
    pub min_price: Option<i64>,
    pub filter: Option<RivenFilter>,
}
