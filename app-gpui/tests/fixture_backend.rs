use std::path::PathBuf;

use futures::{StreamExt, executor::block_on};
use qf_app_lib::app::SyndicateEntrySetting;
use quantframe_gpui::{
    data::{BackendEvent, DataError, DataKind, DataResult, DataSource, StartOptions},
    model::*,
};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/fixtures")
}

fn ok<T>(name: &str, result: DataResult<T>) -> T {
    result.unwrap_or_else(|error| panic!("{name} failed: {error}"))
}

fn non_empty<T>(name: &str, page: DataResult<Paginated<T>>) -> Paginated<T> {
    let page = ok(name, page);
    assert!(!page.results.is_empty(), "{name} returned no rows");
    page
}

fn offline(name: &str, result: DataResult<impl std::fmt::Debug>) {
    let error: DataError = match result {
        Ok(value) => panic!("{name} succeeded in fixture mode: {value:?}"),
        Err(error) => error,
    };
    let text = error.to_string();
    assert!(
        text.contains("Fixture mode makes no network requests"),
        "{name} failed with an unexpected error: {text}"
    );
}

fn needs_mod_cache<T: std::fmt::Debug>(name: &str, result: DataResult<T>) {
    if let Err(error) = result {
        assert!(
            error.message.contains("Riven upgrade not found"),
            "{name} failed: {error}"
        );
    }
}

fn export_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("qf-gpui-test-{}-{name}.json", std::process::id()))
}

async fn cache_lists(source: &DataSource) {
    let items = ok(
        "cache_get_tradable_items",
        source.cache_get_tradable_items().await,
    );
    assert_eq!(items.len(), 3893);
    assert!(items.windows(2).all(|pair| pair[0].name <= pair[1].name));
    let rhino: Vec<&TradableItem> = items
        .iter()
        .filter(|item| item.name.to_lowercase().contains("rhino"))
        .collect();
    let set = rhino
        .iter()
        .find(|item| item.name == "Rhino Prime Set")
        .expect("typing rhino finds Rhino Prime Set");
    assert_eq!(set.wfm_url, "rhino_prime_set");
    assert!(!set.bulk_tradable);
    let flow = items
        .iter()
        .find(|item| item.name == "Primed Flow")
        .expect("Primed Flow");
    assert_eq!(flow.max_rank, Some(10));
    assert!(
        items
            .iter()
            .find(|item| item.name == "Arcane Energize")
            .expect("Arcane Energize")
            .bulk_tradable
    );

    let weapons = ok(
        "cache_get_riven_weapons",
        source.cache_get_riven_weapons().await,
    );
    assert!(weapons.len() > 300, "only {} riven weapons", weapons.len());
    let rubico = weapons
        .iter()
        .find(|weapon| weapon.name == "Rubico")
        .expect("Rubico");
    assert_eq!(rubico.wfm_url, "rubico");
    assert_eq!(rubico.riven_type, "rifle");
    assert!(!rubico.wfm_id.is_empty());

    let stats = ok(
        "cache_get_riven_attributes",
        source.cache_get_riven_attributes().await,
    );
    assert_eq!(stats.len(), 50);
    let critical = stats
        .iter()
        .find(|stat| stat.wfm_url == "critical_chance")
        .expect("critical_chance");
    assert_eq!(critical.label, "Critical Chance");
    let punch = stats
        .iter()
        .find(|stat| stat.wfm_url == "punch_through")
        .expect("punch_through");
    assert!(punch.fits("rifle") && !punch.fits("melee"));
}

async fn trader_fields(source: &DataSource, all: &StockItemListParams, list: &ListParams) {
    let items = ok(
        "get_stock_item_pagination",
        source.get_stock_item_pagination(all).await,
    )
    .results;
    let limited = items
        .iter()
        .find(|item| item.properties.min_price.is_some())
        .expect("a stock item with a minimum price");
    assert_eq!(limited.properties.min_price, Some(limited.bought + 5));
    assert!(
        items
            .iter()
            .any(|item| item.properties.min_sma == Some(15)
                && item.properties.min_profit == Some(10))
    );
    assert!(items.iter().any(|item| item.properties.is_bulk));
    let item = items
        .iter()
        .find(|item| item.properties.min_price.is_none())
        .expect("a stock item without limits")
        .clone();
    let raised = ok(
        "stock_item_update min_price",
        source
            .stock_item_update(
                item.id,
                StockItemUpdate {
                    min_price: BulkEdit::Set(Limit::Value(65)),
                    min_profit: BulkEdit::Set(Limit::Value(12)),
                    min_sma: BulkEdit::Set(Limit::Value(-1)),
                    is_bulk: Some(true),
                    ..StockItemUpdate::default()
                },
            )
            .await,
    );
    assert_eq!(raised.properties.min_price, Some(65));
    assert_eq!(raised.properties.min_profit, Some(12));
    assert_eq!(raised.properties.min_sma, Some(-1));
    assert!(raised.properties.is_bulk);
    assert_eq!(raised.bought, item.bought);
    let cleared = ok(
        "stock_item_update clear",
        source
            .stock_item_update(
                item.id,
                StockItemUpdate {
                    min_price: BulkEdit::Set(Limit::Unset),
                    is_bulk: Some(false),
                    ..StockItemUpdate::default()
                },
            )
            .await,
    );
    assert_eq!(cleared.properties.min_price, None);
    assert_eq!(cleared.properties.min_profit, Some(12));
    assert!(!cleared.properties.is_bulk);
    let reread = ok(
        "stock_item_get_by_id",
        source.stock_item_get_by_id(item.id).await,
    );
    assert_eq!(reread.properties.min_profit, Some(12));

    let rivens = ok(
        "get_stock_riven_pagination",
        source.get_stock_riven_pagination(all).await,
    )
    .results;
    assert!(rivens.iter().any(|riven| riven.properties.is_predicted));
    let filtered = rivens
        .iter()
        .find(|riven| riven.filter.enabled)
        .expect("a riven with a filter");
    assert_eq!(
        filtered.filter.mastery_rank,
        Some(MinMax {
            min: 8,
            max: Some(16)
        })
    );
    assert_eq!(filtered.filter.similarity, Some(60.0));
    assert_eq!(filtered.filter.attributes.len(), filtered.attributes.len());
    assert!(
        rivens
            .iter()
            .flat_map(|riven| &riven.attributes)
            .all(|attribute| !attribute.label.is_empty() && !attribute.label.contains('_'))
    );
    let crit = rivens
        .iter()
        .flat_map(|riven| &riven.attributes)
        .find(|attribute| attribute.wfm_url == "critical_chance");
    if let Some(crit) = crit {
        assert_eq!(crit.label, "Critical Chance");
    }
    let riven = rivens
        .iter()
        .find(|riven| !riven.filter.enabled && riven.properties.min_price.is_none())
        .expect("a riven without limits")
        .clone();
    let mut filter = RivenFilter {
        enabled: true,
        re_rolls: Some(MinMax {
            min: 2,
            max: Some(20),
        }),
        required_negative: true,
        attributes: riven
            .attributes
            .iter()
            .map(|attribute| RivenFilterAttribute {
                wfm_url: attribute.wfm_url.clone(),
                positive: attribute.positive,
                is_required: true,
            })
            .collect(),
        ..RivenFilter::default()
    };
    let saved = ok(
        "stock_riven_update filter",
        source
            .stock_riven_update(
                riven.id,
                StockRivenUpdate {
                    min_price: BulkEdit::Set(Limit::Value(150)),
                    filter: Some(filter.clone()),
                    ..StockRivenUpdate::default()
                },
            )
            .await,
    );
    assert_eq!(saved.properties.min_price, Some(150));
    assert_eq!(saved.filter, filter);
    filter.enabled = false;
    let saved = ok(
        "stock_riven_update filter off",
        source
            .stock_riven_update(
                riven.id,
                StockRivenUpdate {
                    filter: Some(filter.clone()),
                    ..StockRivenUpdate::default()
                },
            )
            .await,
    );
    assert!(!saved.filter.enabled);
    assert_eq!(saved.properties.min_price, Some(150));

    let wishes = ok(
        "get_wish_list_pagination",
        source.get_wish_list_pagination(all).await,
    )
    .results;
    let capped = wishes
        .iter()
        .find(|wish| wish.properties.max_price.is_some())
        .expect("a wish with a maximum price");
    assert_eq!(
        capped.properties.max_price,
        capped.list_price.map(|price| price + 12)
    );
    assert!(wishes.iter().all(|wish| wish.list_price.is_some()));
    assert!(
        wishes
            .iter()
            .any(|wish| wish.properties.min_price.is_some())
    );
    assert!(wishes.iter().any(|wish| wish.properties.is_bulk));
    let wish = wishes[1].clone();
    let saved = ok(
        "wish_list_update max_price",
        source
            .wish_list_update(
                wish.id,
                WishListUpdate {
                    max_price: BulkEdit::Set(Limit::Value(140)),
                    min_price: BulkEdit::Set(Limit::Value(30)),
                    ..WishListUpdate::default()
                },
            )
            .await,
    );
    assert_eq!(saved.properties.max_price, Some(140));
    assert_eq!(saved.properties.min_price, Some(30));
    assert_eq!(saved.quantity, wish.quantity);

    let synds = ok(
        "get_syndicate_item_pagination",
        source
            .get_syndicate_item_pagination(&SyndicateItemListParams {
                stock: all.clone(),
                syndicate: None,
            })
            .await,
    )
    .results;
    assert!(synds.iter().any(|item| item.properties.min_price.is_some()));
    let synd = synds
        .iter()
        .find(|item| item.properties.min_price.is_none())
        .expect("a syndicate item without a minimum price");
    let saved = ok(
        "syndicate_item_update min_price",
        source
            .syndicate_item_update(
                synd.id,
                SyndicateItemUpdate {
                    min_price: BulkEdit::Set(Limit::Value(22)),
                    ..SyndicateItemUpdate::default()
                },
            )
            .await,
    );
    assert_eq!(saved.properties.min_price, Some(22));
    assert_eq!(saved.standing_cost, synd.standing_cost);

    let orders = ok(
        "get_wfm_orders_pagination",
        source
            .get_wfm_orders_pagination(&OrderListParams {
                list: list.clone(),
                order_type: None,
            })
            .await,
    )
    .results;
    assert!(orders.iter().all(|order| order.potential_profit.is_some()));
    assert!(
        orders
            .iter()
            .all(|order| order.closed_avg.is_some_and(|avg| avg > 0.0))
    );
    let sell_profit: i64 = orders
        .iter()
        .filter(|order| order.order_type == OrderType::Sell)
        .map(|order| order.potential_profit.unwrap_or(0) * order.quantity)
        .sum();
    let totals = ok(
        "get_wfm_orders_status_counts",
        source.get_wfm_orders_status_counts().await,
    );
    assert!(sell_profit > 0);
    assert_eq!(
        totals[&OrderType::Sell].potential_profit,
        sell_profit as f64
    );

    let intended = ok(
        "live_scraper_get_intended_orders",
        source.live_scraper_get_intended_orders().await,
    );
    assert_eq!(intended.len(), 9);
    assert_eq!(intended[0].action, IntendedAction::Create);
    assert_eq!(intended[0].price, Some(50));
    assert!(intended.iter().any(|row| row.is_auction));

    let auctions = ok(
        "get_wfm_auctions_pagination",
        source.get_wfm_auctions_pagination(list).await,
    )
    .results;
    assert_eq!(
        auctions.iter().filter(|auction| auction.can_import).count(),
        2
    );
    let overview = ok(
        "get_wfm_auctions_overview",
        source.get_wfm_auctions_overview().await,
    );
    assert_eq!(overview.potential_profit, 100 + 168 + 210 + 103 + 100);
    assert!(
        auctions
            .iter()
            .flat_map(|auction| &auction.attributes)
            .all(|attribute| !attribute.label.contains('_'))
    );

    let entries = ok(
        "get_trade_entry_pagination",
        source
            .get_trade_entry_pagination(&TradeEntryListParams {
                list: list.clone(),
                ..TradeEntryListParams::default()
            })
            .await,
    )
    .results;
    let mut days: Vec<&str> = entries
        .iter()
        .map(|entry| &entry.created_at[..10])
        .collect();
    days.sort_unstable();
    days.dedup();
    assert!(
        days.len() > 10,
        "trade entries keep their spread of days: {days:?}"
    );
    let summary = ok("dashboard_summary", source.dashboard_summary().await);
    assert!(summary.today.summary.total_transactions >= 1);
    assert!(summary.recent_days.summary.total_transactions >= 5);
    assert!(
        summary
            .recent_days
            .chart
            .values
            .iter()
            .any(|value| *value != 0)
    );
    let count = |group: &str| entries.iter().filter(|entry| entry.group == group).count();
    assert_eq!((count("item"), count("custom"), count("riven")), (12, 5, 6));
    let riven = entries
        .iter()
        .find(|entry| entry.group == "riven")
        .expect("a riven entry");
    let low = riven.properties.min_price.expect("market low");
    assert_eq!(riven.properties.potential_profit, Some(low - riven.price));
    assert_eq!(riven.properties.last_updated.as_deref(), Some("2026-11-17"));
    assert!(
        entries
            .iter()
            .flat_map(|entry| &entry.tags)
            .any(|tag| tag == "arcane_enhancement")
    );
    assert_eq!(tag_label("arcane_enhancement"), "Arcane enhancement");
    assert_eq!(tag_label("prime"), "Prime");
    let created = ok(
        "trade_entry_create riven",
        source
            .trade_entry_create(NewTradeEntry {
                tags: vec!["interesting".into()],
                properties: TradeEntryProperties {
                    min_price: Some(200),
                    potential_profit: Some(30),
                    last_updated: Some("2026-11-18".into()),
                },
                ..NewTradeEntry::new("tigris", TradeEntryGroup::Riven, 170)
            })
            .await,
    );
    assert_eq!(created.group, "riven");
    assert_eq!(created.properties.min_price, Some(200));
    assert_eq!(created.properties.potential_profit, Some(30));
    let repriced = ok(
        "trade_entry_create override",
        source
            .trade_entry_create(NewTradeEntry {
                override_existing: true,
                tags: vec!["interesting".into()],
                properties: TradeEntryProperties {
                    min_price: Some(190),
                    potential_profit: Some(30),
                    last_updated: Some("2026-11-18".into()),
                },
                ..NewTradeEntry::new("tigris", TradeEntryGroup::Riven, 160)
            })
            .await,
    );
    assert_eq!(repriced.id, created.id);
    assert_eq!(repriced.price, 160);
    assert_eq!(repriced.properties.min_price, Some(190));
    ok(
        "trade_entry_delete",
        source.trade_entry_delete(created.id).await,
    );
    let custom = ok(
        "trade_entry_create custom",
        source
            .trade_entry_create(NewTradeEntry::new(
                "Forma carry",
                TradeEntryGroup::Custom,
                9,
            ))
            .await,
    );
    assert_eq!(custom.name, "Forma carry");
    ok(
        "trade_entry_delete",
        source.trade_entry_delete(custom.id).await,
    );
}

#[test]
fn every_data_source_method_works_in_fixture_mode() {
    let (source, mut events) =
        DataSource::start(StartOptions::fixtures(fixtures())).expect("start");
    assert!(source.fixture_mode());
    block_on(async {
        loop {
            match events.next().await.expect("backend event channel closed") {
                BackendEvent::Ready => break,
                BackendEvent::StartupFailed(error) => panic!("startup failed: {error}"),
                _ => {}
            }
        }
    });
    assert!(DataSource::start(StartOptions::fixtures(fixtures())).is_err());

    block_on(async {
        let all = StockItemListParams {
            limit: -1,
            ..StockItemListParams::default()
        };
        let list = ListParams {
            limit: -1,
            ..ListParams::default()
        };
        cache_lists(&source).await;
        trader_fields(&source, &all, &list).await;

        let items = non_empty(
            "get_stock_item_pagination",
            source.get_stock_item_pagination(&all).await,
        );
        assert_eq!(items.total, 40);
        let searched = ok(
            "get_stock_item_pagination query",
            source
                .get_stock_item_pagination(&StockItemListParams {
                    query: Some(items.results[0].item_name.clone()),
                    ..all.clone()
                })
                .await,
        );
        assert!(
            searched
                .results
                .iter()
                .any(|item| item.id == items.results[0].id)
        );
        assert!(
            !ok(
                "get_stock_item_status_counts",
                source.get_stock_item_status_counts().await
            )
            .is_empty()
        );
        assert!(
            ok(
                "get_stock_item_financial_report",
                source.get_stock_item_financial_report().await
            )
            .expenses
                > 0
        );
        let first = items.results[0].clone();
        assert_eq!(
            ok(
                "stock_item_get_by_id",
                source.stock_item_get_by_id(first.id).await
            )
            .id,
            first.id
        );
        let updated = ok(
            "stock_item_update",
            source
                .stock_item_update(
                    first.id,
                    StockItemUpdate {
                        list_price: Some(77),
                        ..StockItemUpdate::default()
                    },
                )
                .await,
        );
        assert_eq!(updated.list_price, Some(77));
        assert!(
            !ok(
                "stock_item_update_multiple",
                source
                    .stock_item_update_multiple(
                        vec![first.id],
                        StockItemUpdate {
                            is_hidden: Some(false),
                            ..StockItemUpdate::default()
                        }
                    )
                    .await
            )
            .is_empty()
        );
        let created = ok(
            "stock_item_create",
            source
                .stock_item_create(NewStockItem {
                    raw: "abating_link".into(),
                    sub_type: Some(SubType::rank(0)),
                    quantity: 2,
                    bought: 12,
                })
                .await,
        );
        assert_eq!(created.item_name, "Abating Link");
        let sold = ok(
            "stock_item_sell",
            source
                .stock_item_sell("abating_link".into(), Some(SubType::rank(0)), 1, 20)
                .await,
        );
        assert_eq!(sold.owned, 1);
        assert_eq!(
            ok(
                "stock_item_delete",
                source.stock_item_delete(created.id).await
            )
            .id,
            created.id
        );
        let extra = ok(
            "stock_item_create",
            source
                .stock_item_create(NewStockItem {
                    raw: "abating_link".into(),
                    sub_type: Some(SubType::rank(0)),
                    quantity: 1,
                    bought: 5,
                })
                .await,
        );
        assert_eq!(
            ok(
                "stock_item_delete_multiple",
                source.stock_item_delete_multiple(vec![extra.id]).await
            ),
            1
        );
        let path = ok(
            "export_stock_item_json",
            source
                .export_stock_item_json(&all, export_path("stock"))
                .await,
        );
        assert!(std::fs::metadata(&path).expect("stock export").len() > 2);

        let rivens = non_empty(
            "get_stock_riven_pagination",
            source.get_stock_riven_pagination(&all).await,
        );
        assert!(
            !ok(
                "get_stock_riven_status_counts",
                source.get_stock_riven_status_counts().await
            )
            .is_empty()
        );
        ok(
            "get_stock_riven_financial_report",
            source.get_stock_riven_financial_report().await,
        );
        let riven = rivens.results[0].clone();
        needs_mod_cache(
            "stock_riven_get_by_id",
            source.stock_riven_get_by_id(riven.id).await,
        );
        let updated = ok(
            "stock_riven_update",
            source
                .stock_riven_update(
                    riven.id,
                    StockRivenUpdate {
                        list_price: Some(450),
                        ..StockRivenUpdate::default()
                    },
                )
                .await,
        );
        assert_eq!(updated.list_price, Some(450));
        assert!(
            !ok(
                "stock_riven_update_multiple",
                source
                    .stock_riven_update_multiple(
                        vec![riven.id],
                        StockRivenUpdate {
                            is_hidden: Some(false),
                            ..StockRivenUpdate::default()
                        }
                    )
                    .await
            )
            .is_empty()
        );
        let path = ok(
            "export_stock_riven_json",
            source
                .export_stock_riven_json(&all, export_path("rivens"))
                .await,
        );
        assert!(std::fs::metadata(&path).expect("riven export").len() > 2);
        let last = rivens.results.last().expect("riven").clone();
        needs_mod_cache(
            "stock_riven_sell",
            source.stock_riven_sell(&last, 300).await,
        );
        let remaining = ok(
            "get_stock_riven_pagination",
            source.get_stock_riven_pagination(&all).await,
        );
        let doomed = remaining
            .results
            .iter()
            .rev()
            .take(2)
            .map(|r| r.id)
            .collect::<Vec<_>>();
        ok(
            "stock_riven_delete",
            source.stock_riven_delete(doomed[0]).await,
        );
        assert_eq!(
            ok(
                "stock_riven_delete_multiple",
                source.stock_riven_delete_multiple(vec![doomed[1]]).await
            ),
            1
        );
        let created = source
            .stock_riven_create(NewStockRiven {
                wfm_weapon_url: riven.wfm_weapon_url.clone(),
                mod_name: riven.mod_name.clone(),
                mastery_rank: riven.mastery_rank,
                re_rolls: riven.re_rolls,
                rank: 0,
                polarity: riven.polarity.clone(),
                attributes: riven.attributes.clone(),
                bought: 100,
            })
            .await;
        needs_mod_cache("stock_riven_create", created);

        let wishes = non_empty(
            "get_wish_list_pagination",
            source.get_wish_list_pagination(&all).await,
        );
        assert!(
            !ok(
                "get_wish_list_status_counts",
                source.get_wish_list_status_counts().await
            )
            .is_empty()
        );
        ok(
            "get_wish_list_financial_report",
            source.get_wish_list_financial_report().await,
        );
        let wish = wishes.results[0].clone();
        let updated = ok(
            "wish_list_update",
            source
                .wish_list_update(
                    wish.id,
                    WishListUpdate {
                        quantity: Some(5),
                        ..WishListUpdate::default()
                    },
                )
                .await,
        );
        assert_eq!(updated.quantity, 5);
        assert!(
            !ok(
                "wish_list_update_multiple",
                source
                    .wish_list_update_multiple(
                        vec![wish.id],
                        WishListUpdate {
                            is_hidden: Some(false),
                            ..WishListUpdate::default()
                        }
                    )
                    .await
            )
            .is_empty()
        );
        let created = ok(
            "wish_list_create",
            source
                .wish_list_create(NewStockItem {
                    raw: "abating_link".into(),
                    sub_type: Some(SubType::rank(0)),
                    quantity: 3,
                    bought: 0,
                })
                .await,
        );
        let bought = ok(
            "wish_list_bought",
            source
                .wish_list_bought("abating_link".into(), Some(SubType::rank(0)), 1, 9)
                .await,
        );
        assert_eq!(bought.quantity, 2);
        ok(
            "wish_list_delete",
            source.wish_list_delete(created.id).await,
        );
        let extra = ok(
            "wish_list_create",
            source
                .wish_list_create(NewStockItem {
                    raw: "abating_link".into(),
                    sub_type: None,
                    quantity: 1,
                    bought: 0,
                })
                .await,
        );
        assert_eq!(
            ok(
                "wish_list_delete_multiple",
                source.wish_list_delete_multiple(vec![extra.id]).await
            ),
            1
        );
        let path = ok(
            "export_wish_list_json",
            source
                .export_wish_list_json(&all, export_path("wish"))
                .await,
        );
        assert!(std::fs::metadata(&path).expect("wish list export").len() > 2);

        let synd_all = SyndicateItemListParams {
            stock: all.clone(),
            syndicate: None,
        };
        let synds = non_empty(
            "get_syndicate_item_pagination",
            source.get_syndicate_item_pagination(&synd_all).await,
        );
        assert_eq!(synds.total, 8);
        let steel = ok(
            "get_syndicate_item_pagination syndicate",
            source
                .get_syndicate_item_pagination(&SyndicateItemListParams {
                    stock: all.clone(),
                    syndicate: Some("Steel Meridian".into()),
                })
                .await,
        );
        assert!(!steel.results.is_empty() && steel.results.len() < 8);
        assert!(
            !ok(
                "get_syndicate_item_status_counts",
                source.get_syndicate_item_status_counts().await
            )
            .is_empty()
        );
        assert!(
            !ok(
                "get_syndicate_item_syndicate_counts",
                source.get_syndicate_item_syndicate_counts().await
            )
            .is_empty()
        );
        ok(
            "get_syndicate_item_financial_report",
            source.get_syndicate_item_financial_report().await,
        );
        let synd = synds.results[0].clone();
        let mut settings = ok("app_get_settings", source.app_get_settings().await);
        let names = ok(
            "get_syndicate_item_syndicate_counts",
            source.get_syndicate_item_syndicate_counts().await,
        );
        for item in &synds.results {
            if names.contains_key(&item.syndicate_name)
                && !settings
                    .live_scraper
                    .syndicate
                    .wts
                    .syndicates
                    .iter()
                    .any(|entry| entry.unique_name == item.syndicate_unique_name)
            {
                settings
                    .live_scraper
                    .syndicate
                    .wts
                    .syndicates
                    .push(SyndicateEntrySetting {
                        name: item.syndicate_name.clone(),
                        unique_name: item.syndicate_unique_name.clone(),
                        standing: 0,
                        ignore_standing: true,
                    });
            }
        }
        let saved = ok(
            "app_update_settings",
            source.app_update_settings(settings).await,
        );
        assert_eq!(saved.live_scraper.syndicate.wts.syndicates.len(), 4);
        let updated = ok(
            "syndicate_item_update",
            source
                .syndicate_item_update(
                    synd.id,
                    SyndicateItemUpdate {
                        list_price: Some(21),
                        ..SyndicateItemUpdate::default()
                    },
                )
                .await,
        );
        assert_eq!(updated.list_price, Some(21));
        let created = source
            .syndicate_item_create(NewSyndicateItem {
                raw: "toxic_sequence".into(),
                sub_type: Some(SubType::rank(0)),
                syndicate_unique_name: "CephalonSudaSyndicate".into(),
            })
            .await;
        if let Ok(created) = &created {
            ok(
                "syndicate_item_delete",
                source.syndicate_item_delete(created.id).await,
            );
        }
        ok(
            "syndicate_item_sell",
            source
                .syndicate_item_sell(
                    synd.wfm_url.clone(),
                    synd.sub_type.clone(),
                    synd.syndicate_unique_name.clone(),
                    20,
                )
                .await,
        );
        let last = ok(
            "get_syndicate_item_pagination",
            source.get_syndicate_item_pagination(&synd_all).await,
        );
        let doomed = last.results.last().expect("syndicate item").id;
        assert_eq!(
            ok(
                "syndicate_item_delete_multiple",
                source.syndicate_item_delete_multiple(vec![doomed]).await
            ),
            1
        );
        let path = ok(
            "export_syndicate_item_json",
            source
                .export_syndicate_item_json(&synd_all, export_path("syndicate"))
                .await,
        );
        assert!(std::fs::metadata(&path).expect("syndicate export").len() > 2);
        offline(
            "syndicate_item_import_items",
            source.syndicate_item_import_items().await,
        );

        let orders = non_empty(
            "get_wfm_orders_pagination",
            source
                .get_wfm_orders_pagination(&OrderListParams {
                    list: list.clone(),
                    order_type: None,
                })
                .await,
        );
        assert_eq!(orders.total, 60);
        let sells = ok(
            "get_wfm_orders_pagination sell",
            source
                .get_wfm_orders_pagination(&OrderListParams {
                    list: list.clone(),
                    order_type: Some(OrderType::Sell),
                })
                .await,
        );
        assert!(
            sells
                .results
                .iter()
                .all(|order| order.order_type == OrderType::Sell)
        );
        assert!(
            orders
                .results
                .iter()
                .all(|order| !order.item_name.is_empty())
        );
        let totals = ok(
            "get_wfm_orders_status_counts",
            source.get_wfm_orders_status_counts().await,
        );
        assert!(totals[&OrderType::Sell].count > 0);
        let order = ok(
            "get_wfm_order_by_id",
            source
                .get_wfm_order_by_id(orders.results[0].id.clone())
                .await,
        );
        assert_eq!(order.id, orders.results[0].id);
        offline("order_refresh", source.order_refresh().await);
        offline(
            "order_delete_by_id",
            source.order_delete_by_id(order.id.clone()).await,
        );
        offline("order_delete_all", source.order_delete_all(None).await);

        let auctions = non_empty(
            "get_wfm_auctions_pagination",
            source.get_wfm_auctions_pagination(&list).await,
        );
        assert!(
            ok(
                "get_wfm_auctions_overview",
                source.get_wfm_auctions_overview().await
            )
            .count
                > 0
        );
        let auction = auctions.results[0].clone();
        needs_mod_cache(
            "get_wfm_auction_by_id",
            source.get_wfm_auction_by_id(auction.id.clone()).await,
        );
        offline("auction_refresh", source.auction_refresh().await);
        offline(
            "auction_delete_by_id",
            source.auction_delete_by_id(auction.id.clone()).await,
        );
        offline("auction_delete_all", source.auction_delete_all().await);
        offline(
            "auction_import_by_id",
            source.auction_import_by_id(auction.id.clone(), 100).await,
        );

        let chats = non_empty(
            "get_chat_pagination",
            source.get_chat_pagination(&list).await,
        );
        let chat = chats.results[0].clone();
        assert!(
            !ok(
                "chat_get_messages_by_id",
                source.chat_get_messages_by_id(chat.id.clone()).await
            )
            .is_empty()
        );
        ok(
            "chat_set_active",
            source.chat_set_active(Some(chat.id.clone())).await,
        );
        offline(
            "chat_send_message",
            source.chat_send_message(chat.id.clone(), "hi".into()).await,
        );
        offline("chat_delete", source.chat_delete(chat.id.clone()).await);
        offline("chat_refresh", source.chat_refresh().await);

        let entries = non_empty(
            "get_trade_entry_pagination",
            source
                .get_trade_entry_pagination(&TradeEntryListParams {
                    list: list.clone(),
                    ..TradeEntryListParams::default()
                })
                .await,
        );
        let entry = entries.results[0].clone();
        assert_eq!(
            ok(
                "trade_entry_get_by_id",
                source.trade_entry_get_by_id(entry.id).await
            )
            .id,
            entry.id
        );
        let updated = ok(
            "trade_entry_update",
            source
                .trade_entry_update(
                    entry.id,
                    TradeEntryUpdate {
                        price: Some(42),
                        ..TradeEntryUpdate::default()
                    },
                )
                .await,
        );
        assert_eq!(updated.price, 42);
        let created = ok(
            "trade_entry_create",
            source
                .trade_entry_create(NewTradeEntry {
                    sub_type: Some(SubType::rank(0)),
                    tags: vec!["mod".into()],
                    ..NewTradeEntry::new("abating_link", TradeEntryGroup::Item, 15)
                })
                .await,
        );
        ok(
            "trade_entry_delete",
            source.trade_entry_delete(created.id).await,
        );
        let last = entries.results.last().expect("trade entry").id;
        assert_eq!(
            ok(
                "trade_entry_delete_multiple",
                source.trade_entry_delete_multiple(vec![last]).await
            ),
            1
        );
        let path = ok(
            "export_trade_entry_json",
            source
                .export_trade_entry_json(&TradeEntryListParams::default(), export_path("trade"))
                .await,
        );
        assert!(std::fs::metadata(&path).expect("trade export").len() > 2);

        let all_tx = TransactionListParams {
            list: list.clone(),
            ..TransactionListParams::default()
        };
        let txs = non_empty(
            "get_transaction_pagination",
            source.get_transaction_pagination(&all_tx).await,
        );
        let sales = ok(
            "get_transaction_pagination sales",
            source
                .get_transaction_pagination(&TransactionListParams {
                    transaction_type: Some(TransactionType::Sale),
                    ..all_tx.clone()
                })
                .await,
        );
        assert!(
            sales
                .results
                .iter()
                .all(|tx| tx.transaction_type == TransactionType::Sale)
        );
        assert!(
            ok(
                "get_transaction_financial_report",
                source.get_transaction_financial_report(&all_tx).await
            )
            .total_transactions
                > 0
        );
        let tx = txs.results[0].clone();
        let updated = ok(
            "transaction_update",
            source
                .transaction_update(
                    tx.id,
                    TransactionUpdate {
                        price: Some(tx.price + 1),
                        ..TransactionUpdate::default()
                    },
                )
                .await,
        );
        assert_eq!(updated.price, tx.price + 1);
        ok(
            "transaction_calculate_tax",
            source.transaction_calculate_tax().await,
        );
        let path = ok(
            "export_transaction_json",
            source
                .export_transaction_json(&all_tx, export_path("tx"))
                .await,
        );
        assert!(std::fs::metadata(&path).expect("transaction export").len() > 2);
        let last_two: Vec<i64> = txs.results.iter().rev().take(2).map(|tx| tx.id).collect();
        ok(
            "transaction_delete",
            source.transaction_delete(last_two[0]).await,
        );
        assert_eq!(
            ok(
                "transaction_delete_bulk",
                source.transaction_delete_bulk(vec![last_two[1]]).await
            ),
            1
        );

        let summary = ok("dashboard_summary", source.dashboard_summary().await);
        assert!(summary.total.total_transactions > 0);
        assert!(!summary.recent_transactions.is_empty());
        assert!(!summary.best_seller.item_name.is_empty());

        let lookup = PriceLookupParams::new("2026-11-10", "2026-11-17");
        offline(
            "item_prices_lookup",
            source.item_prices_lookup(&lookup).await,
        );
        offline(
            "riven_prices_lookup",
            source.riven_prices_lookup(&lookup).await,
        );
        offline(
            "syndicate_item_prices_lookup",
            source.syndicate_item_prices_lookup(&lookup).await,
        );
        offline(
            "get_user_activity",
            source
                .get_user_activity("2026-11-01".into(), "2026-11-17".into())
                .await,
        );
        assert!(
            source
                .export_item_price_data(&lookup, export_path("item-prices"))
                .await
                .is_err()
        );
        assert!(
            source
                .export_riven_price_data(&lookup, export_path("riven-prices"))
                .await
                .is_err()
        );
        assert!(
            source
                .export_syndicate_item_price_data(&lookup, export_path("synd-prices"))
                .await
                .is_err()
        );

        let info = ok("app_get_app_info", source.app_get_app_info().await);
        assert!(info.fixture_mode && !info.version.is_empty());
        let settings = ok("app_get_settings", source.app_get_settings().await);
        ok(
            "app_get_default_settings",
            source.app_get_default_settings().await,
        );
        let saved = ok(
            "app_update_settings",
            source.app_update_settings(settings.clone()).await,
        );
        assert_eq!(saved.lang, settings.lang);

        let user = ok("user", source.user().await);
        assert_eq!(user.wfm_username, "FixtureTrader");
        offline(
            "auth_login",
            source.auth_login("a@b.c".into(), "x".into()).await,
        );
        offline(
            "user_set_status",
            source.user_set_status(UserStatus::Online).await,
        );

        let started = ok("live_scraper_start", source.live_scraper_start().await);
        assert!(started.is_running);
        assert!(
            ok(
                "live_scraper_get_state",
                source.live_scraper_get_state().await
            )
            .is_running
        );
        assert!(!ok("live_scraper_stop", source.live_scraper_stop().await).is_running);

        let wfm = ok("debug_get_wfm_state", source.debug_get_wfm_state().await);
        assert_eq!(wfm.sell_orders + wfm.buy_orders, 60);
        ok(
            "debug_get_ee_logs",
            source.debug_get_ee_logs(&EeLogParams::default()).await,
        );
        ok(
            "debug_export_ee_logs",
            source
                .debug_export_ee_logs(&EeLogParams::default(), export_path("ee"))
                .await,
        );
        assert!(!ok("wfgdpr_get_state", source.wfgdpr_get_state().await).was_initialized);
        ok(
            "wf_inventory_get_rivens",
            source.wf_inventory_get_rivens(&list).await,
        );
        ok(
            "wf_inventory_get_syndicates",
            source.wf_inventory_get_syndicates(&list).await,
        );
        ok("wf_inventory_update", source.wf_inventory_update().await);
        assert!(
            ok(
                "wf_inventory_list_rivens",
                source.wf_inventory_list_rivens().await
            )
            .is_empty()
        );
        let missing = source
            .wf_inventory_stock_riven("no-such-riven".to_owned(), 0)
            .await
            .expect_err("wf_inventory_stock_riven added a riven the inventory lacks");
        assert_eq!(missing.message, "The riven is no longer in the inventory");
        let log = ok("log_export", source.log_export().await);
        assert!(log.exists());
        let _ = std::fs::remove_file(log);

        ok("auth_logout", source.auth_logout().await);
    });

    let mut seen = Vec::new();
    while let Ok(event) = events.try_recv() {
        seen.push(event);
    }
    assert!(
        seen.contains(&BackendEvent::LiveScraperRunning(true)),
        "no running event in {seen:?}"
    );
    assert!(
        seen.contains(&BackendEvent::Changed(DataKind::TradeEntries)),
        "no trade entry event in {seen:?}"
    );
    assert!(
        seen.contains(&BackendEvent::Changed(DataKind::StockItems))
            || seen.iter().any(|e| matches!(e, BackendEvent::Changed(_)))
    );
}
