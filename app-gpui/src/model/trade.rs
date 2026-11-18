//! Trade messages (`trade_entry`) and the transaction ledger.

use super::SubType;

/// Mirrors `entity::trade_entry::Model`.
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

/// What "Find rivens" stores on a riven entry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TradeEntryProperties {
    /// The lowest price on the market when the entry was priced.
    pub min_price: Option<i64>,
    /// The market low minus the entry's price.
    pub potential_profit: Option<i64>,
    /// The day the market low is from, `YYYY-MM-DD`.
    pub last_updated: Option<String>,
}

/// Input of `trade_entry_create`.
#[derive(Clone, Debug, PartialEq)]
pub struct NewTradeEntry {
    pub raw: String,
    pub sub_type: Option<SubType>,
    pub price: i64,
    pub group: TradeEntryGroup,
    pub tags: Vec<String>,
    /// Reprices an existing entry for the same item instead of failing.
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

/// A tag as words with a capital first letter, such as "Arcane enhancement"
/// for `arcane_enhancement`.
pub fn tag_label(tag: &str) -> String {
    let words = tag.trim().replace(['_', '-'], " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The groups `trade_entry_create` accepts. `raw` is a warframe.market item
/// for `Item`, a riven weapon for `Riven`, and free text for `Custom`.
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

/// Input of `trade_entry_update`. `None` leaves a field unchanged.
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

/// Mirrors `entity::transaction::Model`.
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

/// Input of `transaction_update`. `None` leaves a field unchanged.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransactionUpdate {
    pub price: Option<i64>,
    pub quantity: Option<i64>,
    pub user_name: Option<String>,
    pub credits: Option<i64>,
}
