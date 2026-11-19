use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use serde::{Deserialize, Serialize};
use utils::{info, LoggerOptions, SubType};
use wf_market::{
    errors::ApiError,
    types::{Auction, CreateAuctionParams, CreateOrderParams, Order, UpdateAuctionParams, UpdateOrderParams},
    Authenticated, Client,
};

use crate::{
    live_scraper::retry::{retry_server_errors, SERVER_ERRORS},
    send_event,
    types::UIEvent,
    utils::modules::states,
};

const KEEP: usize = 500;

static INTENDED: Mutex<VecDeque<IntendedOrder>> = Mutex::new(VecDeque::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteAction {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketKind {
    Order,
    Auction,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intent {
    pub component: String,
    pub item_name: String,
    pub wfm_url: String,
    pub sub_type: Option<SubType>,
    pub order_type: Option<String>,
    pub price: Option<i64>,
    pub old_price: Option<i64>,
    pub quantity: Option<i64>,
    #[serde(default)]
    pub profit: Option<i64>,
    pub reason: String,
}

impl Intent {
    pub fn new(component: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            reason: reason.into(),
            ..Self::default()
        }
    }

    pub fn item(mut self, name: impl Into<String>, wfm_url: impl Into<String>, sub_type: Option<SubType>) -> Self {
        self.item_name = name.into();
        self.wfm_url = wfm_url.into();
        self.sub_type = sub_type;
        self
    }

    pub fn order_type(mut self, order_type: impl Into<String>) -> Self {
        self.order_type = Some(order_type.into());
        self
    }

    pub fn price(mut self, price: i64, old_price: Option<i64>) -> Self {
        self.price = Some(price);
        self.old_price = old_price;
        self
    }

    pub fn profit(mut self, profit: i64) -> Self {
        self.profit = Some(profit);
        self
    }

    pub fn quantity(mut self, quantity: i64) -> Self {
        self.quantity = Some(quantity);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntendedOrder {
    pub id: u64,
    pub created_at: String,
    pub action: WriteAction,
    pub kind: MarketKind,
    pub market_id: String,
    #[serde(flatten)]
    pub intent: Intent,
}

pub enum Written<T> {
    Sent(T),
    Intended(IntendedOrder),
}

pub fn is_dry_run() -> bool {
    states::get_settings()
        .map(|settings| settings.live_scraper.general.dry_run)
        .unwrap_or(true)
}

pub fn intended_orders() -> Vec<IntendedOrder> {
    INTENDED
        .lock()
        .map(|list| list.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn clear_intended_orders() {
    if let Ok(mut list) = INTENDED.lock() {
        list.clear();
    }
    send_event!(UIEvent::OnIntendedOrder, json!(null));
}

pub fn seed_intended_orders(records: Vec<IntendedOrder>) {
    if let Ok(mut list) = INTENDED.lock() {
        for record in records {
            NEXT_ID.fetch_max(record.id + 1, Ordering::SeqCst);
            list.push_back(record);
        }
        list.truncate(KEEP);
    }
}

fn record(action: WriteAction, kind: MarketKind, market_id: &str, intent: Intent) -> IntendedOrder {
    let entry = IntendedOrder {
        id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
        created_at: chrono::Utc::now().to_rfc3339(),
        action,
        kind,
        market_id: market_id.to_owned(),
        intent,
    };
    info(
        format!("{}DryRun", entry.intent.component),
        &format!(
            "Dry run, not sent: {:?} {:?} {} for {} at {:?} ({})",
            entry.action,
            entry.kind,
            entry.market_id,
            entry.intent.item_name,
            entry.intent.price,
            entry.intent.reason
        ),
        &LoggerOptions::default().set_file("live_scraper_dry_run.log"),
    );
    if let Ok(mut list) = INTENDED.lock() {
        list.push_front(entry.clone());
        list.truncate(KEEP);
    }
    send_event!(UIEvent::OnIntendedOrder, json!(entry));
    entry
}

pub async fn create_order(
    client: &Client<Authenticated>,
    params: CreateOrderParams,
    intent: Intent,
) -> Result<Written<Order>, ApiError> {
    if is_dry_run() {
        return Ok(Written::Intended(record(WriteAction::Create, MarketKind::Order, "", intent)));
    }
    client.order().create(params).await.map(Written::Sent)
}

pub async fn update_order(
    client: &Client<Authenticated>,
    order_id: &str,
    params: impl Fn() -> UpdateOrderParams,
    intent: Intent,
) -> Result<Written<Order>, ApiError> {
    if is_dry_run() {
        return Ok(Written::Intended(record(WriteAction::Update, MarketKind::Order, order_id, intent)));
    }
    retry_server_errors("Update order", &SERVER_ERRORS, || {
        let route = client.order();
        let params = params();
        async move { route.update(order_id, params).await }
    })
        .await
        .map(Written::Sent)
}

pub async fn delete_order(
    client: &Client<Authenticated>,
    order_id: &str,
    intent: Intent,
) -> Result<Written<Order>, ApiError> {
    if is_dry_run() {
        return Ok(Written::Intended(record(WriteAction::Delete, MarketKind::Order, order_id, intent)));
    }
    retry_server_errors("Delete order", &SERVER_ERRORS, || {
        let route = client.order();
        async move { route.delete(order_id).await }
    })
        .await
        .map(Written::Sent)
}

pub async fn create_auction(
    client: &Client<Authenticated>,
    params: CreateAuctionParams,
    intent: Intent,
) -> Result<Written<Auction>, ApiError> {
    if is_dry_run() {
        return Ok(Written::Intended(record(WriteAction::Create, MarketKind::Auction, "", intent)));
    }
    client.auction().create(params).await.map(Written::Sent)
}

pub async fn update_auction(
    client: &Client<Authenticated>,
    auction_id: &str,
    params: impl Fn() -> UpdateAuctionParams,
    intent: Intent,
) -> Result<Written<Auction>, ApiError> {
    if is_dry_run() {
        return Ok(Written::Intended(record(WriteAction::Update, MarketKind::Auction, auction_id, intent)));
    }
    retry_server_errors("Update auction", &SERVER_ERRORS, || {
        let route = client.auction();
        let params = params();
        async move { route.update(auction_id, params).await }
    })
        .await
        .map(Written::Sent)
}

pub async fn delete_auction(
    client: &Client<Authenticated>,
    auction_id: &str,
    intent: Intent,
) -> Result<Written<String>, ApiError> {
    if is_dry_run() {
        return Ok(Written::Intended(record(WriteAction::Delete, MarketKind::Auction, auction_id, intent)));
    }
    retry_server_errors("Delete auction", &SERVER_ERRORS, || {
        let route = client.auction();
        async move { route.delete(auction_id).await }
    })
        .await
        .map(Written::Sent)
}
