//! Where the views read their data. Method names follow the Tauri commands in
//! `src-tauri/src/commands`, which a backend-backed source will call.

mod fixture;

use std::collections::BTreeMap;

pub use fixture::FixtureSource;

use crate::model::{
    FinancialReport, LiveScraperState, Paginated, StockItem, StockItemListParams, StockStatus, User,
};

pub trait DataSource {
    fn get_stock_item_pagination(&self, params: &StockItemListParams) -> Paginated<StockItem>;
    fn get_stock_item_status_counts(&self) -> BTreeMap<StockStatus, usize>;
    fn get_stock_item_financial_report(&self) -> FinancialReport;
    fn live_scraper_get_state(&self) -> LiveScraperState;
    fn user(&self) -> User;
}
