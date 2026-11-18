//! Backend events, typed. `GpuiHost` parses what the backend emits and
//! `Backend` re-emits it to the views that subscribe.

use serde_json::Value;

use super::convert;
use crate::model::{ChatMessage, LiveScraperMessage};

/// A list the backend changed. Views showing that list fetch it again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DataKind {
    StockItems,
    StockRivens,
    WishList,
    SyndicateItems,
    WfmOrders,
    WfmAuctions,
    Transactions,
    Chats,
    Cache,
    Settings,
    User,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BackendEvent {
    /// A startup step, as an i18n key such as `wfm.refresh`.
    StartingUp(String),
    /// `app:ready`. Every `DataSource` method works from here on.
    Ready,
    StartupFailed(String),
    /// `App:Error`. `None` clears the error.
    Error(Option<String>),
    Changed(DataKind),
    LiveScraperRunning(bool),
    LiveScraperMessage(LiveScraperMessage),
    /// `App:OnNotify`, an in-app toast.
    Notify {
        i18n_key: String,
        color: String,
        values: Value,
    },
    ChatMessage(ChatMessage),
    /// `Host::notify`, an OS notification.
    SystemNotification {
        title: String,
        body: String,
    },
    PlaySound {
        file: String,
        volume: f32,
    },
    /// The log parser saw a completed in-game trade.
    TradeDetected(Trade),
    /// An event this app doesn't handle yet.
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trade {
    pub player_name: String,
    pub trade_type: String,
    pub platinum: i64,
    pub credits: i64,
    pub item_names: Vec<String>,
}

fn text(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value[*key].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn int(value: &Value, keys: &[&str]) -> i64 {
    keys.iter()
        .find_map(|key| value[*key].as_i64())
        .unwrap_or(0)
}

impl BackendEvent {
    /// Parses one `Host::emit` call.
    pub fn parse(event: &str, payload: &Value) -> Self {
        match event {
            "app:ready" => Self::Ready,
            "message" | "message_update" => Self::parse_ui_event(
                payload["event"].as_str().unwrap_or_default(),
                &payload["data"],
            ),
            "play_sound" => Self::PlaySound {
                file: text(payload, &["file_name"]),
                volume: payload["volume"].as_f64().unwrap_or(1.0) as f32,
            },
            "add_trade" => Self::TradeDetected(Trade {
                player_name: text(payload, &["playerName", "player_name"]),
                trade_type: text(payload, &["tradeType", "trade_type"]),
                platinum: int(payload, &["platinum"]),
                credits: int(payload, &["credits"]),
                item_names: payload["items"]
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| text(&item["properties"], &["name"]))
                            .filter(|name| !name.is_empty())
                            .collect()
                    })
                    .unwrap_or_default(),
            }),
            other => Self::Other(other.to_owned()),
        }
    }

    fn parse_ui_event(name: &str, data: &Value) -> Self {
        match name {
            "App:StartingUp" => Self::StartingUp(text(data, &["i18n_key"])),
            "App:Error" => {
                let message = text(data, &["message"]);
                Self::Error((!message.is_empty()).then_some(message))
            }
            "App:OnNotify" => Self::Notify {
                i18n_key: text(data, &["i18n_key"]),
                color: text(data, &["color"]),
                values: data["values"].clone(),
            },
            "LiveScraper:UpdateRunningState" => {
                Self::LiveScraperRunning(data.as_bool().unwrap_or(false))
            }
            "LiveScraper:OnMessage" => match scraper_message(data) {
                Some(message) => Self::LiveScraperMessage(message),
                None => Self::Other(name.to_owned()),
            },
            "Wfm:OnChatMessage" => match serde_json::from_value(data.clone()) {
                Ok(message) => Self::ChatMessage(convert::chat_message(message)),
                Err(_) => Self::Changed(DataKind::Chats),
            },
            other => match data_kind(other) {
                Some(kind) => Self::Changed(kind),
                None => Self::Other(other.to_owned()),
            },
        }
    }
}

fn data_kind(event: &str) -> Option<DataKind> {
    Some(match event {
        "User:Update" => DataKind::User,
        "App:Settings:Refresh" => DataKind::Settings,
        "LiveScraper:RefreshStockItems" => DataKind::StockItems,
        "LiveScraper:RefreshStockRivens" => DataKind::StockRivens,
        "LiveScraper:RefreshSyndicateItems" => DataKind::SyndicateItems,
        "LiveScraper:RefreshWishListItems" => DataKind::WishList,
        "LiveScraper:RefreshWfmOrders" | "Wfm:OnDeleteOrders" => DataKind::WfmOrders,
        "Wfm:RefreshAuctions" | "Wfm:OnDeleteAuctions" => DataKind::WfmAuctions,
        "Transaction:RefreshTransactions" => DataKind::Transactions,
        "Cache:Refresh" => DataKind::Cache,
        _ => return None,
    })
}

fn scraper_message(data: &Value) -> Option<LiveScraperMessage> {
    let values = &data["values"];
    let name = || text(values, &["name"]);
    let count = |key: &str| values[key].as_u64().unwrap_or(0) as usize;
    let seconds = || values["seconds"].as_u64().unwrap_or(0);
    Some(match data["i18nKey"].as_str()? {
        "item.checking" => LiveScraperMessage::CheckingItem {
            name: name(),
            current: count("current"),
            total: count("total"),
        },
        "item.deleted" => LiveScraperMessage::DeletingOrders {
            current: count("current"),
            total: count("total"),
        },
        "riven.checking" => LiveScraperMessage::CheckingRiven {
            name: name(),
            mod_name: text(values, &["mod_name"]),
            current: count("current"),
            total: count("total"),
        },
        "riven.cooldown" => LiveScraperMessage::RivenCooldown { seconds: seconds() },
        "riven.rate_limited" => LiveScraperMessage::RateLimited { seconds: seconds() },
        _ => return None,
    })
}
