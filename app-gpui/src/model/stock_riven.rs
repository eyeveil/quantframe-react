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
}

/// Mirrors `entity::stock_riven::RivenAttribute`.
#[derive(Clone, Debug, PartialEq)]
pub struct RivenAttribute {
    pub wfm_url: String,
    pub positive: bool,
    pub value: f64,
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

/// Input of `stock_riven_update`. `None` leaves a field unchanged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StockRivenUpdate {
    pub bought: Option<i64>,
    pub list_price: Option<i64>,
    pub re_rolls: Option<i64>,
    pub mastery_rank: Option<i64>,
    pub is_hidden: Option<bool>,
}
