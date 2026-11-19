#[derive(Clone, Debug, Default, PartialEq)]
pub struct GdprAccount {
    pub oid: String,
    pub email: String,
    pub display_name: String,
    pub activated: bool,
    pub subscribed_to_emails: bool,
    pub signup_language: String,
    pub signup_country_code: String,
    pub country_code: String,
    pub signup_page: String,
    pub language: String,
    pub ips: Vec<String>,
    pub created_at: Option<String>,
    pub last_login_at: Option<String>,
    pub trades: Vec<GdprTrade>,
    pub logins: Vec<GdprLogin>,
    pub purchases: Vec<GdprPurchase>,
    pub payments: Vec<GdprPayment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GdprTradeKind {
    Sale,
    Purchase,
    Swap,
}

impl GdprTradeKind {
    pub const ALL: [Self; 3] = [Self::Sale, Self::Purchase, Self::Swap];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Sale => "sale",
            Self::Purchase => "purchase",
            Self::Swap => "swap",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Sale => "Sale",
            Self::Purchase => "Purchase",
            Self::Swap => "Swap",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdprItem {
    pub name: String,
    pub quantity: i64,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdprTrade {
    pub time: String,
    pub kind: GdprTradeKind,
    pub platinum: i64,
    pub credits: i64,
    pub gave: Vec<GdprItem>,
    pub received: Vec<GdprItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdprLogin {
    pub time: String,
    pub ip: Option<String>,
    pub client: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdprPurchase {
    pub time: String,
    pub shop_id: String,
    pub platinum: i64,
    pub items: Vec<GdprItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GdprPayment {
    pub time: String,
    pub sku: String,
    pub price: f64,
    pub currency: String,
    pub vendor: String,
    pub account: String,
}
