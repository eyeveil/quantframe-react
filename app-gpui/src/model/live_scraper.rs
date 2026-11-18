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
