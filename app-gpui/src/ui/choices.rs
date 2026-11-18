use super::form::Choice;
use crate::model::{RivenStat, RivenWeapon, TradableItem};

pub fn items(items: &[TradableItem]) -> Vec<Choice> {
    items
        .iter()
        .map(|item| Choice::new(item.name.clone(), item.wfm_url.clone()))
        .collect()
}

pub fn weapons(weapons: &[RivenWeapon]) -> Vec<Choice> {
    weapons
        .iter()
        .map(|weapon| Choice::new(weapon.name.clone(), weapon.wfm_url.clone()))
        .collect()
}

pub fn stats(stats: &[RivenStat]) -> Vec<Choice> {
    stats
        .iter()
        .map(|stat| Choice::new(stat.label.clone(), stat.wfm_url.clone()))
        .collect()
}
