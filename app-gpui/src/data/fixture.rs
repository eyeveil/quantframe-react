use std::collections::BTreeMap;

use super::DataSource;
use crate::model::{
    CoolDownInfo, FinancialReport, LiveScraperMessage, LiveScraperState, Paginated, PriceHistory,
    SortDirection, StockItem, StockItemListParams, StockItemProperties, StockStatus, SubType, User,
    UserStatus,
};

/// A fixed stock of real Warframe items with plausible platinum prices, for
/// building screens before the backend is linked in.
pub struct FixtureSource {
    items: Vec<StockItem>,
}

enum Sub {
    None,
    Rank(i64),
    Variant(&'static str),
    Stars(i64, i64),
}

struct Seed {
    name: &'static str,
    sub: Sub,
    bought: i64,
    list_price: Option<i64>,
    owned: i64,
    status: StockStatus,
    min_price: Option<i64>,
}

const fn seed(
    name: &'static str,
    sub: Sub,
    bought: i64,
    list_price: Option<i64>,
    owned: i64,
    status: StockStatus,
    min_price: Option<i64>,
) -> Seed {
    Seed {
        name,
        sub,
        bought,
        list_price,
        owned,
        status,
        min_price,
    }
}

use StockStatus::{
    InActive, Live, MaxPriceDrop, NoSellers, OrderLimit, Overpriced, Pending, SMALimit,
    ToLowProfit, Underpriced,
};

const SEEDS: &[Seed] = &[
    seed(
        "Arcane Energize",
        Sub::Rank(5),
        245,
        Some(289),
        1,
        Live,
        Some(270),
    ),
    seed("Arcane Grace", Sub::Rank(5), 150, Some(172), 1, Live, None),
    seed(
        "Arcane Avenger",
        Sub::Rank(5),
        55,
        Some(68),
        1,
        Overpriced,
        None,
    ),
    seed("Molt Augmented", Sub::Rank(5), 38, Some(46), 2, Live, None),
    seed(
        "Primary Merciless",
        Sub::Rank(5),
        30,
        Some(37),
        1,
        Live,
        None,
    ),
    seed("Melee Duplicate", Sub::Rank(5), 28, Some(34), 1, Live, None),
    seed(
        "Primed Sure Footed",
        Sub::Rank(0),
        110,
        Some(128),
        1,
        Live,
        Some(120),
    ),
    seed(
        "Primed Continuity",
        Sub::Rank(10),
        42,
        Some(55),
        3,
        Live,
        Some(50),
    ),
    seed("Primed Flow", Sub::Rank(10), 45, Some(58), 2, Live, None),
    seed(
        "Primed Reach",
        Sub::Rank(10),
        35,
        Some(44),
        1,
        Overpriced,
        None,
    ),
    seed(
        "Primed Target Cracker",
        Sub::Rank(10),
        30,
        Some(38),
        1,
        Live,
        None,
    ),
    seed(
        "Primed Point Blank",
        Sub::Rank(0),
        20,
        Some(27),
        1,
        Live,
        None,
    ),
    seed(
        "Primed Pressure Point",
        Sub::Rank(0),
        18,
        None,
        2,
        ToLowProfit,
        None,
    ),
    seed(
        "Condition Overload",
        Sub::Rank(0),
        12,
        Some(16),
        4,
        Live,
        None,
    ),
    seed("Blind Rage", Sub::Rank(10), 22, Some(30), 1, Live, None),
    seed("Narrow Minded", Sub::Rank(10), 20, None, 1, Pending, None),
    seed(
        "Fleeting Expertise",
        Sub::Rank(5),
        12,
        Some(16),
        2,
        MaxPriceDrop,
        None,
    ),
    seed("Overextended", Sub::Rank(5), 9, None, 1, Pending, None),
    seed(
        "Transient Fortitude",
        Sub::Rank(0),
        8,
        Some(11),
        3,
        Live,
        None,
    ),
    seed("Adaptation", Sub::Rank(0), 9, Some(12), 2, SMALimit, None),
    seed(
        "Galvanized Chamber",
        Sub::Rank(0),
        14,
        Some(18),
        2,
        Live,
        None,
    ),
    seed(
        "Corrosive Projection",
        Sub::Rank(5),
        10,
        Some(14),
        2,
        Live,
        None,
    ),
    seed(
        "Wisp Prime Set",
        Sub::None,
        92,
        Some(108),
        1,
        Live,
        Some(100),
    ),
    seed("Gauss Prime Set", Sub::None, 110, Some(126), 1, Live, None),
    seed("Mesa Prime Set", Sub::None, 78, Some(90), 2, Live, Some(85)),
    seed("Saryn Prime Set", Sub::None, 60, Some(71), 1, Live, None),
    seed(
        "Volt Prime Set",
        Sub::None,
        48,
        Some(55),
        2,
        Underpriced,
        None,
    ),
    seed("Rhino Prime Set", Sub::None, 40, Some(47), 1, Live, None),
    seed("Khora Prime Set", Sub::None, 62, None, 1, ToLowProfit, None),
    seed("Nekros Prime Set", Sub::None, 55, Some(66), 1, Live, None),
    seed("Harrow Prime Set", Sub::None, 45, Some(53), 1, Live, None),
    seed("Titania Prime Set", Sub::None, 52, Some(61), 1, Live, None),
    seed(
        "Glaive Prime Set",
        Sub::None,
        95,
        Some(112),
        1,
        Live,
        Some(105),
    ),
    seed("Kronen Prime Set", Sub::None, 32, Some(39), 2, Live, None),
    seed("Nikana Prime Set", Sub::None, 30, None, 1, NoSellers, None),
    seed(
        "Ash Prime Systems Blueprint",
        Sub::None,
        18,
        Some(24),
        3,
        Live,
        None,
    ),
    seed(
        "Loki Prime Systems Blueprint",
        Sub::None,
        22,
        Some(28),
        1,
        Live,
        None,
    ),
    seed(
        "Wukong Prime Neuroptics Blueprint",
        Sub::None,
        9,
        Some(12),
        4,
        Live,
        None,
    ),
    seed(
        "Nova Prime Chassis Blueprint",
        Sub::None,
        14,
        None,
        2,
        InActive,
        None,
    ),
    seed("Lex Prime Receiver", Sub::None, 3, Some(5), 6, Live, None),
    seed(
        "Axi L4 Relic",
        Sub::Variant("radiant"),
        12,
        Some(16),
        5,
        Live,
        None,
    ),
    seed(
        "Lith G1 Relic",
        Sub::Variant("intact"),
        4,
        Some(6),
        8,
        Live,
        None,
    ),
    seed(
        "Ayatan Anasa Sculpture",
        Sub::Stars(2, 2),
        9,
        Some(13),
        3,
        Live,
        None,
    ),
    seed(
        "Ayatan Orta Sculpture",
        Sub::Stars(1, 3),
        6,
        Some(8),
        2,
        OrderLimit,
        None,
    ),
];

const HIDDEN: &[&str] = &["Nova Prime Chassis Blueprint", "Narrow Minded"];
const BULK: &[&str] = &["Lex Prime Receiver", "Lith G1 Relic", "Condition Overload"];
const PROFIT_OVERRIDE: &[&str] = &["Arcane Energize", "Glaive Prime Set"];
const ON_COOLDOWN: &str = "Fleeting Expertise";

impl FixtureSource {
    pub fn new() -> Self {
        let items = SEEDS
            .iter()
            .enumerate()
            .map(|(index, seed)| stock_item(index as i64 + 1, seed))
            .collect();
        Self { items }
    }
}

fn stock_item(id: i64, seed: &Seed) -> StockItem {
    let wfm_url = seed.name.to_lowercase().replace(' ', "_");
    let sub_type = match seed.sub {
        Sub::None => None,
        Sub::Rank(rank) => Some(SubType::rank(rank)),
        Sub::Variant(variant) => Some(SubType::variant(variant)),
        Sub::Stars(amber, cyan) => Some(SubType::stars(amber, cyan)),
    };
    StockItem {
        id,
        wfm_id: format!("5f{:022x}", id * 7919),
        item_unique_name: format!("/Lotus/Fixture/{}", seed.name.replace(' ', "")),
        wfm_url,
        item_name: seed.name.to_owned(),
        sub_type,
        bought: seed.bought,
        list_price: seed.list_price,
        owned: seed.owned,
        is_hidden: HIDDEN.contains(&seed.name),
        status: seed.status,
        price_history: price_history(id, seed.list_price.unwrap_or(seed.bought)),
        updated_at: "2026-11-17T21:14:00Z".to_owned(),
        created_at: "2026-11-02T18:40:00Z".to_owned(),
        properties: StockItemProperties {
            min_price: seed.min_price,
            min_sma: None,
            min_profit: PROFIT_OVERRIDE.contains(&seed.name).then_some(15),
            is_bulk: BULK.contains(&seed.name),
            cooldown: (seed.name == ON_COOLDOWN).then(|| CoolDownInfo {
                id: format!("stock_item_{id}"),
                cooldown_type: "price_drop".to_owned(),
                start_time: "2026-11-17T21:10:00Z".to_owned(),
                end_time: "2026-11-17T21:40:00Z".to_owned(),
            }),
        },
    }
}

/// Twelve checks, ten minutes apart, that drift within 8% of `price` and end on it.
fn price_history(id: i64, price: i64) -> Vec<PriceHistory> {
    const DRIFT_PERCENT: [i64; 11] = [-6, -8, -3, -5, 2, 4, -1, 5, 7, 3, 1];
    const CHECKS: usize = DRIFT_PERCENT.len() + 1;
    (0..CHECKS)
        .map(|check| {
            let percent = if check == CHECKS - 1 {
                0
            } else {
                DRIFT_PERCENT[(check + id as usize) % DRIFT_PERCENT.len()]
            };
            PriceHistory {
                created_at: format!(
                    "2026-11-17T{:02}:{:02}:00Z",
                    19 + check / 6,
                    (check % 6) * 10
                ),
                price: (price + price * percent / 100).max(1),
            }
        })
        .collect()
}

fn compare(a: &StockItem, b: &StockItem, sort_by: &str) -> std::cmp::Ordering {
    match sort_by {
        "bought" => a.bought.cmp(&b.bought),
        "list_price" => a.list_price.cmp(&b.list_price),
        "minimum_price" => a.properties.min_price.cmp(&b.properties.min_price),
        "owned" => a.owned.cmp(&b.owned),
        "status" => a.status.label().cmp(b.status.label()),
        _ => a.item_name.cmp(&b.item_name),
    }
}

impl DataSource for FixtureSource {
    fn get_stock_item_pagination(&self, params: &StockItemListParams) -> Paginated<StockItem> {
        let query = params.query.as_deref().unwrap_or("").trim().to_lowercase();
        let mut matches: Vec<StockItem> = self
            .items
            .iter()
            .filter(|item| params.status.is_none_or(|status| item.status == status))
            .filter(|item| query.is_empty() || item.item_name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        if let Some(sort_by) = &params.sort_by {
            matches.sort_by(|a, b| {
                let ordering = compare(a, b, sort_by);
                match params.sort_direction {
                    SortDirection::Asc => ordering,
                    SortDirection::Desc => ordering.reverse(),
                }
            });
        }
        let total = matches.len() as i64;
        let limit = params.limit.max(1);
        let results = matches
            .into_iter()
            .skip(((params.page.max(1) - 1) * limit) as usize)
            .take(limit as usize)
            .collect();
        Paginated {
            total,
            limit,
            page: params.page.max(1),
            total_pages: (total + limit - 1) / limit,
            results,
        }
    }

    fn get_stock_item_status_counts(&self) -> BTreeMap<StockStatus, usize> {
        let mut counts = BTreeMap::new();
        for item in &self.items {
            *counts.entry(item.status).or_default() += 1;
        }
        counts
    }

    fn get_stock_item_financial_report(&self) -> FinancialReport {
        let expenses: i64 = self.items.iter().map(|item| item.bought * item.owned).sum();
        let revenue: i64 = self
            .items
            .iter()
            .map(|item| item.list_price.unwrap_or(0) * item.owned)
            .sum();
        let units: i64 = self.items.iter().map(|item| item.owned).sum();
        let total_profit = revenue - expenses;
        FinancialReport {
            total_transactions: units,
            total_profit,
            average_profit: total_profit as f64 / units as f64,
            profit_margin: total_profit as f64 / revenue as f64,
            roi: total_profit as f64 / expenses as f64,
            revenue,
            expenses,
            purchases_count: units,
            ..FinancialReport::default()
        }
    }

    fn live_scraper_get_state(&self) -> LiveScraperState {
        LiveScraperState {
            is_running: true,
            message: Some(LiveScraperMessage::CheckingItem {
                name: "Volt Prime Set".to_owned(),
                current: 27,
                total: self.items.len(),
            }),
        }
    }

    fn user(&self) -> User {
        User {
            wfm_id: "5f0000000000000000001f3a".to_owned(),
            wfm_username: "TennoTrader".to_owned(),
            wfm_status: UserStatus::InGame,
            platform: "pc".to_owned(),
            region: "en".to_owned(),
            order_limit: 100,
            auctions_limit: 50,
            unread_messages: 3,
            verification: true,
        }
    }
}
