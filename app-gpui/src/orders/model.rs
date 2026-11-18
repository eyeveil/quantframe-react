//! View models and stores for the Orders screen. Field names follow `Order`
//! and `Auction` in `src/types/warframeMarket.type.ts`, with the item details
//! the backend adds under `properties` flattened in. The stores keep rows in
//! memory behind the read and write functions a backend-backed source
//! replaces.

use crate::listings::model::RivenAttribute;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrderType {
    Sell,
    Buy,
}

impl OrderType {
    pub const ALL: [Self; 2] = [Self::Sell, Self::Buy];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sell => "sell",
            Self::Buy => "buy",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Sell => "Sell",
            Self::Buy => "Buy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

/// A Warframe Market id as a number, for tables that key rows by `u64`.
pub fn row_key(id: &str) -> u64 {
    id.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Order {
    pub id: String,
    pub item_id: String,
    pub item_name: String,
    pub wfm_url: String,
    pub order_type: OrderType,
    pub platinum: i64,
    pub quantity: i64,
    pub per_trade: Option<i64>,
    pub rank: Option<i64>,
    pub max_rank: Option<i64>,
    pub subtype: Option<String>,
    pub visible: bool,
    pub created_at: String,
    pub updated_at: String,
    /// What a unit of the matching stock item cost, when the order came from
    /// stock. The profit estimate uses it.
    pub stock_cost: Option<i64>,
}

impl Order {
    pub fn variant_label(&self) -> Option<String> {
        match (self.rank, self.max_rank, &self.subtype) {
            (Some(rank), Some(max), _) => Some(format!("Rank {rank} of {max}")),
            (Some(rank), None, _) => Some(format!("Rank {rank}")),
            (None, _, Some(subtype)) => {
                let mut chars = subtype.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars).collect())
            }
            (None, _, None) => None,
        }
    }

    pub fn total(&self) -> i64 {
        self.platinum * self.quantity
    }

    /// The estimated profit of a sell order over what its stock cost.
    pub fn profit(&self) -> Option<i64> {
        match self.order_type {
            OrderType::Sell => self
                .stock_cost
                .map(|cost| (self.platinum - cost) * self.quantity),
            OrderType::Buy => None,
        }
    }
}

/// The count, platinum and estimated profit of the orders of one type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OrderTotals {
    pub count: usize,
    pub platinum: i64,
    pub profit: i64,
}

pub struct OrderStore {
    rows: Vec<Order>,
    blacklist: Vec<String>,
}

impl OrderStore {
    pub fn new(rows: Vec<Order>) -> Self {
        Self {
            rows,
            blacklist: Vec::new(),
        }
    }

    pub fn list(&self, text: &str, order_type: Option<OrderType>) -> Vec<Order> {
        let needle = text.trim().to_lowercase();
        self.rows
            .iter()
            .filter(|order| order_type.is_none_or(|kind| order.order_type == kind))
            .filter(|order| needle.is_empty() || order.item_name.to_lowercase().contains(&needle))
            .cloned()
            .collect()
    }

    pub fn get(&self, key: u64) -> Option<&Order> {
        self.rows.iter().find(|order| row_key(&order.id) == key)
    }

    pub fn totals(&self, order_type: OrderType) -> OrderTotals {
        self.rows
            .iter()
            .filter(|order| order.order_type == order_type)
            .fold(OrderTotals::default(), |totals, order| OrderTotals {
                count: totals.count + 1,
                platinum: totals.platinum + order.total(),
                profit: totals.profit + order.profit().unwrap_or(0),
            })
    }

    /// Takes `quantity` units off an order that traded. An order with no
    /// units left closes.
    pub fn traded(&mut self, key: u64, quantity: i64) {
        if let Some(order) = self.rows.iter_mut().find(|order| row_key(&order.id) == key) {
            order.quantity -= quantity.min(order.quantity);
        }
        self.rows.retain(|order| order.quantity > 0);
    }

    pub fn delete(&mut self, key: u64) {
        self.rows.retain(|order| row_key(&order.id) != key);
    }

    /// Deletes every order, or every order of one type. Returns how many went.
    pub fn delete_all(&mut self, order_type: Option<OrderType>) -> usize {
        let before = self.rows.len();
        self.rows
            .retain(|order| order_type.is_some_and(|kind| order.order_type != kind));
        before - self.rows.len()
    }

    /// Removes the order and keeps the live scraper from listing the item again.
    pub fn blacklist(&mut self, key: u64) {
        if let Some(order) = self.get(key) {
            self.blacklist.push(order.wfm_url.clone());
        }
        self.delete(key);
    }

    pub fn blacklisted(&self) -> &[String] {
        &self.blacklist
    }
}

/// A riven auction on Warframe Market.
#[derive(Clone, Debug, PartialEq)]
pub struct Auction {
    pub id: String,
    pub weapon_name: String,
    pub wfm_weapon_url: String,
    pub mod_name: String,
    pub attributes: Vec<RivenAttribute>,
    pub mastery_level: i64,
    pub mod_rank: i64,
    pub re_rolls: i64,
    pub polarity: String,
    pub starting_price: i64,
    pub buyout_price: Option<i64>,
    pub top_bid: Option<i64>,
    pub is_direct_sell: bool,
    pub visible: bool,
    pub created: String,
    pub updated: String,
    /// True while no stock riven matches the auction, so it can be imported.
    pub can_import: bool,
    /// What the matching stock riven cost. The profit estimate uses it.
    pub stock_cost: Option<i64>,
}

impl Auction {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.weapon_name, self.mod_name)
    }

    pub fn profit(&self) -> Option<i64> {
        self.stock_cost.map(|cost| self.starting_price - cost)
    }
}

pub struct AuctionStore {
    rows: Vec<Auction>,
}

impl AuctionStore {
    pub fn new(rows: Vec<Auction>) -> Self {
        Self { rows }
    }

    pub fn list(&self, text: &str) -> Vec<Auction> {
        let needle = text.trim().to_lowercase();
        self.rows
            .iter()
            .filter(|auction| {
                needle.is_empty() || auction.full_name().to_lowercase().contains(&needle)
            })
            .cloned()
            .collect()
    }

    pub fn get(&self, key: u64) -> Option<&Auction> {
        self.rows.iter().find(|auction| row_key(&auction.id) == key)
    }

    /// The count, starting prices and estimated profit of every auction.
    pub fn overview(&self) -> (usize, i64, i64) {
        (
            self.rows.len(),
            self.rows.iter().map(|auction| auction.starting_price).sum(),
            self.rows.iter().filter_map(Auction::profit).sum(),
        )
    }

    pub fn delete(&mut self, key: u64) {
        self.rows.retain(|auction| row_key(&auction.id) != key);
    }

    pub fn delete_all(&mut self) -> usize {
        let count = self.rows.len();
        self.rows.clear();
        count
    }

    /// Records the auction's riven as stock bought for `bought` platinum.
    pub fn import(&mut self, key: u64, bought: i64) {
        if let Some(auction) = self
            .rows
            .iter_mut()
            .find(|auction| row_key(&auction.id) == key)
        {
            auction.can_import = false;
            auction.stock_cost = Some(bought);
        }
    }
}
