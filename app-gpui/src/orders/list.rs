//! The Orders tab: buy and sell orders on Warframe Market.

use std::{cmp::Ordering, collections::BTreeMap};

use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement, Styled, div, px};
use crate::ui::numeric;
use gpui_kit_assets::IconName;
use serde_json::json;

use super::row_key;
use crate::{
    data::{DataKind, DataSource},
    model::{
        ListParams, NewStockItem, OrderListParams, OrderTotals, OrderType, SubType, WfmOrder,
    },
    theme::Theme,
    ui::{
        IconButton, cells, day_and_time,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ChipSpec, ColumnSpec, Emit, Load, MenuEntry, Outcome,
            PrimaryAction, Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number,
        panel::{Fact, PanelAction, PanelModel},
        prompts, row_button,
    },
};

const DELETE_CHOICES: [&str; 3] = ["All orders", "Sell orders", "Buy orders"];
/// The live scraper modes a blacklisted item is kept out of.
const BLACKLIST_MODES: [&str; 3] = ["buy", "sell", "wishlist"];

#[derive(Clone, Debug)]
pub enum OrderAction {
    Refresh,
    DeleteAll,
    DeletedAll(Option<OrderType>),
    Trade(u64),
    Traded { key: u64, quantity: i64, total: i64 },
    TradeAtPrice(u64),
    Blacklist(u64),
    Blacklisted(u64),
    Delete(u64),
    Deleted(u64),
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
}

#[derive(Default)]
pub struct OrdersData {
    rows: Vec<WfmOrder>,
    totals: BTreeMap<OrderType, OrderTotals>,
    order_limit: i64,
}

#[derive(Default)]
pub struct Orders {
    data: OrdersData,
}

const fn type_key(order_type: OrderType) -> &'static str {
    match order_type {
        OrderType::Sell => "sell",
        OrderType::Buy => "buy",
    }
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

const fn at_price_label(order_type: OrderType) -> &'static str {
    match order_type {
        OrderType::Sell => "Sold one at the order price",
        OrderType::Buy => "Bought one at the order price",
    }
}

fn variant(order: &WfmOrder) -> Option<String> {
    order.sub_type.as_ref().and_then(SubType::label)
}

fn total(order: &WfmOrder) -> i64 {
    order.platinum * order.quantity
}

/// The estimated profit of every unit of the order, as the totals count it.
fn profit(order: &WfmOrder) -> Option<i64> {
    order.potential_profit.map(|profit| profit * order.quantity)
}

impl Orders {
    fn order(&self, key: u64) -> Option<WfmOrder> {
        self.data
            .rows
            .iter()
            .find(|order| row_key(&order.id) == key)
            .cloned()
    }

    fn totals(&self, order_type: OrderType) -> OrderTotals {
        self.data
            .totals
            .get(&order_type)
            .cloned()
            .unwrap_or_default()
    }
}

impl Catalog for Orders {
    type Row = WfmOrder;
    type Action = OrderAction;
    type Data = OrdersData;

    const NOUN: &'static str = "orders";
    const SEARCH_PLACEHOLDER: &'static str = "Search orders";
    const EMPTY_HINT: &'static str =
        "Orders appear here once the live scraper lists your stock and wish list.";
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[DataKind::WfmOrders, DataKind::User];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let rows = source.get_wfm_orders_pagination(&OrderListParams {
            list: ListParams {
                limit: -1,
                ..ListParams::default()
            },
            order_type: None,
        });
        let totals = source.get_wfm_orders_status_counts();
        let user = source.user();
        Box::pin(async move {
            Ok(OrdersData {
                rows: rows.await?.results,
                totals: totals.await?,
                order_limit: user.await?.order_limit,
            })
        })
    }

    fn apply(&mut self, data: Self::Data) {
        self.data = data;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("item", "Item", 200.).flex().sortable(),
            ColumnSpec::new("type", "Type", 96.).sortable(),
            ColumnSpec::new("rank", "Rank", 120.),
            ColumnSpec::new("quantity", "Quantity", 92.).end().sortable(),
            ColumnSpec::new("platinum", "Price", 92.).end().sortable(),
            ColumnSpec::new("total", "Total", 96.)
                .end()
                .sortable()
                .droppable(1),
            ColumnSpec::new("profit", "Est. profit", 104.)
                .end()
                .sortable()
                .droppable(3),
            ColumnSpec::new("updated", "Updated", 140.)
                .sortable()
                .droppable(2),
            ColumnSpec::new("actions", "", 160.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<WfmOrder> {
        let kind = query.filter("type");
        self.data
            .rows
            .iter()
            .filter(|order| kind.is_none_or(|kind| type_key(order.order_type) == kind))
            .filter(|order| query.matches(&order.item_name))
            .cloned()
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        let theme = Theme::global(cx);
        vec![ChipGroup {
            id: "type",
            chips: [OrderType::Sell, OrderType::Buy]
                .into_iter()
                .map(|order_type| (order_type, self.totals(order_type)))
                .filter(|(_, totals)| totals.count > 0)
                .map(|(order_type, totals)| ChipSpec {
                    key: type_key(order_type).to_owned(),
                    label: format!("{} orders", order_type.label()),
                    count: totals.count,
                    color: Some(type_color(order_type, theme)),
                    tooltip: format!("{} platinum in total", number(totals.platinum)),
                })
                .collect(),
        }]
    }

    fn summary(&self) -> Vec<Stat> {
        let sell = self.totals(OrderType::Sell);
        let buy = self.totals(OrderType::Buy);
        let profit = sell.potential_profit.round() as i64;
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
                value: number(profit),
                note: "if every sell order fills".to_owned(),
                tone: Tone::of_profit(profit),
            },
            Stat {
                label: "Orders",
                value: number(self.data.rows.len() as i64),
                note: format!("of {} allowed", number(self.data.order_limit)),
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

    fn perform(&mut self, action: OrderAction, source: &DataSource) -> Outcome<OrderAction> {
        match action {
            OrderAction::Refresh => Outcome::run(
                source.order_refresh(),
                "Refreshed the orders from Warframe Market.",
            ),
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
            OrderAction::DeletedAll(order_type) => Outcome::run(
                source.order_delete_all(order_type),
                "Deleted the orders from Warframe Market.",
            ),
            OrderAction::Trade(key) => match self.order(key) {
                Some(order) => {
                    let spec = match order.order_type {
                        OrderType::Sell => {
                            prompts::sold(&order.item_name, Some(order.quantity), Some(total(&order)))
                        }
                        OrderType::Buy => prompts::bought(
                            &order.item_name,
                            Some(order.quantity),
                            Some(total(&order)),
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
                Some(order) => self.perform(
                    OrderAction::Traded {
                        key,
                        quantity: 1,
                        total: order.platinum,
                    },
                    source,
                ),
                None => Outcome::Done,
            },
            OrderAction::Traded {
                key,
                quantity,
                total,
            } => match self.order(key) {
                Some(order) => {
                    let notice = format!(
                        "{} {} {} for {} platinum.",
                        traded_word(order.order_type),
                        number(quantity),
                        order.item_name,
                        number(total)
                    );
                    // A filled sell order takes the units out of stock, and a
                    // filled buy order adds them to stock at the price paid.
                    match order.order_type {
                        OrderType::Sell => Outcome::run(
                            source.stock_item_sell(order.item_url, order.sub_type, quantity, total),
                            notice,
                        ),
                        OrderType::Buy => Outcome::run(
                            source.stock_item_create(NewStockItem {
                                raw: order.item_id,
                                sub_type: order.sub_type,
                                quantity,
                                bought: total,
                            }),
                            notice,
                        ),
                    }
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
            OrderAction::Blacklisted(key) => match self.order(key) {
                Some(order) => {
                    let source = source.clone();
                    Outcome::Run(Box::pin(async move {
                        let settings = source.app_get_settings().await?;
                        let mut tree = serde_json::to_value(&settings)?;
                        let blacklist = &mut tree["live_scraper"]["items"]["general"]["blacklist"];
                        let mut entries = blacklist.as_array().cloned().unwrap_or_default();
                        entries.retain(|entry| entry["wfmId"] != json!(order.item_id));
                        entries.push(json!({
                            "wfmId": order.item_id,
                            "disabled_for": BLACKLIST_MODES,
                        }));
                        *blacklist = json!(entries);
                        source
                            .app_update_settings(serde_json::from_value(tree)?)
                            .await?;
                        source.order_delete_by_id(order.id).await?;
                        Ok(format!("Blacklisted {}.", order.item_name))
                    }))
                }
                None => Outcome::Done,
            },
            OrderAction::Delete(key) => Outcome::ask(
                prompts::delete(1, "order", "orders"),
                move |_| Some(OrderAction::Deleted(key)),
            ),
            OrderAction::Deleted(key) => match self.order(key) {
                Some(order) => Outcome::run_quietly(source.order_delete_by_id(order.id)),
                None => Outcome::Done,
            },
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
                    order.item_url
                )),
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &WfmOrder) -> u64 {
        row_key(&row.id)
    }

    fn compare(key: &str, a: &WfmOrder, b: &WfmOrder) -> Ordering {
        match key {
            "type" => a.order_type.cmp(&b.order_type),
            "quantity" => a.quantity.cmp(&b.quantity),
            "platinum" => a.platinum.cmp(&b.platinum),
            "total" => total(a).cmp(&total(b)),
            "profit" => profit(a).cmp(&profit(b)),
            "updated" => a.updated_at.cmp(&b.updated_at),
            _ => a.item_name.cmp(&b.item_name),
        }
    }

    fn cell(row: &WfmOrder, key: &str, emit: &Emit<OrderAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let id = row_key(&row.id);
        match key {
            "item" => {
                let mut flags = Vec::new();
                if !row.visible {
                    flags.push((IconName::EyeOff, "Hidden on Warframe Market"));
                }
                if row.per_trade.is_some_and(|units| units > 1) {
                    flags.push((IconName::Handshake, "Trades in batches"));
                }
                cells::name(row.item_name.clone(), &flags, cx)
            }
            "type" => cells::status(type_color(row.order_type, theme), row.order_type.label(), cx),
            "rank" => cells::text(variant(row).unwrap_or_default()),
            "quantity" => cells::amount(Some(row.quantity), "", cx),
            "platinum" => cells::amount(Some(row.platinum), "", cx),
            "total" => cells::amount(Some(total(row)), "", cx),
            "profit" => match profit(row) {
                Some(profit) => numeric(number(profit))
                    .text_color(Tone::of_profit(profit).color(theme))
                    .into_any_element(),
                None => cells::amount(None, "No price data", cx),
            },
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

    fn cell_text(row: &WfmOrder, key: &str) -> Option<String> {
        match key {
            "item" => Some(row.item_name.clone()),
            "type" => Some(row.order_type.label().to_owned()),
            "rank" => variant(row),
            "quantity" => Some(number(row.quantity)),
            "platinum" => Some(number(row.platinum)),
            "total" => Some(number(total(row))),
            "profit" => profit(row).map(number),
            "updated" => Some(day_and_time(&row.updated_at)),
            _ => None,
        }
    }

    fn row_menu(row: &WfmOrder) -> Vec<MenuEntry<OrderAction>> {
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

    fn panel(row: &WfmOrder, cx: &App) -> PanelModel<OrderAction> {
        let theme = Theme::global(cx);
        let id = row_key(&row.id);
        PanelModel {
            title: row.item_name.clone(),
            subtitle: match variant(row) {
                Some(variant) => format!("{variant}, {}", row.item_url),
                None => row.item_url.clone(),
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
                Fact::new("Total", number(total(row))),
                match row.potential_profit {
                    Some(profit) => Fact::new("Estimated profit per unit", number(profit))
                        .tone(Tone::of_profit(profit)),
                    None => Fact::new("Estimated profit per unit", "No price data")
                        .tone(Tone::Muted),
                },
                match row.closed_avg {
                    Some(average) => Fact::new("Closing average", format!("{average:.1}")),
                    None => Fact::new("Closing average", "No price data").tone(Tone::Muted),
                },
                Fact::amount("Units per trade", row.per_trade, "One"),
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

    fn dimmed(row: &WfmOrder) -> bool {
        !row.visible
    }
}
