//! The item cache lists the add dialogs choose from.

use std::future::Future;

use qf_app_lib::{State, commands::cache};

use crate::{
    data::{DataResult, DataSource, convert},
    model::*,
};

impl DataSource {
    /// Every item Warframe Market trades, sorted by name.
    pub fn cache_get_tradable_items(
        &self,
    ) -> impl Future<Output = DataResult<Vec<TradableItem>>> + Send + 'static {
        self.call(async move {
            let mut items: Vec<TradableItem> = cache::cache_get_tradable_items(State::get())
                .await?
                .into_iter()
                .map(convert::tradable_item)
                .collect();
            items.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(items)
        })
    }

    /// Every weapon that has rivens, sorted by name.
    pub fn cache_get_riven_weapons(
        &self,
    ) -> impl Future<Output = DataResult<Vec<RivenWeapon>>> + Send + 'static {
        self.call(async move {
            let mut weapons: Vec<RivenWeapon> = cache::cache_get_riven_weapons(State::get())
                .await?
                .into_iter()
                .map(convert::riven_weapon)
                .collect();
            weapons.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(weapons)
        })
    }

    /// Every stat a riven can roll, with its display label, sorted by label.
    pub fn cache_get_riven_attributes(
        &self,
    ) -> impl Future<Output = DataResult<Vec<RivenStat>>> + Send + 'static {
        self.call(async move {
            let mut stats: Vec<RivenStat> = cache::cache_get_riven_attributes(State::get())
                .await?
                .into_iter()
                .map(convert::riven_stat)
                .collect();
            stats.sort_by(|a, b| a.label.cmp(&b.label));
            Ok(stats)
        })
    }
}
