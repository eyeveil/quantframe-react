use super::{BulkEdit, CoolDownInfo, Limit, PriceHistory, StockStatus, SubType};

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
    pub properties: WishListProperties,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WishListProperties {
    pub min_price: Option<i64>,
    pub max_price: Option<i64>,
    pub is_bulk: bool,
    pub cooldown: Option<CoolDownInfo>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WishListUpdate {
    pub quantity: Option<i64>,
    pub list_price: Option<i64>,
    pub is_hidden: Option<bool>,
    pub min_price: BulkEdit<Limit>,
    pub max_price: BulkEdit<Limit>,
    pub is_bulk: Option<bool>,
}
