use super::{PriceHistory, StockStatus, SubType};

/// Mirrors `entity::stock_item::Model`.
#[derive(Clone, Debug, PartialEq)]
pub struct StockItem {
    pub id: i64,
    pub wfm_id: String,
    pub wfm_url: String,
    pub item_name: String,
    pub item_unique_name: String,
    pub sub_type: Option<SubType>,
    /// Average platinum paid per unit.
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

/// The keys the backend allows in a stock item's `properties`
/// (`ALLOWED_PROPERTIES_FIELDS`), typed as in `StockEntryPropertiesBase`.
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

    /// An override of the scraper's global profit or moving-average settings.
    pub fn has_override(&self) -> bool {
        self.properties.min_sma.is_some() || self.properties.min_profit.is_some()
    }

    pub fn profit_per_unit(&self) -> Option<i64> {
        self.list_price.map(|price| price - self.bought)
    }
}
