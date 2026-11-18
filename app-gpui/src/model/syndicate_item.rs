use super::{CoolDownInfo, PriceHistory, StockStatus, SubType};

/// Mirrors `entity::syndicate_item::Model`.
#[derive(Clone, Debug, PartialEq)]
pub struct SyndicateItem {
    pub id: i64,
    pub wfm_id: String,
    pub wfm_url: String,
    pub item_name: String,
    pub item_unique_name: String,
    pub sub_type: Option<SubType>,
    pub list_price: Option<i64>,
    pub syndicate_name: String,
    pub syndicate_unique_name: String,
    pub standing_cost: i64,
    pub status: StockStatus,
    pub price_history: Vec<PriceHistory>,
    pub updated_at: String,
    pub created_at: String,
    pub properties: SyndicateItemProperties,
}

/// The keys the backend allows in a syndicate item's `properties`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyndicateItemProperties {
    pub min_price: Option<i64>,
    pub cooldown: Option<CoolDownInfo>,
}

/// Input of `syndicate_item_create`.
#[derive(Clone, Debug, PartialEq)]
pub struct NewSyndicateItem {
    pub raw: String,
    pub sub_type: Option<SubType>,
    pub syndicate_unique_name: String,
}

/// Input of `syndicate_item_update`. `None` leaves a field unchanged. The
/// backend clears a minimum price that is set to 0.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyndicateItemUpdate {
    pub list_price: Option<i64>,
    pub standing_cost: Option<i64>,
    pub min_price: Option<i64>,
}
