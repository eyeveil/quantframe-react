use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use chrono::{DateTime, Utc};
use entity::{stock_item, stock_riven, syndicate_item, trade_entry, transaction, wish_list};
use serde::Deserialize;
use service::{
    sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set},
    StockItemMutation, StockRivenMutation, SyndicateItemMutation, TradeEntryMutation,
    TransactionMutation, WishListMutation,
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
    syndicate_items: Vec<syndicate_item::Model>,
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

const FIXTURE_MARKER: &str = ".quantframe-fixtures";

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
    let marker = storage.join(FIXTURE_MARKER);
    let has_data = std::fs::read_dir(&storage)
        .map(|entries| entries.flatten().any(|entry| entry.file_name() != "logs"))
        .unwrap_or(false);
    if has_data && !marker.exists() {
        return Err(Error::new(
            "Fixtures:PrepareStorage",
            format!(
                "{} holds data from a real run. Fixture mode needs an empty directory or one it created.",
                storage.display()
            ),
            get_location!(),
        ));
    }
    std::fs::create_dir_all(&storage).map_err(|e| io_error(&storage, "Failed to create", e))?;
    std::fs::write(&marker, "").map_err(|e| io_error(&marker, "Failed to write", e))?;
    copy_dir(&fixtures.join("cache"), &storage.join("cache"))?;
    let user = fixtures.join("user.json");
    std::fs::copy(&user, storage.join("auth.json"))
        .map_err(|e| io_error(&user, "Failed to copy", e))?;
    Ok(())
}

/// Fills the database and the warframe.market caches. Runs after `setup_manages`.
/// A database that already has stock items, from an earlier run on the same
/// `QF_STORAGE_DIR`, keeps its rows, so edits survive a restart.
pub async fn seed(fixtures: &Path) -> Result<(), Error> {
    let conn = DATABASE.get().expect("Database not initialized");
    let existing = stock_item::Entity::find().count(conn).await.unwrap_or(0);
    if existing == 0 {
        seed_database(fixtures).await?;
    } else {
        info(
            "Fixtures:Seed",
            format!("Kept the {existing} stock items already in the database"),
            &LoggerOptions::default(),
        );
    }
    seed_market(fixtures).await
}

/// How far to move a list of fixture dates so the latest lands on `now`.
fn shift_to(now: DateTime<Utc>, dates: impl Iterator<Item = DateTime<Utc>>) -> chrono::Duration {
    dates
        .max()
        .map_or(chrono::Duration::zero(), |latest| now - latest)
}

async fn seed_database(fixtures: &Path) -> Result<(), Error> {
    let conn = DATABASE.get().expect("Database not initialized");
    let mut db: Database = read_json_file(&fixtures.join("database.json"))?;
    // The fixture dates are fixed, so the transactions and trade entries move
    // to end at the seed time, and the dashboard's "today" and "last 7 days"
    // charts have data on any day.
    let now = Utc::now();
    let shift = shift_to(now, db.transactions.iter().map(|model| model.created_at));
    for model in &mut db.transactions {
        model.created_at += shift;
        model.updated_at += shift;
    }
    let trade_shift = shift_to(now, db.trade_entries.iter().map(|model| model.created_at));
    let counts = format!(
        "stock items {}, stock rivens {}, wish list {}, transactions {}, trade entries {}, syndicate items {}",
        db.stock_items.len(),
        db.stock_rivens.len(),
        db.wish_list.len(),
        db.transactions.len(),
        db.trade_entries.len(),
        db.syndicate_items.len()
    );
    for model in db.stock_items {
        StockItemMutation::create(conn, model).await?;
    }
    for model in db.stock_rivens {
        StockRivenMutation::create(conn, model).await?;
    }
    for model in &db.wish_list {
        let created = WishListMutation::create(conn, model).await?;
        // `create` starts every wish as pending with no bid.
        let update = wish_list::UpdateWishList::new(created.id)
            .with_list_price(model.list_price)
            .with_status(model.status.clone())
            .with_price_history(model.price_history.0.clone());
        WishListMutation::update_by_id(conn, update).await?;
    }
    for model in &db.transactions {
        TransactionMutation::create(conn, model, false).await?;
    }
    for model in &db.trade_entries {
        let created = TradeEntryMutation::create(conn, model).await?;
        // `create` stamps the entry with the current time.
        trade_entry::ActiveModel {
            id: Set(created.id),
            created_at: Set(model.created_at + trade_shift),
            updated_at: Set(model.updated_at + trade_shift),
            ..Default::default()
        }
        .update(conn)
        .await
        .map_err(|e| {
            Error::new(
                "Fixtures:Seed",
                format!("Failed to date a trade entry: {e}"),
                get_location!(),
            )
        })?;
    }
    for model in db.syndicate_items {
        SyndicateItemMutation::create(conn, model).await?;
    }
    info(
        "Fixtures:Seed",
        format!("Seeded {counts} from {}", fixtures.display()),
        &LoggerOptions::default(),
    );
    Ok(())
}

async fn seed_market(fixtures: &Path) -> Result<(), Error> {
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
        format!("Seeded {wfm_counts} from {}", fixtures.display()),
        &LoggerOptions::default(),
    );
    Ok(())
}
