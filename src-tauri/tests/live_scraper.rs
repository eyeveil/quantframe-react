mod common;

use common::*;
use qf_app_lib::{
    entity::stock_item::{Model as StockItem, StockItemPaginationQueryDto},
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
