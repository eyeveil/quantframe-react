//! The Orders tab: buy and sell orders on Warframe Market.

use std::cmp::Ordering;

use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement, Styled, div, px};
use gpui_kit_assets::IconName;

use super::{
    REFRESH_NOT_CONNECTED,
    model::{Order, OrderStore, OrderType, row_key},
};
use crate::{
    theme::Theme,
    ui::{
        IconButton, cells, day_and_time,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ChipSpec, ColumnSpec, Emit, MenuEntry, Outcome, PrimaryAction,
            Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number,
        panel::{Fact, PanelAction, PanelModel},
        prompts, row_button,
    },
};

const DELETE_CHOICES: [&str; 3] = ["All orders", "Sell orders", "Buy orders"];

#[derive(Clone, Debug)]
pub enum OrderAction {
    Refresh,
    DeleteAll,
    DeletedAll(Option<OrderType>),
    Trade(u64),
    Traded {
        key: u64,
        quantity: i64,
        total: i64,
    },
    TradeAtPrice(u64),
    Blacklist(u64),
    Blacklisted(u64),
    Delete(u64),
    Deleted(u64),
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
}

pub struct Orders {
    store: OrderStore,
    order_limit: i64,
}

fn type_color(order_type: OrderType, theme: &Theme) -> Hsla {
    match order_type {
        OrderType::Sell => theme.colors.positive,
        OrderType::Buy => theme.colors.info,
    }
}

/// The word for an order that traded: a sell order sold, a buy order bought.
const fn traded_word(order_type: OrderType) -> &'static str {
    match order_type {
        OrderType::Sell => "Sold",
        OrderType::Buy => "Bought",
    }
}

fn at_price_label(order_type: OrderType) -> &'static str {
    match order_type {
        OrderType::Sell => "Sold one at the order price",
        OrderType::Buy => "Bought one at the order price",
    }
}

impl Orders {
    pub fn new(rows: Vec<Order>, order_limit: i64) -> Self {
        Self {
            store: OrderStore::new(rows),
            order_limit,
        }
    }

    fn order(&self, key: u64) -> Option<Order> {
        self.store.get(key).cloned()
    }
}

impl Catalog for Orders {
    type Row = Order;
    type Action = OrderAction;

    const NOUN: &'static str = "orders";
    const SEARCH_PLACEHOLDER: &'static str = "Search orders";
    const EMPTY_HINT: &'static str =
        "Orders appear here once the live scraper lists your stock and wish list.";
    const CHECKABLE: bool = false;

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("item", "Item", 200.).flex().sortable(),
            ColumnSpec::new("type", "Type", 92.).sortable(),
            ColumnSpec::new("rank", "Rank", 132.),
            ColumnSpec::new("quantity", "Quantity", 88.).end().sortable(),
            ColumnSpec::new("platinum", "Price", 88.).end().sortable(),
            ColumnSpec::new("total", "Total", 92.).end().sortable().droppable(2),
            ColumnSpec::new("profit", "Est. profit", 100.).end().sortable().droppable(1),
            ColumnSpec::new("updated", "Updated", 132.).sortable().droppable(3),
            ColumnSpec::new("actions", "", 156.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<Order> {
        self.store
            .list(&query.text, query.filter("type").and_then(OrderType::parse))
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        let theme = Theme::global(cx);
        vec![ChipGroup {
            id: "type",
            chips: OrderType::ALL
                .into_iter()
                .map(|order_type| (order_type, self.store.totals(order_type)))
                .filter(|(_, totals)| totals.count > 0)
                .map(|(order_type, totals)| ChipSpec {
                    key: order_type.as_str().to_owned(),
                    label: format!("{} orders", order_type.label()),
                    count: totals.count,
                    color: Some(type_color(order_type, theme)),
                    tooltip: format!("{} platinum in total", number(totals.platinum)),
                })
                .collect(),
        }]
    }

    fn summary(&self) -> Vec<Stat> {
        let sell = self.store.totals(OrderType::Sell);
        let buy = self.store.totals(OrderType::Buy);
        let open = sell.count + buy.count;
        vec![
            Stat {
                label: "Selling",
                value: number(sell.platinum),
                note: format!("{} sell orders", sell.count),
                tone: Tone::Plain,
            },
            Stat {
                label: "Buying",
                value: number(buy.platinum),
                note: format!("{} buy orders", buy.count),
                tone: Tone::Plain,
            },
            Stat {
                label: "Estimated profit",
                value: number(sell.profit),
                note: "if every sell order fills".to_owned(),
                tone: Tone::of_profit(sell.profit),
            },
            Stat {
                label: "Orders",
                value: number(open as i64),
                note: format!("of {} allowed", number(self.order_limit)),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<OrderAction>> {
        vec![
            ToolbarItem {
                id: "refresh",
                icon: IconName::RefreshCw,
                tooltip: "Refresh the orders from Warframe Market".to_owned(),
                action: Some(OrderAction::Refresh),
                danger: false,
            },
            ToolbarItem {
                id: "delete-all",
                icon: IconName::Trash,
                tooltip: "Delete orders from Warframe Market".to_owned(),
                action: Some(OrderAction::DeleteAll),
                danger: true,
            },
        ]
    }

    fn primary(&self) -> Option<PrimaryAction<OrderAction>> {
        None
    }

    fn perform(&mut self, action: OrderAction) -> Outcome<OrderAction> {
        match action {
            OrderAction::Refresh => Outcome::Notice(REFRESH_NOT_CONNECTED.to_owned()),
            OrderAction::DeleteAll => {
                let mut spec = FormSpec::new("Delete orders", "Delete")
                    .description(
                        "This removes the orders from Warframe Market. Your stock and wish list stay as they are.",
                    )
                    .field(Field::choice(
                        "which",
                        "Orders to delete",
                        DELETE_CHOICES.map(str::to_owned).to_vec(),
                        Some(0),
                    ));
                spec.danger = true;
                Outcome::ask(spec, |values| {
                    let order_type = match values.choice("which")?.as_str() {
                        "Sell orders" => Some(OrderType::Sell),
                        "Buy orders" => Some(OrderType::Buy),
                        _ => None,
                    };
                    Some(OrderAction::DeletedAll(order_type))
                })
            }
            OrderAction::DeletedAll(order_type) => {
                let count = self.store.delete_all(order_type);
                Outcome::Notice(format!("Deleted {count} orders."))
            }
            OrderAction::Trade(key) => match self.order(key) {
                Some(order) => {
                    let spec = match order.order_type {
                        OrderType::Sell => {
                            prompts::sold(&order.item_name, Some(order.quantity), Some(order.total()))
                        }
                        OrderType::Buy => prompts::bought(
                            &order.item_name,
                            Some(order.quantity),
                            Some(order.total()),
                        ),
                    };
                    Outcome::ask(spec, move |values| {
                        Some(OrderAction::Traded {
                            key,
                            quantity: values.number("quantity")?,
                            total: values.number("price")?,
                        })
                    })
                }
                None => Outcome::Done,
            },
            OrderAction::TradeAtPrice(key) => match self.order(key) {
                Some(order) => self.perform(OrderAction::Traded {
                    key,
                    quantity: 1,
                    total: order.platinum,
                }),
                None => Outcome::Done,
            },
            OrderAction::Traded {
                key,
                quantity,
                total,
            } => match self.order(key) {
                Some(order) => {
                    let quantity = quantity.min(order.quantity);
                    self.store.traded(key, quantity);
                    Outcome::Notice(format!(
                        "{} {} {} for {} platinum.",
                        traded_word(order.order_type),
                        number(quantity),
                        order.item_name,
                        number(total)
                    ))
                }
                None => Outcome::Done,
            },
            OrderAction::Blacklist(key) => match self.order(key) {
                Some(order) => Outcome::ask(
                    FormSpec::confirm_danger(
                        format!("Blacklist {}?", order.item_name),
                        "This removes the order from Warframe Market and keeps the live scraper from listing the item again. You can take it off the blacklist in Settings.",
                        "Blacklist",
                    ),
                    move |_| Some(OrderAction::Blacklisted(key)),
                ),
                None => Outcome::Done,
            },
            OrderAction::Blacklisted(key) => {
                self.store.blacklist(key);
                Outcome::Notice(format!(
                    "{} items are on the blacklist.",
                    self.store.blacklisted().len()
                ))
            }
            OrderAction::Delete(key) => Outcome::ask(
                prompts::delete(1, "order", "orders"),
                move |_| Some(OrderAction::Deleted(key)),
            ),
            OrderAction::Deleted(key) => {
                self.store.delete(key);
                Outcome::Done
            }
            OrderAction::Details(key) => Outcome::ShowPanel(key),
            OrderAction::CopyName(key) => match self.order(key) {
                Some(order) => Outcome::Copy {
                    what: order.item_name.clone(),
                    text: order.item_name,
                },
                None => Outcome::Done,
            },
            OrderAction::OpenMarket(key) => match self.order(key) {
                Some(order) => Outcome::OpenUrl(format!(
                    "https://warframe.market/items/{}",
                    order.wfm_url
                )),
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &Order) -> u64 {
        row_key(&row.id)
    }

    fn compare(key: &str, a: &Order, b: &Order) -> Ordering {
        match key {
            "type" => a.order_type.cmp(&b.order_type),
            "quantity" => a.quantity.cmp(&b.quantity),
            "platinum" => a.platinum.cmp(&b.platinum),
            "total" => a.total().cmp(&b.total()),
            "profit" => a.profit().cmp(&b.profit()),
            "updated" => a.updated_at.cmp(&b.updated_at),
            _ => a.item_name.cmp(&b.item_name),
        }
    }

    fn cell(row: &Order, key: &str, emit: &Emit<OrderAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let id = row_key(&row.id);
        match key {
            "item" => {
                let mut flags = Vec::new();
                if !row.visible {
                    flags.push((IconName::EyeOff, "Hidden on Warframe Market"));
                }
                if row.per_trade.is_some() {
                    flags.push((IconName::Handshake, "Trades in batches"));
                }
                cells::name(row.item_name.clone(), &flags, cx)
            }
            "type" => cells::status(type_color(row.order_type, theme), row.order_type.label(), cx),
            "rank" => cells::text(row.variant_label().unwrap_or_default()),
            "quantity" => cells::amount(Some(row.quantity), "", cx),
            "platinum" => cells::amount(Some(row.platinum), "", cx),
            "total" => cells::amount(Some(row.total()), "", cx),
            "profit" => {
                let profit = row.profit();
                cells::toned_amount(profit, "", Tone::of_profit(profit.unwrap_or(0)), cx)
            }
            "updated" => cells::text(day_and_time(&row.updated_at)),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(
                        div().pr(px(5.)).child(
                            row_button(("trade", id), traded_word(row.order_type), cx)
                                .on_click(emit.on_click(OrderAction::Trade(id))),
                        ),
                    )
                    .child(
                        IconButton::new(
                            ("trade-price", id),
                            IconName::ShoppingCart,
                            at_price_label(row.order_type),
                        )
                        .in_row()
                        .on_click(emit.on_click(OrderAction::TradeAtPrice(id))),
                    )
                    .child(
                        IconButton::new(("delete", id), IconName::Trash, "Delete this order")
                            .in_row()
                            .hover_tone(theme.colors.negative)
                            .on_click(emit.on_click(OrderAction::Delete(id))),
                    )
                    .child(overflow_button(
                        id,
                        move || Self::row_menu(&menu_row),
                        emit,
                    ))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &Order, key: &str) -> Option<String> {
        match key {
            "item" => Some(row.item_name.clone()),
            "type" => Some(row.order_type.label().to_owned()),
            "rank" => row.variant_label(),
            "quantity" => Some(number(row.quantity)),
            "platinum" => Some(number(row.platinum)),
            "total" => Some(number(row.total())),
            "profit" => row.profit().map(number),
            "updated" => Some(day_and_time(&row.updated_at)),
            _ => None,
        }
    }

    fn row_menu(row: &Order) -> Vec<MenuEntry<OrderAction>> {
        let id = row_key(&row.id);
        vec![
            MenuEntry::new(
                traded_word(row.order_type),
                IconName::HandCoins,
                OrderAction::Trade(id),
            ),
            MenuEntry::new(
                at_price_label(row.order_type),
                IconName::ShoppingCart,
                OrderAction::TradeAtPrice(id),
            ),
            MenuEntry::new("Show details", IconName::PanelRight, OrderAction::Details(id)),
            MenuEntry::new("Copy name", IconName::Copy, OrderAction::CopyName(id)).separated(),
            MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                OrderAction::OpenMarket(id),
            ),
            MenuEntry::new("Blacklist item", IconName::Ban, OrderAction::Blacklist(id)).separated(),
            MenuEntry::new("Delete", IconName::Trash, OrderAction::Delete(id)),
        ]
    }

    fn panel(row: &Order, cx: &App) -> PanelModel<OrderAction> {
        let theme = Theme::global(cx);
        let id = row_key(&row.id);
        let profit = row.profit();
        PanelModel {
            title: row.item_name.clone(),
            subtitle: match row.variant_label() {
                Some(variant) => format!("{variant}, {}", row.wfm_url),
                None => row.wfm_url.clone(),
            },
            status: Some((
                format!("{} order", row.order_type.label()),
                type_color(row.order_type, theme),
            )),
            note: Some(
                if row.visible {
                    "Visible on Warframe Market"
                } else {
                    "Hidden on Warframe Market"
                }
                .to_owned(),
            ),
            history: Vec::new(),
            lines: None,
            facts: vec![
                Fact::new("Price", number(row.platinum)),
                Fact::new("Quantity", number(row.quantity)),
                Fact::new("Total", number(row.total())),
                Fact::amount("Units per trade", row.per_trade, "One"),
                Fact::amount("Stock cost per unit", row.stock_cost, "Not from stock"),
                match profit {
                    Some(profit) => {
                        Fact::new("Estimated profit", number(profit)).tone(Tone::of_profit(profit))
                    }
                    None => Fact::new("Estimated profit", "None to show").tone(Tone::Muted),
                },
                Fact::new("Created", day_and_time(&row.created_at)),
                Fact::new("Updated", day_and_time(&row.updated_at)),
            ],
            primary: vec![
                PanelAction::new(
                    traded_word(row.order_type),
                    IconName::HandCoins,
                    OrderAction::Trade(id),
                ),
                PanelAction::new(
                    at_price_label(row.order_type),
                    IconName::ShoppingCart,
                    OrderAction::TradeAtPrice(id),
                ),
            ],
            secondary: vec![
                PanelAction::new("Copy name", IconName::Copy, OrderAction::CopyName(id)),
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    OrderAction::OpenMarket(id),
                ),
                PanelAction::new("Blacklist item", IconName::Ban, OrderAction::Blacklist(id)),
                PanelAction::new("Delete", IconName::Trash, OrderAction::Delete(id)).danger(),
            ],
        }
    }

    fn dimmed(row: &Order) -> bool {
        !row.visible
    }
}
