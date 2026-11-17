use std::{path::PathBuf, sync::Arc};

use qf_app_lib::{
    commands::{
        auction, chat, order, stock_item, stock_riven, trade_entry, transaction, wish_list,
    },
    entity::{
        stock_item::StockItemPaginationQueryDto, stock_riven::StockRivenPaginationQueryDto,
        trade_entry::TradeEntryPaginationQueryDto, transaction::TransactionPaginationQueryDto,
        wish_list::WishListPaginationQueryDto,
    },
    init_headless, Error, HeadlessOptions, Host, State,
};
use serde_json::{json, Value};

struct PrintHost;

impl Host for PrintHost {
    fn emit(&self, event: &str, payload: Value) {
        let text = payload.to_string();
        println!("event {event}: {}", &text[..text.len().min(160)]);
    }
    fn notify(&self, title: &str, body: &str) {
        println!("notify {title}: {body}");
    }
    fn play_sound(&self, file: &str, volume: f32) {
        println!("sound {file} at {volume}");
    }
}

fn all<T: serde::de::DeserializeOwned>() -> T {
    serde_json::from_value(json!({ "page": 1, "limit": -1 })).expect("valid query")
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut opts = HeadlessOptions::from_env();
    if opts.fixtures.is_none() {
        opts.fixtures = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures"));
    }
    init_headless(opts, Arc::new(PrintHost)).await?;

    let counts = [
        (
            "stock items",
            stock_item::get_stock_item_pagination(StockItemPaginationQueryDto::new(1, -1))
                .await?
                .total,
        ),
        (
            "stock rivens",
            stock_riven::get_stock_riven_pagination(StockRivenPaginationQueryDto::new(1, -1))
                .await?
                .total,
        ),
        (
            "wish list",
            wish_list::get_wish_list_pagination(WishListPaginationQueryDto::new(1, -1))
                .await?
                .total,
        ),
        (
            "wf.market orders",
            order::get_wfm_orders_pagination(all(), State::get())?.total,
        ),
        (
            "wf.market auctions",
            auction::get_wfm_auctions_pagination(all(), State::get())
                .await?
                .total,
        ),
        (
            "trade entries",
            trade_entry::get_trade_entry_pagination(TradeEntryPaginationQueryDto::new(1, -1))
                .await?
                .total,
        ),
        (
            "transactions",
            transaction::get_transaction_pagination(TransactionPaginationQueryDto::new(1, -1))
                .await?
                .total,
        ),
        (
            "chats",
            chat::get_chat_pagination(all(), State::get()).await?.total,
        ),
    ];
    for (name, count) in counts {
        println!("{name}: {count}");
    }
    Ok(())
}
