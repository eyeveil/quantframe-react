use super::{BulkEdit, Limit, PriceHistory, StockStatus, SubType};

#[derive(Clone, Debug, PartialEq)]
pub struct StockItem {
    pub id: i64,
    pub wfm_id: String,
    pub wfm_url: String,
    pub item_name: String,
    pub item_unique_name: String,
    pub sub_type: Option<SubType>,
    pub bought: i64,
    pub list_price: Option<i64>,
    pub owned: i64,
    pub is_hidden: bool,
    pub status: StockStatus,
    pub price_history: Vec<PriceHistory>,
    pub updated_at: String,
    pub created_at: String,
    pub properties: StockItemProperties,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StockItemProperties {
    pub min_price: Option<i64>,
    pub min_sma: Option<i64>,
    pub min_profit: Option<i64>,
    pub is_bulk: bool,
    pub cooldown: Option<CoolDownInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoolDownInfo {
    pub id: String,
    pub cooldown_type: String,
    pub start_time: String,
    pub end_time: String,
}

impl StockItem {
    pub fn rank_label(&self) -> Option<String> {
        self.sub_type.as_ref().and_then(SubType::label)
    }

    pub fn has_override(&self) -> bool {
        self.properties.min_sma.is_some() || self.properties.min_profit.is_some()
    }

    pub fn profit_per_unit(&self) -> Option<i64> {
        self.list_price.map(|price| price - self.bought)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewStockItem {
    pub raw: String,
    pub sub_type: Option<SubType>,
    pub quantity: i64,
    pub bought: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StockItemUpdate {
    pub owned: Option<i64>,
    pub bought: Option<i64>,
    pub list_price: Option<i64>,
    pub is_hidden: Option<bool>,
    pub min_price: BulkEdit<Limit>,
    pub min_sma: BulkEdit<Limit>,
    pub min_profit: BulkEdit<Limit>,
    pub is_bulk: Option<bool>,
}
