#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StockStatus {
    Error,
    Unknown,
    Pending,
    Live,
    ToLowProfit,
    NoSellers,
    NoBuyers,
    InActive,
    SMALimit,
    OrderLimit,
    Overpriced,
    Underpriced,
    MaxPriceDrop,
    InsufficientStanding,
}

impl StockStatus {
    pub const ALL: [Self; 14] = [
        Self::Error,
        Self::Unknown,
        Self::Pending,
        Self::Live,
        Self::ToLowProfit,
        Self::NoSellers,
        Self::NoBuyers,
        Self::InActive,
        Self::SMALimit,
        Self::OrderLimit,
        Self::Overpriced,
        Self::Underpriced,
        Self::MaxPriceDrop,
        Self::InsufficientStanding,
    ];

    pub fn parse(name: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == name)
            .unwrap_or(Self::Unknown)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Unknown => "unknown",
            Self::Pending => "pending",
            Self::Live => "live",
            Self::ToLowProfit => "to_low_profit",
            Self::NoSellers => "no_sellers",
            Self::NoBuyers => "no_buyers",
            Self::InActive => "inactive",
            Self::SMALimit => "sma_limit",
            Self::OrderLimit => "order_limit",
            Self::Overpriced => "overpriced",
            Self::Underpriced => "underpriced",
            Self::MaxPriceDrop => "max_price_drop",
            Self::InsufficientStanding => "insufficient_standing",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Unknown => "Unknown",
            Self::Pending => "Pending",
            Self::Live => "Live",
            Self::ToLowProfit => "Low profit",
            Self::NoSellers => "No sellers",
            Self::NoBuyers => "No buyers",
            Self::InActive => "Inactive",
            Self::SMALimit => "SMA limit",
            Self::OrderLimit => "Order limit",
            Self::Overpriced => "Overpriced",
            Self::Underpriced => "Underpriced",
            Self::MaxPriceDrop => "Max price drop",
            Self::InsufficientStanding => "Insufficient standing",
        }
    }

    pub const fn detail(self) -> &'static str {
        match self {
            Self::Error => "The last check failed",
            Self::Unknown => "The backend sent a status this app doesn't know",
            Self::Pending => "Waiting for the live scraper",
            Self::Live => "Live on Warframe Market",
            Self::ToLowProfit => "The profit is below your minimum",
            Self::NoSellers => "Warframe Market has no sellers for this item",
            Self::NoBuyers => "Warframe Market has no buyers for this item",
            Self::InActive => "The live scraper skips inactive items",
            Self::SMALimit => "The moving average limits the price",
            Self::OrderLimit => "You have reached your order limit",
            Self::Overpriced => "The list price is above the market",
            Self::Underpriced => "The list price is below the market",
            Self::MaxPriceDrop => "The maximum price drop holds the price",
            Self::InsufficientStanding => "You don't have enough standing to buy this item",
        }
    }
}
