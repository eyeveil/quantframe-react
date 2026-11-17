/// The fields of `app::types::User` that the shell shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub wfm_id: String,
    pub wfm_username: String,
    pub wfm_status: UserStatus,
    pub platform: String,
    pub region: String,
    pub order_limit: i64,
    pub auctions_limit: i64,
    pub unread_messages: i64,
    pub verification: bool,
}

/// Mirrors the `user_status` enum of warframe.market.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserStatus {
    Online,
    InGame,
    Invisible,
}

impl UserStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Online => "Online",
            Self::InGame => "In game",
            Self::Invisible => "Offline",
        }
    }
}
