#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LiveScraperState {
    pub is_running: bool,
    pub message: Option<LiveScraperMessage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveScraperMessage {
    CheckingItem {
        name: String,
        current: usize,
        total: usize,
    },
    DeletingOrders {
        current: usize,
        total: usize,
    },
    CheckingRiven {
        name: String,
        mod_name: String,
        current: usize,
        total: usize,
    },
    RivenCooldown {
        seconds: u64,
    },
    RateLimited {
        seconds: u64,
    },
}

impl LiveScraperMessage {
    pub fn text(&self) -> String {
        match self {
            Self::CheckingItem {
                name,
                current,
                total,
            } => format!("Checking {name}, {current} of {total}"),
            Self::DeletingOrders { current, total } => {
                format!("Deleting orders, {current} of {total}")
            }
            Self::CheckingRiven {
                name,
                mod_name,
                current,
                total,
            } => format!("Checking {name} {mod_name}, {current} of {total}"),
            Self::RivenCooldown { seconds } => format!("Riven cooldown, {seconds} seconds left"),
            Self::RateLimited { seconds } => {
                format!("Rate limited, waiting {seconds} seconds")
            }
        }
    }

    pub fn progress(&self) -> Option<(usize, usize)> {
        match self {
            Self::CheckingItem { current, total, .. }
            | Self::DeletingOrders { current, total }
            | Self::CheckingRiven { current, total, .. } => Some((*current, *total)),
            Self::RivenCooldown { .. } | Self::RateLimited { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntendedAction {
    Create,
    Update,
    Delete,
}

impl IntendedAction {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Update => "Update",
            Self::Delete => "Delete",
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntendedOrder {
    pub id: u64,
    pub created_at: String,
    pub action: IntendedAction,
    pub is_auction: bool,
    pub market_id: String,
    pub item_name: String,
    pub wfm_url: String,
    pub sub_type: Option<super::SubType>,
    pub order_type: Option<super::OrderType>,
    pub price: Option<i64>,
    pub old_price: Option<i64>,
    pub quantity: Option<i64>,
    pub reason: String,
}
