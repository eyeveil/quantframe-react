//! Select entries built from the backend's cache lists.

use super::form::Choice;
use crate::model::{RivenStat, RivenWeapon, TradableItem};

/// Every tradable item by name. The value is its Warframe Market slug.
pub fn items(items: &[TradableItem]) -> Vec<Choice> {
    items
        .iter()
        .map(|item| Choice::new(item.name.clone(), item.wfm_url.clone()))
        .collect()
}

/// Every riven weapon by name. The value is its riven slug.
pub fn weapons(weapons: &[RivenWeapon]) -> Vec<Choice> {
    weapons
        .iter()
        .map(|weapon| Choice::new(weapon.name.clone(), weapon.wfm_url.clone()))
        .collect()
}

/// Every riven stat by its label. The value is its slug.
pub fn stats(stats: &[RivenStat]) -> Vec<Choice> {
    stats
        .iter()
        .map(|stat| Choice::new(stat.label.clone(), stat.wfm_url.clone()))
        .collect()
}
