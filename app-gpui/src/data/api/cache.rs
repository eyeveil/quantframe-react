use std::future::Future;

use qf_app_lib::{State, commands::cache};

use crate::{
    data::{DataResult, DataSource, convert},
    model::*,
};

impl DataSource {
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
