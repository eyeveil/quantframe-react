use std::cmp::Ordering;

use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement};
use gpui_kit_assets::IconName;

use super::row_key;
use crate::{
    data::{DataKind, DataSource},
    model::{IntendedAction, IntendedOrder, SubType},
    theme::Theme,
    ui::{
        cells, counted, day_and_time,
        form::FormSpec,
        grid::{
            Catalog, ChipGroup, ChipSpec, ColumnSpec, Emit, Load, MenuEntry, Outcome,
            PrimaryAction, Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number,
        panel::{Fact, PanelAction, PanelModel},
    },
};

const ACTIONS: [IntendedAction; 3] = [
    IntendedAction::Create,
    IntendedAction::Update,
    IntendedAction::Delete,
];

#[derive(Clone, Debug)]
pub enum IntendedOrderAction {
    Refresh,
    Clear,
    Cleared,
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
}

#[derive(Default)]
pub struct IntendedData {
    rows: Vec<IntendedOrder>,
    dry_run: bool,
}

#[derive(Default)]
pub struct IntendedOrders {
    data: IntendedData,
}

fn action_color(action: IntendedAction, theme: &Theme) -> Hsla {
    match action {
        IntendedAction::Create => theme.colors.positive,
        IntendedAction::Update => theme.colors.info,
        IntendedAction::Delete => theme.colors.negative,
    }
}

fn kind_label(row: &IntendedOrder) -> String {
    let side = row.order_type.map(|kind| kind.label()).unwrap_or("Any");
    if row.is_auction {
        "Auction".to_owned()
    } else {
        format!("{side} order")
    }
}

fn item_label(row: &IntendedOrder) -> String {
    match row.sub_type.as_ref().and_then(SubType::label) {
        Some(variant) => format!("{}, {variant}", row.item_name),
        None => row.item_name.clone(),
    }
}

fn reason_text(code: &str) -> &str {
    match code {
        "MaxPriceDrop" => "Held by the largest price change",
        "LowProfit" => "Raised to the lowest profit",
        "MinimumPrice" => "Raised to the minimum price",
        "SMALimit" => "Held near the closing average",
        "NoSellers" => "No other sellers",
        "NoBuyers" => "No other buyers",
        "MaxStockQuantity" => "Stock reached the highest quantity",
        "Overpriced" => "Above the average price",
        "LowMargin" => "Too little margin to the lowest sell price",
        "AboveMaxBuyPrice" => "Capped at the item's highest price",
        "AboveAvgPrice" => "Above the highest average price",
        "MaxPrice" => "Capped at the highest price",
        "MinPrice" => "Raised to the lowest price",
        "Skip" => "Left out of the buy budget",
        other => other,
    }
}

fn reason(row: &IntendedOrder) -> String {
    row.reason
        .split(", ")
        .map(reason_text)
        .collect::<Vec<_>>()
        .join(", ")
}

fn row_id(row: &IntendedOrder) -> u64 {
    row_key(&row.id.to_string())
}

impl IntendedOrders {
    fn row(&self, key: u64) -> Option<IntendedOrder> {
        self.data.rows.iter().find(|row| row_id(row) == key).cloned()
    }

    fn count(&self, action: IntendedAction) -> usize {
        self.data
            .rows
            .iter()
            .filter(|row| row.action == action)
            .count()
    }
}

impl Catalog for IntendedOrders {
    type Row = IntendedOrder;
    type Action = IntendedOrderAction;
    type Data = IntendedData;

    const NOUN: &'static str = "intended orders";
    const SEARCH_PLACEHOLDER: &'static str = "Search intended orders";
    const EMPTY_HINT: &'static str = "With dry run on in Settings, the orders the live scraper would create, change or delete appear here. It sends none of them.";
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[DataKind::IntendedOrders, DataKind::Settings];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let rows = source.live_scraper_get_intended_orders();
        let settings = source.app_get_settings();
        Box::pin(async move {
            Ok(IntendedData {
                rows: rows.await?,
                dry_run: settings.await?.live_scraper.general.dry_run,
            })
        })
    }

    fn apply(&mut self, data: Self::Data) {
        self.data = data;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("time", "Time", 130.).sortable(),
            ColumnSpec::new("action", "Action", 100.).sortable(),
            ColumnSpec::new("item", "Item", 200.).flex().sortable(),
            ColumnSpec::new("kind", "Order", 110.).sortable().droppable(2),
            ColumnSpec::new("price", "Price", 84.).end().sortable(),
            ColumnSpec::new("old_price", "Was", 84.).end().droppable(1),
            ColumnSpec::new("quantity", "Quantity", 92.)
                .end()
                .droppable(3),
            ColumnSpec::new("reason", "Reason", 220.).droppable(1),
            ColumnSpec::new("actions", "", 52.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<IntendedOrder> {
        let action = query.filter("action");
        self.data
            .rows
            .iter()
            .filter(|row| action.is_none_or(|action| row.action.key() == action))
            .filter(|row| query.matches(&row.item_name) || query.matches(&reason(row)))
            .cloned()
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        let theme = Theme::global(cx);
        vec![ChipGroup {
            id: "action",
            chips: ACTIONS
                .into_iter()
                .map(|action| (action, self.count(action)))
                .filter(|(_, count)| *count > 0)
                .map(|(action, count)| ChipSpec {
                    key: action.key().to_owned(),
                    label: action.label().to_owned(),
                    count,
                    color: Some(action_color(action, theme)),
                    tooltip: format!("{} not sent", counted(count as i64, "order", "orders")),
                })
                .collect(),
        }]
    }

    fn summary(&self) -> Vec<Stat> {
        let count = |action| self.count(action) as i64;
        vec![
            Stat {
                label: "Dry run",
                value: if self.data.dry_run { "On" } else { "Off" }.to_owned(),
                note: if self.data.dry_run {
                    "no order reaches warframe.market".to_owned()
                } else {
                    "orders go to warframe.market".to_owned()
                },
                tone: if self.data.dry_run {
                    Tone::Positive
                } else {
                    Tone::Muted
                },
            },
            Stat {
                label: "Create",
                value: number(count(IntendedAction::Create)),
                note: "new orders".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Update",
                value: number(count(IntendedAction::Update)),
                note: "price or quantity changes".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Delete",
                value: number(count(IntendedAction::Delete)),
                note: "orders taken down".to_owned(),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<IntendedOrderAction>> {
        vec![
            ToolbarItem {
                id: "refresh",
                icon: IconName::RefreshCw,
                tooltip: "Reload the intended orders".to_owned(),
                action: Some(IntendedOrderAction::Refresh),
                danger: false,
            },
            ToolbarItem {
                id: "clear",
                icon: IconName::Trash,
                tooltip: "Clear the intended orders".to_owned(),
                action: (!self.data.rows.is_empty()).then_some(IntendedOrderAction::Clear),
                danger: true,
            },
        ]
    }

    fn primary(&self) -> Option<PrimaryAction<IntendedOrderAction>> {
        None
    }

    fn perform(
        &mut self,
        action: IntendedOrderAction,
        source: &DataSource,
    ) -> Outcome<IntendedOrderAction> {
        match action {
            IntendedOrderAction::Refresh => {
                Outcome::run_quietly(source.live_scraper_get_intended_orders())
            }
            IntendedOrderAction::Clear => Outcome::ask(
                FormSpec::confirm_danger(
                    "Clear the intended orders?",
                    "This empties the list. It changes nothing on warframe.market.",
                    "Clear",
                ),
                |_| Some(IntendedOrderAction::Cleared),
            ),
            IntendedOrderAction::Cleared => Outcome::run(
                source.live_scraper_clear_intended_orders(),
                "Cleared the intended orders.",
            ),
            IntendedOrderAction::Details(key) => Outcome::ShowPanel(key),
            IntendedOrderAction::CopyName(key) => match self.row(key) {
                Some(row) => Outcome::Copy {
                    what: row.item_name.clone(),
                    text: row.item_name,
                },
                None => Outcome::Done,
            },
            IntendedOrderAction::OpenMarket(key) => match self.row(key) {
                Some(row) if !row.wfm_url.is_empty() => {
                    Outcome::OpenUrl(format!("https://warframe.market/items/{}", row.wfm_url))
                }
                _ => Outcome::Done,
            },
        }
    }

    fn row_id(row: &IntendedOrder) -> u64 {
        row_id(row)
    }

    fn compare(key: &str, a: &IntendedOrder, b: &IntendedOrder) -> Ordering {
        match key {
            "action" => a.action.cmp(&b.action),
            "item" => a.item_name.cmp(&b.item_name),
            "kind" => kind_label(a).cmp(&kind_label(b)),
            "price" => a.price.cmp(&b.price),
            _ => a.created_at.cmp(&b.created_at),
        }
    }

    fn cell(row: &IntendedOrder, key: &str, emit: &Emit<IntendedOrderAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        match key {
            "time" => cells::text(day_and_time(&row.created_at)),
            "action" => cells::status(action_color(row.action, theme), row.action.label(), cx),
            "item" => cells::name(item_label(row), &[], cx),
            "kind" => cells::text(kind_label(row)),
            "price" => cells::amount(row.price, "", cx),
            "old_price" => cells::amount(row.old_price, "", cx),
            "quantity" => cells::amount(row.quantity, "", cx),
            "reason" => cells::text(reason(row)),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(overflow_button(row_id(row), move || Self::row_menu(&menu_row), emit))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &IntendedOrder, key: &str) -> Option<String> {
        match key {
            "time" => Some(day_and_time(&row.created_at)),
            "action" => Some(row.action.label().to_owned()),
            "item" => Some(item_label(row)),
            "kind" => Some(kind_label(row)),
            "price" => row.price.map(number),
            "old_price" => row.old_price.map(number),
            "quantity" => row.quantity.map(number),
            "reason" => Some(reason(row)),
            _ => None,
        }
    }

    fn row_menu(row: &IntendedOrder) -> Vec<MenuEntry<IntendedOrderAction>> {
        let id = row_id(row);
        vec![
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                IntendedOrderAction::Details(id),
            ),
            MenuEntry::new("Copy name", IconName::Copy, IntendedOrderAction::CopyName(id))
                .separated(),
            MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                IntendedOrderAction::OpenMarket(id),
            ),
        ]
    }

    fn panel(row: &IntendedOrder, cx: &App) -> PanelModel<IntendedOrderAction> {
        let theme = Theme::global(cx);
        let id = row_id(row);
        PanelModel {
            title: item_label(row),
            subtitle: row.wfm_url.clone(),
            status: Some((
                format!("{}, not sent", row.action.label()),
                action_color(row.action, theme),
            )),
            note: Some(reason(row)),
            history: Vec::new(),
            lines: None,
            facts: vec![
                Fact::new("Order", kind_label(row)),
                Fact::amount("Price", row.price, "None"),
                Fact::amount("Price before", row.old_price, "New order"),
                Fact::amount("Quantity", row.quantity, "None"),
                Fact::new(
                    "Order ID",
                    if row.market_id.is_empty() {
                        "Not created".to_owned()
                    } else {
                        row.market_id.clone()
                    },
                ),
                Fact::new("Time", day_and_time(&row.created_at)),
            ],
            primary: Vec::new(),
            secondary: vec![
                PanelAction::new("Copy name", IconName::Copy, IntendedOrderAction::CopyName(id)),
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    IntendedOrderAction::OpenMarket(id),
                ),
            ],
        }
    }
}
