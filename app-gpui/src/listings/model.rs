//! View models and stores for the listings tabs. Field names follow
//! `StockRiven`, `WishListItem` and `SyndicateItem` in
//! `src/types/tauri.type.ts`. Each store keeps its rows in memory behind the
//! read and write functions a backend-backed source replaces.

use std::collections::BTreeMap;

use crate::model::{PriceHistory, StockItem, StockStatus, SubType};

/// What the stores need from a row: its id, status and searchable text.
pub trait Record: Clone {
    fn id(&self) -> i64;
    fn set_id(&mut self, id: i64);
    fn status(&self) -> StockStatus;
    fn search_text(&self) -> String;
}

/// Rows of one listings tab, held in memory.
pub struct Store<T> {
    rows: Vec<T>,
    next_id: i64,
}

impl<T: Record> Store<T> {
    pub fn new(rows: Vec<T>) -> Self {
        let next_id = rows.iter().map(Record::id).max().unwrap_or(0) + 1;
        Self { rows, next_id }
    }

    pub fn rows(&self) -> &[T] {
        &self.rows
    }

    /// The rows whose text contains `text` and whose status is `status`.
    pub fn list(&self, text: &str, status: Option<StockStatus>) -> Vec<T> {
        let needle = text.trim().to_lowercase();
        self.rows
            .iter()
            .filter(|row| status.is_none_or(|status| row.status() == status))
            .filter(|row| needle.is_empty() || row.search_text().to_lowercase().contains(&needle))
            .cloned()
            .collect()
    }

    pub fn get(&self, id: i64) -> Option<&T> {
        self.rows.iter().find(|row| row.id() == id)
    }

    pub fn status_counts(&self) -> BTreeMap<StockStatus, usize> {
        let mut counts = BTreeMap::new();
        for row in &self.rows {
            *counts.entry(row.status()).or_default() += 1;
        }
        counts
    }

    pub fn insert(&mut self, mut row: T) {
        row.set_id(self.next_id);
        self.next_id += 1;
        self.rows.insert(0, row);
    }

    pub fn update(&mut self, id: i64, change: impl FnOnce(&mut T)) {
        if let Some(row) = self.rows.iter_mut().find(|row| row.id() == id) {
            change(row);
        }
    }

    pub fn delete(&mut self, ids: &[i64]) {
        self.rows.retain(|row| !ids.contains(&row.id()));
    }
}

impl Record for StockItem {
    fn id(&self) -> i64 {
        self.id
    }

    fn set_id(&mut self, id: i64) {
        self.id = id;
    }

    fn status(&self) -> StockStatus {
        self.status
    }

    fn search_text(&self) -> String {
        self.item_name.clone()
    }
}

impl Store<StockItem> {
    /// Takes `quantity` units out of stock. A row with no units left goes.
    pub fn sell(&mut self, id: i64, quantity: i64) {
        self.update(id, |item| item.owned -= quantity.min(item.owned));
        self.rows.retain(|item| item.owned > 0);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RivenAttribute {
    pub url_name: String,
    pub name: String,
    pub value: f64,
    pub positive: bool,
}

impl RivenAttribute {
    /// The attribute as the game writes it, such as `+172.4% Critical Chance`.
    pub fn label(&self) -> String {
        let sign = if self.value < 0.0 { "-" } else { "+" };
        format!("{sign}{:.1}% {}", self.value.abs(), self.name)
    }
}

/// Mirrors `MinMaxDto`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MinMax {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

/// Mirrors `StockRivenFilter`: which Warframe Market auctions count as
/// competition when the live scraper prices this riven.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StockRivenFilter {
    pub enabled: bool,
    pub re_rolls: MinMax,
    pub mastery_rank: MinMax,
    pub required_negative: bool,
    pub similarity: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StockRivenProperties {
    pub min_price: Option<i64>,
    /// The price came from the riven price model, not from live auctions.
    pub is_predicted: bool,
}

/// Mirrors `entity::stock_riven::Model`.
#[derive(Clone, Debug, PartialEq)]
pub struct StockRiven {
    pub id: i64,
    pub wfm_weapon_url: String,
    pub weapon_name: String,
    pub weapon_type: String,
    pub mod_name: String,
    pub attributes: Vec<RivenAttribute>,
    pub mastery_rank: i64,
    pub re_rolls: i64,
    pub polarity: String,
    pub sub_type: Option<SubType>,
    pub bought: i64,
    pub list_price: Option<i64>,
    pub is_hidden: bool,
    pub status: StockStatus,
    pub price_history: Vec<PriceHistory>,
    pub updated_at: String,
    pub comment: String,
    pub filter: StockRivenFilter,
    pub properties: StockRivenProperties,
}

impl StockRiven {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.weapon_name, self.mod_name)
    }

    pub fn rank(&self) -> i64 {
        self.sub_type
            .as_ref()
            .and_then(|sub_type| sub_type.rank)
            .unwrap_or(0)
    }
}

impl Record for StockRiven {
    fn id(&self) -> i64 {
        self.id
    }

    fn set_id(&mut self, id: i64) {
        self.id = id;
    }

    fn status(&self) -> StockStatus {
        self.status
    }

    fn search_text(&self) -> String {
        self.full_name()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WishListProperties {
    pub max_price: Option<i64>,
    pub is_bulk: bool,
}

/// Mirrors `entity::wish_list::Model`.
#[derive(Clone, Debug, PartialEq)]
pub struct WishListItem {
    pub id: i64,
    pub wfm_url: String,
    pub item_name: String,
    pub sub_type: Option<SubType>,
    pub quantity: i64,
    /// The price of the buy order on Warframe Market, when one is up.
    pub list_price: Option<i64>,
    pub is_hidden: bool,
    pub status: StockStatus,
    pub price_history: Vec<PriceHistory>,
    pub updated_at: String,
    pub properties: WishListProperties,
}

impl WishListItem {
    pub fn rank_label(&self) -> Option<String> {
        self.sub_type.as_ref().and_then(SubType::label)
    }
}

impl Record for WishListItem {
    fn id(&self) -> i64 {
        self.id
    }

    fn set_id(&mut self, id: i64) {
        self.id = id;
    }

    fn status(&self) -> StockStatus {
        self.status
    }

    fn search_text(&self) -> String {
        self.item_name.clone()
    }
}

impl Store<WishListItem> {
    /// Takes `quantity` units off the wish. A wish with none left goes.
    pub fn bought(&mut self, id: i64, quantity: i64) {
        self.update(id, |item| item.quantity -= quantity.min(item.quantity));
        self.rows.retain(|item| item.quantity > 0);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyndicateItemProperties {
    pub min_price: Option<i64>,
    pub min_sma: Option<i64>,
    pub min_profit: Option<i64>,
}

/// Mirrors `SyndicateItem`: something a syndicate sells for standing that
/// the live scraper lists on Warframe Market.
#[derive(Clone, Debug, PartialEq)]
pub struct SyndicateItem {
    pub id: i64,
    pub wfm_url: String,
    pub item_name: String,
    pub sub_type: Option<SubType>,
    pub syndicate_name: String,
    pub syndicate_unique_name: String,
    pub standing_cost: i64,
    pub list_price: Option<i64>,
    pub status: StockStatus,
    pub price_history: Vec<PriceHistory>,
    pub updated_at: String,
    pub properties: SyndicateItemProperties,
}

impl SyndicateItem {
    /// An override of the scraper's global profit or moving-average settings.
    pub fn has_override(&self) -> bool {
        self.properties.min_sma.is_some() || self.properties.min_profit.is_some()
    }

    /// Platinum earned per 1,000 standing spent, at the list price.
    pub fn platinum_per_thousand_standing(&self) -> Option<f64> {
        let price = self.list_price?;
        (self.standing_cost > 0).then(|| price as f64 * 1000.0 / self.standing_cost as f64)
    }
}

impl Record for SyndicateItem {
    fn id(&self) -> i64 {
        self.id
    }

    fn set_id(&mut self, id: i64) {
        self.id = id;
    }

    fn status(&self) -> StockStatus {
        self.status
    }

    fn search_text(&self) -> String {
        format!("{} {}", self.item_name, self.syndicate_name)
    }
}

impl Store<SyndicateItem> {
    pub fn syndicate_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for row in &self.rows {
            *counts.entry(row.syndicate_name.clone()).or_default() += 1;
        }
        counts
    }
}
