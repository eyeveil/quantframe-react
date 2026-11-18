use super::{PriceHistory, StockStatus, SubType};

/// Mirrors `entity::wish_list::Model`.
#[derive(Clone, Debug, PartialEq)]
pub struct WishListItem {
    pub id: i64,
    pub wfm_id: String,
    pub wfm_url: String,
    pub item_name: String,
    pub item_unique_name: String,
    pub sub_type: Option<SubType>,
    pub quantity: i64,
    pub list_price: Option<i64>,
    pub is_hidden: bool,
    pub status: StockStatus,
    pub price_history: Vec<PriceHistory>,
    pub updated_at: String,
    pub created_at: String,
}

/// Input of `wish_list_update`. `None` leaves a field unchanged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WishListUpdate {
    pub quantity: Option<i64>,
    pub list_price: Option<i64>,
    pub is_hidden: Option<bool>,
}
