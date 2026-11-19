mod common;

use common::*;
use qf_app_lib::{
    entity::stock_item::{Model as StockItem, StockItemPaginationQueryDto},
    live_scraper::writes::WriteAction,
    service::StockItemQuery,
    DATABASE,
};

async fn stock_item(wfm_url: &str) -> StockItem {
    let conn = DATABASE.get().unwrap();
    StockItemQuery::get_all(conn, StockItemPaginationQueryDto::new(1, -1))
        .await
        .unwrap()
        .results
        .into_iter()
        .find(|item| item.wfm_url == wfm_url)
        .unwrap_or_else(|| panic!("fixture stock item {wfm_url}"))
}

fn requests(harness: &Harness, method: &str) -> Vec<String> {
    harness
        .sent()
        .into_iter()
        .filter(|request| request.method == method)
        .map(|request| request.url)
        .collect()
}

#[test]
fn max_stock_quantity_deletes_the_live_bid() {
    run(|harness| async move {
        let stock = stock_item("saryn_prime_set").await;
        assert!(stock.owned >= 1);
        write_market("saryn_prime_set", &[90, 95], &[50, 55]);
        set_own_orders(vec![own_order("bid-saryn", "buy", &stock.wfm_id, 40, 1)]);
        let wanted = entry("saryn_prime_set", &stock.wfm_id, &["Buy"], Some(stock.id), 1, 0);
        update_settings(|settings| {
            settings.live_scraper.items.wtb.max_stock_quantity = 1;
            settings.debugging.live_scraper.entries = vec![wanted];
        });

        let _ = scraper().item().check().await;

        let deletes = requests(harness, "DELETE");
        assert_eq!(deletes.len(), 1, "requests: {:?}", harness.sent());
        assert!(deletes[0].ends_with("/order/bid-saryn"), "{deletes:?}");
        assert!(requests(harness, "POST").is_empty());
        assert!(requests(harness, "PATCH").is_empty());
        assert!(
            harness.proxy_lines().iter().any(|line| line.starts_with("CONNECT api.warframe.market")),
            "proxy saw {:?}",
            harness.proxy_lines()
        );
    });
}

const NEKROS: (&str, &str) = ("nekros_prime_set", "57bc9a40e506eb45ea251451");

#[test]
fn dry_run_pass_records_intended_orders_and_sends_no_writes() {
    run(|harness| async move {
        let saryn = stock_item("saryn_prime_set").await;
        let rhino = stock_item("rhino_prime_set").await;
        write_market(NEKROS.0, &[80, 85, 90], &[30, 40, 50]);
        write_market("saryn_prime_set", &[90, 95], &[50, 55]);
        set_item_prices(vec![
            price(NEKROS.0, NEKROS.1, 80.0),
            price("saryn_prime_set", &saryn.wfm_id, 92.0),
        ])
        .await;
        set_own_orders(vec![
            own_order("ask-saryn", "sell", &saryn.wfm_id, 100, 3),
            own_order("bid-rhino", "buy", &rhino.wfm_id, 20, 1),
        ]);
        update_settings(|settings| {
            settings.live_scraper.general.dry_run = true;
            settings.live_scraper.general.auto_delete = true;
            settings.debugging.live_scraper.entries = vec![
                entry(NEKROS.0, NEKROS.1, &["Buy"], None, 1, 0),
                entry("saryn_prime_set", &saryn.wfm_id, &["Sell"], Some(saryn.id), 0, saryn.owned),
            ];
        });
        assert!(intended().is_empty());

        scraper().item().check().await.expect("dry-run pass");

        assert!(harness.writes().is_empty(), "write requests: {:?}", harness.writes());
        assert!(harness.proxy_lines().is_empty(), "left the process: {:?}", harness.proxy_lines());

        let records = intended();
        let find = |action: WriteAction, market_id: &str, wfm_url: &str| {
            records.iter().find(|record| {
                record.action == action
                    && record.market_id == market_id
                    && (wfm_url.is_empty() || record.intent.wfm_url == wfm_url)
            })
        };
        let created = find(WriteAction::Create, "", NEKROS.0).expect("create for Nekros Prime Set");
        assert_eq!(created.intent.price, Some(50));
        assert_eq!(created.intent.order_type.as_deref(), Some("buy"));
        let updated = find(WriteAction::Update, "ask-saryn", "").expect("update for Saryn Prime Set");
        assert_eq!(updated.intent.price, Some(90));
        assert_eq!(updated.intent.old_price, Some(100));
        let deleted = find(WriteAction::Delete, "bid-rhino", "").expect("delete at start");
        assert_eq!(deleted.intent.reason, "Delete at start");
        assert_eq!(deleted.intent.item_name, "Rhino Prime Set");

        let emitted = harness.host.ui_events("LiveScraper:IntendedOrder");
        assert!(emitted.len() >= records.len(), "{} events for {} records", emitted.len(), records.len());
    });
}

#[test]
fn every_scraper_write_goes_through_the_gate() {
    let sources = [
        ("client.rs", include_str!("../src/live_scraper/client.rs")),
        ("modules/helpers.rs", include_str!("../src/live_scraper/modules/helpers.rs")),
        ("modules/item.rs", include_str!("../src/live_scraper/modules/item.rs")),
        ("modules/riven.rs", include_str!("../src/live_scraper/modules/riven.rs")),
    ];
    let calls = [
        ".order().create(",
        ".order().update(",
        ".order().delete(",
        ".order().close(",
        ".auction().create(",
        ".auction().update(",
        ".auction().delete(",
    ];
    for (file, source) in sources {
        let flat: String = source.split_whitespace().collect();
        for call in calls {
            assert!(!flat.contains(call), "{file} calls {call} outside live_scraper::writes");
        }
    }
}
