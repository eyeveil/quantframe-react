use super::SubType;

#[derive(Clone, Debug, PartialEq)]
pub struct TradeEntry {
    pub id: i64,
    pub wfm_id: String,
    pub name: String,
    pub sub_type: Option<SubType>,
    pub price: i64,
    pub group: String,
    pub tags: Vec<String>,
    pub updated_at: String,
    pub created_at: String,
    pub properties: TradeEntryProperties,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TradeEntryProperties {
    pub min_price: Option<i64>,
    pub potential_profit: Option<i64>,
    pub last_updated: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewTradeEntry {
    pub raw: String,
    pub sub_type: Option<SubType>,
    pub price: i64,
    pub group: TradeEntryGroup,
    pub tags: Vec<String>,
    pub override_existing: bool,
    pub properties: TradeEntryProperties,
}

impl NewTradeEntry {
    pub fn new(raw: impl Into<String>, group: TradeEntryGroup, price: i64) -> Self {
        Self {
            raw: raw.into(),
            sub_type: None,
            price,
            group,
            tags: Vec::new(),
            override_existing: false,
            properties: TradeEntryProperties::default(),
        }
    }
}

pub fn tag_label(tag: &str) -> String {
    let words = tag.trim().replace(['_', '-'], " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeEntryGroup {
    Item,
    Riven,
    Custom,
}

impl TradeEntryGroup {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Riven => "riven",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TradeEntryUpdate {
    pub price: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransactionType {
    Sale,
    Purchase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransactionItemType {
    Item,
    Riven,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    pub id: i64,
    pub wfm_id: String,
    pub wfm_url: String,
    pub item_name: String,
    pub item_type: TransactionItemType,
    pub item_unique_name: String,
    pub sub_type: Option<SubType>,
    pub tags: Vec<String>,
    pub transaction_type: TransactionType,
    pub quantity: i64,
    pub user_name: String,
    pub price: i64,
    pub profit: Option<i64>,
    pub credits: i64,
    pub updated_at: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransactionUpdate {
    pub price: Option<i64>,
    pub quantity: Option<i64>,
    pub user_name: Option<String>,
    pub credits: Option<i64>,
}
