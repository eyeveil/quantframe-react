//! View models and the store for the Trade messages screen. Field names
//! follow `TradeEntry` in `src/types/tauri.type.ts`. The store keeps rows in
//! memory behind the read and write functions a backend-backed source
//! replaces.

use std::collections::BTreeMap;

use crate::model::SubType;

/// The three lists of the screen, the `group` of a trade entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TradeGroup {
    Item,
    Custom,
    Riven,
}

impl TradeGroup {
    pub const ALL: [Self; 3] = [Self::Item, Self::Custom, Self::Riven];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Custom => "custom",
            Self::Riven => "riven",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Item => "Items",
            Self::Custom => "Custom",
            Self::Riven => "Rivens",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TradeEntryProperties {
    /// The lowest price seen on the market when the entry was priced.
    pub min_price: Option<i64>,
    pub potential_profit: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeEntry {
    pub id: i64,
    pub wfm_id: String,
    pub name: String,
    pub sub_type: Option<SubType>,
    pub price: i64,
    pub group: TradeGroup,
    pub tags: Vec<String>,
    pub updated_at: String,
    pub properties: TradeEntryProperties,
}

impl TradeEntry {
    pub fn rank_label(&self) -> Option<String> {
        self.sub_type.as_ref().and_then(SubType::label)
    }
}

/// A riven weapon from the last hour of market data, for "Find rivens".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RivenCandidate {
    pub wfm_id: String,
    pub name: String,
    pub min_price: i64,
    pub volume: i64,
}

/// The settings of "Find rivens".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RivenSearch {
    pub min_price: i64,
    pub min_volume: i64,
    pub discount_percent: i64,
    pub round_to: i64,
    pub override_existing: bool,
    pub tags: Vec<String>,
}

pub struct TradeStore {
    rows: Vec<TradeEntry>,
    candidates: Vec<RivenCandidate>,
    next_id: i64,
}

impl TradeStore {
    pub fn new(rows: Vec<TradeEntry>, candidates: Vec<RivenCandidate>) -> Self {
        let next_id = rows.iter().map(|row| row.id).max().unwrap_or(0) + 1;
        Self {
            rows,
            candidates,
            next_id,
        }
    }

    pub fn list(&self, group: TradeGroup, text: &str, tag: Option<&str>) -> Vec<TradeEntry> {
        let needle = text.trim().to_lowercase();
        self.rows
            .iter()
            .filter(|entry| entry.group == group)
            .filter(|entry| tag.is_none_or(|tag| entry.tags.iter().any(|own| own == tag)))
            .filter(|entry| needle.is_empty() || entry.name.to_lowercase().contains(&needle))
            .cloned()
            .collect()
    }

    pub fn get(&self, id: i64) -> Option<&TradeEntry> {
        self.rows.iter().find(|entry| entry.id == id)
    }

    pub fn tag_counts(&self, group: TradeGroup) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for entry in self.rows.iter().filter(|entry| entry.group == group) {
            for tag in &entry.tags {
                *counts.entry(tag.clone()).or_default() += 1;
            }
        }
        counts
    }

    pub fn create(
        &mut self,
        group: TradeGroup,
        name: String,
        sub_type: Option<SubType>,
        price: i64,
        tags: Vec<String>,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        self.rows.insert(
            0,
            TradeEntry {
                id,
                wfm_id: String::new(),
                name,
                sub_type,
                price,
                group,
                tags,
                updated_at: String::new(),
                properties: TradeEntryProperties::default(),
            },
        );
    }

    pub fn update(&mut self, id: i64, change: impl FnOnce(&mut TradeEntry)) {
        if let Some(entry) = self.rows.iter_mut().find(|entry| entry.id == id) {
            change(entry);
        }
    }

    pub fn delete(&mut self, ids: &[i64]) {
        self.rows.retain(|entry| !ids.contains(&entry.id));
    }

    /// Adds a discounted riven entry for every weapon that passes `search`.
    /// Returns how many entries it added or repriced.
    pub fn find_rivens(&mut self, search: &RivenSearch) -> usize {
        let round = search.round_to.max(1);
        let matches: Vec<RivenCandidate> = self
            .candidates
            .iter()
            .filter(|riven| riven.min_price >= search.min_price)
            .filter(|riven| riven.volume >= search.min_volume)
            .cloned()
            .collect();
        let mut changed = 0;
        for riven in matches {
            let discounted = riven.min_price * (100 - search.discount_percent) / 100;
            let price = (discounted / round * round).max(1);
            let properties = TradeEntryProperties {
                min_price: Some(riven.min_price),
                potential_profit: Some(riven.min_price - price),
            };
            let existing = self
                .rows
                .iter_mut()
                .find(|entry| entry.group == TradeGroup::Riven && entry.name == riven.name);
            match existing {
                Some(entry) if search.override_existing => {
                    entry.price = price;
                    entry.properties = properties;
                    changed += 1;
                }
                Some(_) => {}
                None => {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.rows.insert(
                        0,
                        TradeEntry {
                            id,
                            wfm_id: riven.wfm_id,
                            name: riven.name,
                            sub_type: None,
                            price,
                            group: TradeGroup::Riven,
                            tags: search.tags.clone(),
                            updated_at: String::new(),
                            properties,
                        },
                    );
                    changed += 1;
                }
            }
        }
        changed
    }
}
