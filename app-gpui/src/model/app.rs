//! App-level state: startup info, the Warframe log parser and the debug screen.

/// The payload of `app_get_app_info`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: String,
    pub is_dev: bool,
    pub use_temp_db: bool,
    pub is_pre_release: bool,
    pub tos_uuid: String,
    /// The backend runs on `src-tauri/fixtures` and makes no network requests.
    pub fixture_mode: bool,
}

/// The payload of `debug_get_wfm_state`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WfmDebugState {
    pub sell_orders: usize,
    pub buy_orders: usize,
    pub auctions: usize,
    pub order_limit: i64,
    /// Requests per warframe.market route this session.
    pub tracking: Vec<(String, u64)>,
    pub limiters: Vec<RouteLimiter>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteLimiter {
    pub route: String,
    pub limit: i64,
    pub wait_time_sec: f64,
    pub quota_type: String,
}

/// One line of `debug_get_ee_logs`, from Warframe's `EE.log`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EeLogLine {
    pub index: usize,
    pub line: String,
    pub date: i64,
}

/// Query of `debug_get_ee_logs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EeLogParams {
    pub page: i64,
    pub limit: i64,
    pub query: Option<String>,
    pub hide_empty: bool,
}

impl Default for EeLogParams {
    fn default() -> Self {
        Self {
            page: 1,
            limit: 100,
            query: None,
            hide_empty: true,
        }
    }
}

/// The payload of `wfgdpr_get_state`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GdprState {
    pub was_initialized: bool,
}
