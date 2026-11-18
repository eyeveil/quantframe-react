//! Lists from the backend's item cache that the add dialogs choose from.

/// Mirrors `cache::types::CacheTradableItem`.
#[derive(Clone, Debug, PartialEq)]
pub struct TradableItem {
    pub wfm_id: String,
    pub wfm_url: String,
    pub name: String,
    pub unique_name: String,
    pub tags: Vec<String>,
    /// Warframe Market lists the item in batches.
    pub bulk_tradable: bool,
    /// The highest rank of a mod or arcane.
    pub max_rank: Option<i64>,
    pub variants: Vec<String>,
}

/// A weapon that has rivens, from `cache_get_riven_weapons`.
#[derive(Clone, Debug, PartialEq)]
pub struct RivenWeapon {
    /// The weapon's riven slug on Warframe Market, such as `rubico`.
    pub wfm_url: String,
    pub wfm_id: String,
    pub name: String,
    pub unique_name: String,
    /// `rifle`, `pistol`, `melee` and so on.
    pub riven_type: String,
    pub disposition: f64,
}

/// A stat a riven can roll, from `cache_get_riven_attributes`.
#[derive(Clone, Debug, PartialEq)]
pub struct RivenStat {
    pub wfm_url: String,
    /// The display label, such as "Critical Chance".
    pub label: String,
    pub unit: Option<String>,
    pub positive_only: bool,
    pub negative_only: bool,
    /// The riven types the stat rolls on. Empty means every type.
    pub exclusive_to: Vec<String>,
}

impl RivenStat {
    pub fn fits(&self, riven_type: &str) -> bool {
        self.exclusive_to.is_empty() || self.exclusive_to.iter().any(|kind| kind == riven_type)
    }
}
