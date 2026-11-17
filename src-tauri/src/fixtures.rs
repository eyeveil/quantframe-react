use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use entity::{stock_item, stock_riven, trade_entry, transaction, wish_list};
use serde::Deserialize;
use service::{
    StockItemMutation, StockRivenMutation, TradeEntryMutation, TransactionMutation,
    WishListMutation,
};
use utils::{get_location, info, read_json_file, Error, LoggerOptions};
use wf_market::types::{Auction, AuctionList, ChatList, Order, OrderList};

use crate::{app::AppState, cache::CacheState, helper, host, utils::OrderListExt, DATABASE};

#[derive(Deserialize)]
struct Database {
    stock_items: Vec<stock_item::Model>,
    stock_rivens: Vec<stock_riven::Model>,
    wish_list: Vec<wish_list::Model>,
    transactions: Vec<transaction::Model>,
    trade_entries: Vec<trade_entry::Model>,
}

fn io_error(path: &Path, message: &str, e: std::io::Error) -> Error {
    Error::from_io("Fixtures", &path.to_path_buf(), message, e, get_location!())
}

pub fn temp_storage_dir() -> Result<PathBuf, Error> {
    let dir = std::env::temp_dir().join(format!("quantframe-fixtures-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| io_error(&dir, "Failed to clear", e))?;
    }
    std::fs::create_dir_all(&dir).map_err(|e| io_error(&dir, "Failed to create", e))?;
    Ok(dir)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(to).map_err(|e| io_error(to, "Failed to create", e))?;
    for entry in std::fs::read_dir(from).map_err(|e| io_error(from, "Failed to read", e))? {
        let path = entry
            .map_err(|e| io_error(from, "Failed to read", e))?
            .path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            std::fs::copy(&path, &target).map_err(|e| io_error(&path, "Failed to copy", e))?;
        }
    }
    Ok(())
}

/// Puts the fixture cache and user into the storage directory before startup reads them.
pub fn prepare_storage(fixtures: &Path) -> Result<(), Error> {
    let storage = helper::get_app_storage_path();
    copy_dir(&fixtures.join("cache"), &storage.join("cache"))?;
    let user = fixtures.join("user.json");
    std::fs::copy(&user, storage.join("auth.json"))
        .map_err(|e| io_error(&user, "Failed to copy", e))?;
    Ok(())
}

/// Fills the database and the warframe.market caches. Runs after `setup_manages`.
pub async fn seed(fixtures: &Path) -> Result<(), Error> {
    let conn = DATABASE.get().expect("Database not initialized");
    let db: Database = read_json_file(&fixtures.join("database.json"))?;
    let counts = format!(
        "stock items {}, stock rivens {}, wish list {}, transactions {}, trade entries {}",
        db.stock_items.len(),
        db.stock_rivens.len(),
        db.wish_list.len(),
        db.transactions.len(),
        db.trade_entries.len()
    );
    for model in db.stock_items {
        StockItemMutation::create(conn, model).await?;
    }
    for model in db.stock_rivens {
        StockRivenMutation::create(conn, model).await?;
    }
    for model in &db.wish_list {
        WishListMutation::create(conn, model).await?;
    }
    for model in &db.transactions {
        TransactionMutation::create(conn, model, false).await?;
    }
    for model in &db.trade_entries {
        TradeEntryMutation::create(conn, model).await?;
    }

    let mut orders: OrderList<Order> = read_json_file(&fixtures.join("wfm_orders.json"))?;
    let auctions: AuctionList<Auction> = read_json_file(&fixtures.join("wfm_auctions.json"))?;
    let chats: ChatList = read_json_file(&fixtures.join("wfm_chats.json"))?;
    let wfm_counts = format!(
        "orders {}, auctions {}, chats {}",
        orders.sell_orders.len() + orders.buy_orders.len(),
        auctions.auctions.len(),
        chats.chats.len()
    );
    let cache = host::managed::<Mutex<CacheState>>()?.lock()?.clone();
    orders.apply_item_info(&cache)?;
    let app = host::managed::<Mutex<AppState>>()?.lock()?;
    app.wfm_client.order().set_orders(orders);
    *app.wfm_client.auction().cache_auctions_mut() = auctions;
    *app.wfm_client.chat().cache_chats_mut() = chats;

    info(
        "Fixtures:Seed",
        format!("Seeded {counts}, {wfm_counts} from {}", fixtures.display()),
        &LoggerOptions::default(),
    );
    Ok(())
}
