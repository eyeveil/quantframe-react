//! View models for the screens. Field names follow the backend types in
//! `src-tauri/entity`, `src-tauri/utils` and `wf_market`, and `data/convert.rs`
//! fills them.

mod analytics;
mod app;
mod cache;
mod dashboard;
mod financial_report;
mod inventory;
mod live_scraper;
mod pagination;
mod price_history;
mod stock_item;
mod stock_riven;
mod stock_status;
mod sub_type;
mod syndicate_item;
mod trade;
mod user;
mod wfm;
mod wish_list;

pub use analytics::{ItemPrice, PriceLookupParams, RivenPrice, SyndicateItemPrice, UserActivity};
pub use cache::{RivenStat, RivenWeapon, TradableItem};
pub use app::{AppInfo, EeLogLine, EeLogParams, GdprState, RouteLimiter, WfmDebugState};
pub use dashboard::{BestSeller, CategorySummary, Chart, DashboardSummary, PeriodSummary};
pub use financial_report::FinancialReport;
pub use inventory::{InventoryRiven, InventorySyndicate};
pub use live_scraper::{LiveScraperMessage, LiveScraperState};
pub use pagination::{
    ListParams, OrderListParams, Paginated, SortDirection, StockItemListParams,
    SyndicateItemListParams, TradeEntryListParams, TransactionListParams,
};
pub use price_history::PriceHistory;
pub use stock_item::{CoolDownInfo, NewStockItem, StockItem, StockItemProperties, StockItemUpdate};
pub use stock_riven::{
    MinMax, NewStockRiven, RivenAttribute, RivenFilter, RivenFilterAttribute, StockRiven,
    StockRivenProperties, StockRivenUpdate,
};
pub use stock_status::StockStatus;
pub use sub_type::SubType;
pub use syndicate_item::{
    NewSyndicateItem, SyndicateItem, SyndicateItemProperties, SyndicateItemUpdate,
};
pub use trade::{
    NewTradeEntry, TradeEntry, TradeEntryGroup, TradeEntryProperties, TradeEntryUpdate,
    Transaction, TransactionItemType, TransactionType, TransactionUpdate, tag_label,
};
pub use user::{Ban, BanService, User, UserStatus};
pub use wfm::{
    AuctionOverview, Chat, ChatMessage, ChatUser, OrderTotals, OrderType, WfmAuction, WfmOrder,
};
pub use wish_list::{WishListItem, WishListProperties, WishListUpdate};

/// The full settings tree, edited in place and saved with `app_update_settings`.
pub type Settings = qf_app_lib::app::Settings;
