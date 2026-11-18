//! Fixture rows for the screens the backend-backed data source doesn't feed
//! yet: rivens, the wish list, syndicate items, Warframe Market orders and
//! auctions, and trade messages. The names are real Warframe items and the
//! prices are plausible platinum amounts. Each function returns the rows one
//! backend command will return.

use crate::{
    listings::model::{
        MinMax, RivenAttribute, StockRiven, StockRivenFilter, StockRivenProperties, SyndicateItem,
        SyndicateItemProperties, WishListItem, WishListProperties,
    },
    model::{PriceHistory, StockStatus, SubType},
    orders::model::{Auction, Order, OrderType},
    trade_messages::model::{RivenCandidate, TradeEntry, TradeEntryProperties, TradeGroup},
};

const CHECKED_AT: &str = "2026-11-17T21:14:00Z";

fn slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "_")
}

/// Twelve checks, ten minutes apart, that drift within 8% of `price` and end
/// on it.
fn price_history(seed: i64, price: i64) -> Vec<PriceHistory> {
    const DRIFT_PERCENT: [i64; 11] = [-6, -8, -3, -5, 2, 4, -1, 5, 7, 3, 1];
    const CHECKS: usize = DRIFT_PERCENT.len() + 1;
    (0..CHECKS)
        .map(|check| {
            let percent = if check == CHECKS - 1 {
                0
            } else {
                DRIFT_PERCENT[(check + seed as usize) % DRIFT_PERCENT.len()]
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

fn attribute(name: &str, value: f64) -> RivenAttribute {
    RivenAttribute {
        url_name: slug(name),
        name: name.to_owned(),
        value,
        positive: value >= 0.0,
    }
}

struct RivenSeed {
    weapon: &'static str,
    kind: &'static str,
    name: &'static str,
    attributes: &'static [(&'static str, f64)],
    mastery_rank: i64,
    re_rolls: i64,
    polarity: &'static str,
    rank: i64,
    bought: i64,
    list_price: Option<i64>,
    min_price: Option<i64>,
    status: StockStatus,
}

const RIVENS: &[RivenSeed] = &[
    RivenSeed {
        weapon: "Rubico",
        kind: "Rifle",
        name: "Crita-visican",
        attributes: &[
            ("Critical Chance", 172.4),
            ("Multishot", 98.1),
            ("Critical Damage", 121.7),
            ("Zoom", -38.2),
        ],
        mastery_rank: 16,
        re_rolls: 12,
        polarity: "Madurai",
        rank: 8,
        bought: 1850,
        list_price: Some(2400),
        min_price: Some(2100),
        status: StockStatus::Live,
    },
    RivenSeed {
        weapon: "Torid",
        kind: "Rifle",
        name: "Toxi-satitis",
        attributes: &[
            ("Toxin", 148.9),
            ("Status Chance", 131.2),
            ("Multishot", 104.6),
        ],
        mastery_rank: 14,
        re_rolls: 31,
        polarity: "Naramon",
        rank: 8,
        bought: 3200,
        list_price: Some(4350),
        min_price: Some(3800),
        status: StockStatus::Live,
    },
    RivenSeed {
        weapon: "Burston",
        kind: "Rifle",
        name: "Vexi-critacan",
        attributes: &[
            ("Critical Chance", 164.3),
            ("Critical Damage", 118.4),
            ("Fire Rate", 71.5),
            ("Reload Speed", -44.1),
        ],
        mastery_rank: 11,
        re_rolls: 47,
        polarity: "Madurai",
        rank: 8,
        bought: 2600,
        list_price: Some(3100),
        min_price: None,
        status: StockStatus::Overpriced,
    },
    RivenSeed {
        weapon: "Dual Toxocyst",
        kind: "Pistol",
        name: "Sati-hexadex",
        attributes: &[
            ("Status Chance", 142.8),
            ("Slash", 126.3),
            ("Damage", 187.9),
        ],
        mastery_rank: 13,
        re_rolls: 22,
        polarity: "Vazarin",
        rank: 8,
        bought: 1400,
        list_price: Some(1750),
        min_price: Some(1500),
        status: StockStatus::Live,
    },
    RivenSeed {
        weapon: "Magistar",
        kind: "Melee",
        name: "Crita-gelidex",
        attributes: &[
            ("Critical Chance", 138.6),
            ("Cold", 94.2),
            ("Critical Damage", 97.5),
        ],
        mastery_rank: 12,
        re_rolls: 9,
        polarity: "Madurai",
        rank: 8,
        bought: 950,
        list_price: Some(1200),
        min_price: None,
        status: StockStatus::Live,
    },
    RivenSeed {
        weapon: "Nikana",
        kind: "Melee",
        name: "Vexi-croniata",
        attributes: &[
            ("Attack Speed", 61.8),
            ("Critical Damage", 104.9),
            ("Range", 2.1),
            ("Finisher Damage", -71.4),
        ],
        mastery_rank: 10,
        re_rolls: 18,
        polarity: "Naramon",
        rank: 8,
        bought: 420,
        list_price: Some(560),
        min_price: Some(480),
        status: StockStatus::Live,
    },
    RivenSeed {
        weapon: "Kronen",
        kind: "Melee",
        name: "Sci-tempitis",
        attributes: &[
            ("Slash", 128.7),
            ("Combo Duration", 9.4),
            ("Status Chance", 96.1),
        ],
        mastery_rank: 9,
        re_rolls: 4,
        polarity: "Vazarin",
        rank: 8,
        bought: 180,
        list_price: Some(230),
        min_price: None,
        status: StockStatus::Underpriced,
    },
    RivenSeed {
        weapon: "Phantasma",
        kind: "Shotgun",
        name: "Igni-sati",
        attributes: &[("Heat", 151.2), ("Status Chance", 137.6)],
        mastery_rank: 13,
        re_rolls: 2,
        polarity: "Madurai",
        rank: 0,
        bought: 150,
        list_price: Some(195),
        min_price: None,
        status: StockStatus::Live,
    },
    RivenSeed {
        weapon: "Acceltra",
        kind: "Rifle",
        name: "Crita-acrican",
        attributes: &[
            ("Critical Chance", 149.5),
            ("Critical Damage", 112.8),
            ("Multishot", 89.3),
            ("Ammo Maximum", -52.6),
        ],
        mastery_rank: 15,
        re_rolls: 26,
        polarity: "Madurai",
        rank: 8,
        bought: 780,
        list_price: Some(940),
        min_price: Some(850),
        status: StockStatus::Live,
    },
    RivenSeed {
        weapon: "Cedo",
        kind: "Shotgun",
        name: "Visi-toxicron",
        attributes: &[
            ("Multishot", 106.4),
            ("Toxin", 121.9),
            ("Fire Rate", 68.3),
        ],
        mastery_rank: 14,
        re_rolls: 15,
        polarity: "Naramon",
        rank: 8,
        bought: 640,
        list_price: None,
        min_price: Some(700),
        status: StockStatus::NoSellers,
    },
    RivenSeed {
        weapon: "Nukor",
        kind: "Pistol",
        name: "Acri-critatis",
        attributes: &[
            ("Critical Damage", 131.4),
            ("Critical Chance", 158.2),
            ("Status Duration", -61.7),
        ],
        mastery_rank: 16,
        re_rolls: 38,
        polarity: "Madurai",
        rank: 8,
        bought: 1100,
        list_price: Some(1320),
        min_price: None,
        status: StockStatus::ToLowProfit,
    },
    RivenSeed {
        weapon: "Epitaph",
        kind: "Pistol",
        name: "Geli-critatis",
        attributes: &[
            ("Cold", 102.7),
            ("Critical Chance", 144.1),
            ("Critical Damage", 99.6),
        ],
        mastery_rank: 12,
        re_rolls: 7,
        polarity: "Vazarin",
        rank: 8,
        bought: 520,
        list_price: Some(690),
        min_price: None,
        status: StockStatus::Live,
    },
    RivenSeed {
        weapon: "Glaive",
        kind: "Melee",
        name: "Crita-toxitox",
        attributes: &[
            ("Critical Chance", 127.3),
            ("Toxin", 88.4),
            ("Initial Combo", 22.5),
        ],
        mastery_rank: 8,
        re_rolls: 0,
        polarity: "Madurai",
        rank: 0,
        bought: 60,
        list_price: None,
        min_price: None,
        status: StockStatus::Pending,
    },
    RivenSeed {
        weapon: "Tenora",
        kind: "Rifle",
        name: "Satiata",
        attributes: &[("Status Chance", 118.9), ("Fire Rate", 54.7)],
        mastery_rank: 9,
        re_rolls: 1,
        polarity: "Naramon",
        rank: 0,
        bought: 45,
        list_price: Some(70),
        min_price: None,
        status: StockStatus::InActive,
    },
];

const HIDDEN_RIVENS: &[&str] = &["Tenora"];
const FILTERED_RIVENS: &[&str] = &["Rubico", "Torid", "Burston", "Acceltra"];
const PREDICTED_RIVENS: &[&str] = &["Phantasma", "Kronen", "Epitaph"];

/// The rows of `get_stock_riven_pagination`.
pub fn stock_rivens() -> Vec<StockRiven> {
    RIVENS
        .iter()
        .enumerate()
        .map(|(index, seed)| {
            let id = index as i64 + 1;
            StockRiven {
                id,
                wfm_weapon_url: slug(seed.weapon),
                weapon_name: seed.weapon.to_owned(),
                weapon_type: seed.kind.to_owned(),
                mod_name: seed.name.to_owned(),
                attributes: seed
                    .attributes
                    .iter()
                    .map(|(name, value)| attribute(name, *value))
                    .collect(),
                mastery_rank: seed.mastery_rank,
                re_rolls: seed.re_rolls,
                polarity: seed.polarity.to_owned(),
                sub_type: Some(SubType::rank(seed.rank)),
                bought: seed.bought,
                list_price: seed.list_price,
                is_hidden: HIDDEN_RIVENS.contains(&seed.weapon),
                status: seed.status,
                price_history: price_history(id, seed.list_price.unwrap_or(seed.bought)),
                updated_at: CHECKED_AT.to_owned(),
                comment: String::new(),
                filter: StockRivenFilter {
                    enabled: FILTERED_RIVENS.contains(&seed.weapon),
                    re_rolls: MinMax::default(),
                    mastery_rank: MinMax {
                        min: None,
                        max: Some(16),
                    },
                    required_negative: seed.attributes.iter().any(|(_, value)| *value < 0.0),
                    similarity: Some(75),
                },
                properties: StockRivenProperties {
                    min_price: seed.min_price,
                    is_predicted: PREDICTED_RIVENS.contains(&seed.weapon),
                },
            }
        })
        .collect()
}

/// The weapons a riven can be created for, from `cache_get_riven_weapons`.
pub fn riven_weapons() -> Vec<String> {
    [
        "Acceltra",
        "Braton",
        "Burston",
        "Catchmoon",
        "Cedo",
        "Dual Toxocyst",
        "Epitaph",
        "Felarx",
        "Glaive",
        "Kronen",
        "Latron",
        "Magistar",
        "Nikana",
        "Nukor",
        "Phantasma",
        "Rubico",
        "Sporelacer",
        "Tenora",
        "Torid",
        "Vectis",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// The attributes a riven can roll, from `cache_get_riven_attributes`.
pub fn riven_attributes() -> Vec<String> {
    [
        "Attack Speed",
        "Cold",
        "Critical Chance",
        "Critical Damage",
        "Damage",
        "Electricity",
        "Fire Rate",
        "Heat",
        "Multishot",
        "Range",
        "Reload Speed",
        "Slash",
        "Status Chance",
        "Status Duration",
        "Toxin",
        "Zoom",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// A slice of `cache_get_tradable_items`, for the item pickers.
pub fn tradable_items() -> Vec<String> {
    [
        "Arcane Energize",
        "Arcane Grace",
        "Ash Prime Set",
        "Baruuk Prime Set",
        "Condition Overload",
        "Gauss Prime Set",
        "Glaive Prime Set",
        "Kronen Prime Set",
        "Lex Prime Receiver",
        "Lith G1 Relic",
        "Molt Augmented",
        "Nova Prime Chassis Blueprint",
        "Primed Continuity",
        "Primed Flow",
        "Primed Sure Footed",
        "Rhino Prime Set",
        "Saryn Prime Set",
        "Volt Prime Set",
        "Wisp Prime Set",
        "Xaku Prime Set",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// The rows of `get_wish_list_pagination`.
pub fn wish_list() -> Vec<WishListItem> {
    type Seed = (
        &'static str,
        Option<i64>,
        i64,
        Option<i64>,
        Option<i64>,
        StockStatus,
    );
    const SEEDS: &[Seed] = &[
        ("Wisp Prime Set", None, 1, Some(95), Some(82), StockStatus::Live),
        ("Gauss Prime Set", None, 1, Some(70), Some(61), StockStatus::Live),
        ("Xaku Prime Set", None, 2, Some(110), Some(96), StockStatus::Live),
        ("Arcane Energize", Some(0), 21, Some(48), Some(41), StockStatus::Live),
        ("Arcane Grace", Some(0), 10, Some(26), Some(22), StockStatus::Live),
        ("Primed Sure Footed", Some(0), 1, Some(105), Some(98), StockStatus::Live),
        ("Molt Augmented", Some(0), 21, Some(7), None, StockStatus::NoBuyers),
        ("Kronen Prime Set", None, 1, Some(60), Some(64), StockStatus::Overpriced),
        ("Glaive Prime Set", None, 1, Some(140), Some(118), StockStatus::Live),
        ("Lith G1 Relic", None, 12, Some(4), Some(3), StockStatus::OrderLimit),
        ("Baruuk Prime Set", None, 1, None, None, StockStatus::Pending),
    ];
    const BULK: &[&str] = &["Arcane Energize", "Lith G1 Relic", "Molt Augmented"];
    SEEDS
        .iter()
        .enumerate()
        .map(|(index, (name, rank, quantity, max_price, list_price, status))| {
            let id = index as i64 + 1;
            WishListItem {
                id,
                wfm_url: slug(name),
                item_name: (*name).to_owned(),
                sub_type: rank.map(SubType::rank),
                quantity: *quantity,
                list_price: *list_price,
                is_hidden: false,
                status: *status,
                price_history: price_history(id + 3, list_price.or(*max_price).unwrap_or(1)),
                updated_at: CHECKED_AT.to_owned(),
                properties: WishListProperties {
                    max_price: *max_price,
                    is_bulk: BULK.contains(name),
                },
            }
        })
        .collect()
}

/// The rows of `get_syndicate_item_pagination`.
pub fn syndicate_items() -> Vec<SyndicateItem> {
    type Seed = (
        &'static str,
        &'static str,
        i64,
        Option<i64>,
        Option<i64>,
        StockStatus,
    );
    const SEEDS: &[Seed] = &[
        ("Vaykor Hek", "Steel Meridian", 125_000, Some(42), Some(35), StockStatus::Live),
        ("Vaykor Sydon", "Steel Meridian", 125_000, Some(38), None, StockStatus::Live),
        ("Scattered Justice", "Steel Meridian", 25_000, Some(14), None, StockStatus::Live),
        ("Telos Boltace", "Arbiters of Hexis", 125_000, Some(36), None, StockStatus::Live),
        ("Telos Akbolto", "Arbiters of Hexis", 100_000, Some(24), None, StockStatus::ToLowProfit),
        ("Gilded Truth", "Arbiters of Hexis", 25_000, Some(11), None, StockStatus::Live),
        ("Synoid Gammacor", "Cephalon Suda", 100_000, Some(29), Some(25), StockStatus::Live),
        ("Synoid Simulor", "Cephalon Suda", 125_000, Some(33), None, StockStatus::Underpriced),
        ("Entropy Burst", "Cephalon Suda", 25_000, Some(12), None, StockStatus::Live),
        ("Secura Lecta", "The Perrin Sequence", 125_000, Some(47), Some(40), StockStatus::Live),
        ("Secura Penta", "The Perrin Sequence", 125_000, Some(35), None, StockStatus::Live),
        ("Rakta Cernos", "Red Veil", 125_000, Some(41), None, StockStatus::Live),
        ("Rakta Dark Dagger", "Red Veil", 100_000, Some(22), None, StockStatus::NoBuyers),
        ("Gleaming Blight", "Red Veil", 25_000, None, None, StockStatus::InsufficientStanding),
        ("Sancti Tigris", "New Loka", 125_000, Some(52), Some(45), StockStatus::Live),
        ("Sancti Magistar", "New Loka", 125_000, Some(58), None, StockStatus::Live),
        ("Winds of Purity", "New Loka", 25_000, None, None, StockStatus::Pending),
    ];
    const PROFIT_OVERRIDE: &[&str] = &["Secura Lecta", "Sancti Tigris"];
    SEEDS
        .iter()
        .enumerate()
        .map(|(index, (name, syndicate, standing, list_price, min_price, status))| {
            let id = index as i64 + 1;
            SyndicateItem {
                id,
                wfm_url: slug(name),
                item_name: (*name).to_owned(),
                sub_type: None,
                syndicate_name: (*syndicate).to_owned(),
                syndicate_unique_name: format!("/Lotus/Syndicates/{}", syndicate.replace(' ', "")),
                standing_cost: *standing,
                list_price: *list_price,
                status: *status,
                price_history: price_history(id + 5, list_price.unwrap_or(20)),
                updated_at: CHECKED_AT.to_owned(),
                properties: SyndicateItemProperties {
                    min_price: *min_price,
                    min_sma: None,
                    min_profit: PROFIT_OVERRIDE.contains(name).then_some(10),
                },
            }
        })
        .collect()
}

/// The rows of `order_get_pagination`.
pub fn orders() -> Vec<Order> {
    type Seed = (
        &'static str,
        OrderType,
        i64,
        i64,
        Option<(i64, i64)>,
        Option<&'static str>,
        Option<i64>,
    );
    use OrderType::{Buy, Sell};
    const SEEDS: &[Seed] = &[
        ("Arcane Energize", Sell, 289, 1, Some((5, 5)), None, Some(245)),
        ("Arcane Grace", Sell, 172, 1, Some((5, 5)), None, Some(150)),
        ("Molt Augmented", Sell, 46, 2, Some((5, 5)), None, Some(38)),
        ("Primed Sure Footed", Sell, 128, 1, Some((0, 10)), None, Some(110)),
        ("Primed Continuity", Sell, 55, 3, Some((10, 10)), None, Some(42)),
        ("Primed Flow", Sell, 58, 2, Some((10, 10)), None, Some(45)),
        ("Condition Overload", Sell, 16, 4, Some((0, 10)), None, Some(12)),
        ("Blind Rage", Sell, 30, 1, Some((10, 10)), None, Some(22)),
        ("Volt Prime Set", Sell, 62, 1, None, None, Some(48)),
        ("Rhino Prime Set", Sell, 85, 1, None, None, Some(70)),
        ("Saryn Prime Set", Sell, 54, 2, None, None, Some(44)),
        ("Glaive Prime Set", Sell, 131, 1, None, None, Some(105)),
        ("Lex Prime Receiver", Sell, 9, 6, None, None, Some(5)),
        ("Lith G1 Relic", Sell, 6, 12, None, Some("intact"), Some(3)),
        ("Axi S7 Relic", Sell, 14, 5, None, Some("radiant"), Some(8)),
        ("Wisp Prime Set", Buy, 82, 1, None, None, None),
        ("Gauss Prime Set", Buy, 61, 1, None, None, None),
        ("Xaku Prime Set", Buy, 96, 2, None, None, None),
        ("Arcane Energize", Buy, 41, 21, Some((0, 5)), None, None),
        ("Arcane Grace", Buy, 22, 10, Some((0, 5)), None, None),
        ("Primed Sure Footed", Buy, 98, 1, Some((0, 10)), None, None),
        ("Glaive Prime Set", Buy, 118, 1, None, None, None),
        ("Lith G1 Relic", Buy, 3, 12, None, Some("intact"), None),
    ];
    const HIDDEN: &[(&str, OrderType)] = &[("Blind Rage", Sell)];
    SEEDS
        .iter()
        .enumerate()
        .map(
            |(index, (name, order_type, platinum, quantity, rank, subtype, stock_cost))| Order {
                id: format!("6739{:020x}", (index as u64 + 1) * 104_729),
                item_id: format!("5f{:022x}", (index as u64 + 1) * 7919),
                item_name: (*name).to_owned(),
                wfm_url: slug(name),
                order_type: *order_type,
                platinum: *platinum,
                quantity: *quantity,
                per_trade: (*quantity >= 6).then_some(6),
                rank: rank.map(|(rank, _)| rank),
                max_rank: rank.map(|(_, max)| max),
                subtype: subtype.map(str::to_owned),
                visible: !HIDDEN.contains(&(*name, *order_type)),
                created_at: "2026-11-12T18:20:00Z".to_owned(),
                updated_at: format!("2026-11-17T{:02}:{:02}:00Z", 20 + index % 2, (index * 7) % 60),
                stock_cost: *stock_cost,
            },
        )
        .collect()
}

/// The rows of `auction_get_pagination`.
pub fn auctions() -> Vec<Auction> {
    let stock = stock_rivens();
    let listed = ["Rubico", "Torid", "Dual Toxocyst", "Magistar", "Acceltra"];
    let mut auctions: Vec<Auction> = listed
        .iter()
        .enumerate()
        .filter_map(|(index, weapon)| {
            let riven = stock.iter().find(|riven| riven.weapon_name == *weapon)?;
            let price = riven.list_price?;
            Some(Auction {
                id: format!("6738{:020x}", (index as u64 + 1) * 15_485_863),
                weapon_name: riven.weapon_name.clone(),
                wfm_weapon_url: riven.wfm_weapon_url.clone(),
                mod_name: riven.mod_name.clone(),
                attributes: riven.attributes.clone(),
                mastery_level: riven.mastery_rank,
                mod_rank: riven.rank(),
                re_rolls: riven.re_rolls,
                polarity: riven.polarity.clone(),
                starting_price: price,
                buyout_price: Some(price),
                top_bid: None,
                is_direct_sell: true,
                visible: true,
                created: "2026-11-15T10:05:00Z".to_owned(),
                updated: format!("2026-11-17T21:{:02}:00Z", 2 + index * 3),
                can_import: false,
                stock_cost: Some(riven.bought),
            })
        })
        .collect();
    auctions.push(Auction {
        id: format!("6738{:020x}", 6u64 * 15_485_863),
        weapon_name: "Sporelacer".to_owned(),
        wfm_weapon_url: "sporelacer".to_owned(),
        mod_name: "Toxi-visitis".to_owned(),
        attributes: vec![
            attribute("Toxin", 133.5),
            attribute("Multishot", 101.2),
            attribute("Status Chance", 87.4),
        ],
        mastery_level: 12,
        mod_rank: 8,
        re_rolls: 5,
        polarity: "Naramon".to_owned(),
        starting_price: 450,
        buyout_price: Some(600),
        top_bid: Some(480),
        is_direct_sell: false,
        visible: true,
        created: "2026-11-16T19:30:00Z".to_owned(),
        updated: "2026-11-17T20:48:00Z".to_owned(),
        can_import: true,
        stock_cost: None,
    });
    auctions
}

/// The rows of `trade_entry_get_pagination` for every group.
pub fn trade_entries() -> Vec<TradeEntry> {
    type Seed = (
        &'static str,
        TradeGroup,
        Option<i64>,
        i64,
        Option<i64>,
        Option<i64>,
        &'static [&'static str],
    );
    use TradeGroup::{Custom, Item, Riven};
    const SEEDS: &[Seed] = &[
        ("Arcane Energize", Item, Some(5), 289, Some(275), Some(44), &["arcanes"]),
        ("Arcane Grace", Item, Some(5), 172, Some(165), Some(22), &["arcanes"]),
        ("Molt Augmented", Item, Some(5), 46, Some(42), Some(8), &["arcanes"]),
        ("Primed Sure Footed", Item, Some(0), 128, Some(120), Some(18), &["primed mods"]),
        ("Primed Continuity", Item, Some(10), 55, Some(50), Some(13), &["primed mods"]),
        ("Primed Flow", Item, Some(10), 58, Some(54), Some(13), &["primed mods"]),
        ("Volt Prime Set", Item, None, 62, Some(58), Some(14), &["prime sets"]),
        ("Rhino Prime Set", Item, None, 85, Some(80), Some(15), &["prime sets"]),
        ("Glaive Prime Set", Item, None, 131, Some(124), Some(26), &["prime sets", "weapons"]),
        ("Maxed Serration", Custom, None, 25, None, None, &["maxed mods"]),
        ("Maxed Blind Rage", Custom, None, 45, None, None, &["maxed mods"]),
        ("Legendary Core", Custom, None, 1150, None, None, &["rare"]),
        ("Taxi to Steel Path Circuit", Custom, None, 5, None, None, &["services"]),
        ("Rubico", Riven, None, 45, Some(60), Some(15), &["snipers"]),
        ("Torid", Riven, None, 240, Some(320), Some(80), &["incarnon"]),
        ("Burston", Riven, None, 180, Some(240), Some(60), &["incarnon"]),
        ("Dual Toxocyst", Riven, None, 150, Some(200), Some(50), &["incarnon"]),
        ("Magistar", Riven, None, 90, Some(120), Some(30), &["incarnon", "melee"]),
        ("Glaive", Riven, None, 35, Some(45), Some(10), &["melee"]),
    ];
    SEEDS
        .iter()
        .enumerate()
        .map(
            |(index, (name, group, rank, price, min_price, potential_profit, tags))| TradeEntry {
                id: index as i64 + 1,
                wfm_id: format!("5f{:022x}", (index as u64 + 11) * 7919),
                name: (*name).to_owned(),
                sub_type: rank.map(SubType::rank),
                price: *price,
                group: *group,
                tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
                updated_at: format!("2026-11-17T{:02}:{:02}:00Z", 18 + index % 4, (index * 11) % 60),
                properties: TradeEntryProperties {
                    min_price: *min_price,
                    potential_profit: *potential_profit,
                },
            },
        )
        .collect()
}

/// The last hour of riven prices, which "Find rivens" filters.
pub fn riven_candidates() -> Vec<RivenCandidate> {
    const SEEDS: &[(&str, i64, i64)] = &[
        ("Felarx", 280, 14),
        ("Latron", 210, 22),
        ("Braton", 160, 18),
        ("Vectis", 75, 9),
        ("Catchmoon", 55, 11),
        ("Sporelacer", 40, 6),
        ("Torid", 320, 31),
    ];
    SEEDS
        .iter()
        .enumerate()
        .map(|(index, (name, min_price, volume))| RivenCandidate {
            wfm_id: format!("5c{:022x}", (index as u64 + 3) * 6151),
            name: (*name).to_owned(),
            min_price: *min_price,
            volume: *volume,
        })
        .collect()
}
