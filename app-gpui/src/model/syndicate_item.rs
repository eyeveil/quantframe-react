use super::{BulkEdit, CoolDownInfo, Limit, PriceHistory, StockStatus, SubType};

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyndicateItemProperties {
    pub min_price: Option<i64>,
    pub cooldown: Option<CoolDownInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewSyndicateItem {
    pub raw: String,
    pub sub_type: Option<SubType>,
    pub syndicate_unique_name: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyndicateItemUpdate {
    pub list_price: Option<i64>,
    pub standing_cost: Option<i64>,
    pub min_price: BulkEdit<Limit>,
}
