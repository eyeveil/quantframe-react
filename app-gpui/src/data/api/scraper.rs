use std::future::Future;

use qf_app_lib::{
    commands::live_scraper,
    live_scraper::writes::{self, MarketKind, WriteAction},
};

use crate::{
    data::{DataResult, DataSource, convert},
    model::{IntendedAction, IntendedOrder, OrderType},
};

pub fn intended_order(record: writes::IntendedOrder) -> IntendedOrder {
    IntendedOrder {
        id: record.id,
        created_at: record.created_at,
        action: match record.action {
            WriteAction::Create => IntendedAction::Create,
            WriteAction::Update => IntendedAction::Update,
            WriteAction::Delete => IntendedAction::Delete,
        },
        is_auction: record.kind == MarketKind::Auction,
        market_id: record.market_id,
        item_name: record.intent.item_name,
        wfm_url: record.intent.wfm_url,
        sub_type: convert::sub_type(record.intent.sub_type),
        order_type: match record.intent.order_type.as_deref() {
            Some("buy") => Some(OrderType::Buy),
            Some("sell") => Some(OrderType::Sell),
            _ => None,
        },
        price: record.intent.price,
        old_price: record.intent.old_price,
        quantity: record.intent.quantity,
        reason: record.intent.reason,
    }
}

impl DataSource {
    pub fn live_scraper_get_intended_orders(
        &self,
    ) -> impl Future<Output = DataResult<Vec<IntendedOrder>>> + Send + 'static {
        self.call(async move {
            let records = live_scraper::live_scraper_get_intended_orders().await?;
            Ok(records.into_iter().map(intended_order).collect())
        })
    }

    pub fn live_scraper_clear_intended_orders(
        &self,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(live_scraper::live_scraper_clear_intended_orders().await?) })
    }
}
