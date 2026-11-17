//! View models for the screens. Field names follow the backend types in
//! `src-tauri/entity`, `src-tauri/utils` and `src/types/tauri.type.ts`, so a
//! backend-backed data source can fill them without renaming.

#![allow(
    dead_code,
    reason = "the types mirror the backend, and the fixture fills only part of them"
)]

mod financial_report;
mod live_scraper;
mod pagination;
mod price_history;
mod stock_item;
mod stock_status;
mod sub_type;
mod user;

pub use financial_report::FinancialReport;
pub use live_scraper::{LiveScraperMessage, LiveScraperState};
pub use pagination::{Paginated, SortDirection, StockItemListParams};
pub use price_history::PriceHistory;
pub use stock_item::{CoolDownInfo, StockItem, StockItemProperties};
pub use stock_status::StockStatus;
pub use sub_type::SubType;
pub use user::{User, UserStatus};
