use super::SubType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OrderType {
    Buy,
    Sell,
}

impl OrderType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Buy => "Buy",
            Self::Sell => "Sell",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WfmOrder {
    pub id: String,
    pub order_type: OrderType,
    pub item_id: String,
    pub item_name: String,
    pub item_url: String,
    pub sub_type: Option<SubType>,
    pub platinum: i64,
    pub quantity: i64,
    pub per_trade: Option<i64>,
    pub visible: bool,
    pub created_at: String,
    pub updated_at: String,
    pub potential_profit: Option<i64>,
    pub closed_avg: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrderTotals {
    pub count: usize,
    pub platinum: i64,
    pub potential_profit: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WfmAuction {
    pub id: String,
    pub uuid: String,
    pub item_type: String,
    pub weapon_url: String,
    pub name: String,
    pub mod_name: Option<String>,
    pub mastery_level: Option<i64>,
    pub mod_rank: Option<i64>,
    pub re_rolls: Option<i64>,
    pub polarity: Option<String>,
    pub attributes: Vec<super::RivenAttribute>,
    pub starting_price: i64,
    pub buyout_price: Option<i64>,
    pub top_bid: Option<i64>,
    pub is_direct_sell: bool,
    pub visible: bool,
    pub closed: bool,
    pub note: String,
    pub created: String,
    pub updated: String,
    pub can_import: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AuctionOverview {
    pub count: i64,
    pub revenue: i64,
    pub potential_profit: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chat {
    pub id: String,
    pub chat_name: String,
    pub chat_with: Vec<ChatUser>,
    pub unread_count: i64,
    pub last_update: String,
    pub last_message: Option<ChatMessage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatUser {
    pub id: String,
    pub name: String,
    pub reputation: f64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: String,
    pub chat_id: String,
    pub message: String,
    pub send_date: String,
    pub message_from: String,
}
