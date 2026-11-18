/// Mirrors `app::types::User` without the tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct User {
    pub anonymous: bool,
    pub wfm_id: String,
    pub wfm_username: String,
    pub wfm_avatar: Option<String>,
    pub wfm_status: UserStatus,
    pub platform: String,
    pub locale: String,
    pub order_limit: i64,
    pub auctions_limit: i64,
    pub unread_messages: i64,
    pub verification: bool,
    pub banned: Option<Ban>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ban {
    pub service: BanService,
    pub until: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BanService {
    WarframeMarket,
    Quantframe,
}

/// Mirrors the `user_status` enum of warframe.market.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UserStatus {
    Online,
    InGame,
    #[default]
    Invisible,
}

impl UserStatus {
    pub fn parse(status: &str) -> Self {
        match status {
            "online" => Self::Online,
            "ingame" | "in_game" => Self::InGame,
            _ => Self::Invisible,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::InGame => "ingame",
            Self::Invisible => "invisible",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Online => "Online",
            Self::InGame => "In game",
            Self::Invisible => "Offline",
        }
    }
}
