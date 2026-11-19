use std::{cmp::Ordering, sync::Arc};

use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement, Styled};
use gpui_kit_assets::IconName;

use super::{
    item_text, quantities, ready,
    report::{Ink, Monthly, Report, Table},
    year, year_chips,
};
use crate::{
    data::{DataKind, DataSource},
    model::{GdprAccount, GdprTrade, GdprTradeKind},
    theme::Theme,
    ui::{
        cells, counted, date_time_with_year,
        grid::{
            Catalog, ChipGroup, ChipSpec, ColumnSpec, Emit, Load, MenuEntry, Outcome,
            PrimaryAction, Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number, numeric,
        panel::{Fact, PanelAction, PanelModel},
    },
};

#[derive(Clone, Debug)]
pub enum TradeAction {
    Details(u64),
    CopyItems(u64),
}

#[derive(Clone, Debug)]
pub struct TradeRow {
    id: u64,
    trade: GdprTrade,
}

#[derive(Default)]
pub struct Trades {
    pub account: Option<Arc<GdprAccount>>,
}

fn kind_color(kind: GdprTradeKind, cx: &App) -> Hsla {
    let colors = Theme::global(cx).colors;
    match kind {
        GdprTradeKind::Sale => colors.positive,
        GdprTradeKind::Purchase => colors.info,
        GdprTradeKind::Swap => colors.warning,
    }
}

fn items(trade: &GdprTrade) -> String {
    match trade.kind {
        GdprTradeKind::Sale => item_text(&trade.gave),
        GdprTradeKind::Purchase => item_text(&trade.received),
        GdprTradeKind::Swap => match (trade.gave.is_empty(), trade.received.is_empty()) {
            (false, false) => format!(
                "{} for {}",
                item_text(&trade.gave),
                item_text(&trade.received)
            ),
            (false, true) => item_text(&trade.gave),
            (true, _) => item_text(&trade.received),
        },
    }
}

fn platinum(trade: &GdprTrade) -> Option<i64> {
    match trade.kind {
        GdprTradeKind::Sale => Some(trade.platinum),
        GdprTradeKind::Purchase => Some(-trade.platinum),
        GdprTradeKind::Swap => (trade.platinum != 0).then_some(trade.platinum),
    }
}

fn signed(value: i64) -> String {
    if value > 0 {
        format!("+{}", number(value))
    } else {
        number(value)
    }
}

fn sum(trades: &[GdprTrade], kind: GdprTradeKind) -> (i64, i64, i64) {
    let matching = trades.iter().filter(|trade| trade.kind == kind);
    let count = matching.clone().count() as i64;
    let total = matching.clone().map(|trade| trade.platinum).sum();
    let largest = matching.map(|trade| trade.platinum).max().unwrap_or(0);
    (count, total, largest)
}

impl Trades {
    fn trades(&self) -> &[GdprTrade] {
        self.account
            .as_ref()
            .map_or(&[], |account| account.trades.as_slice())
    }

    fn row(&self, id: u64) -> Option<&GdprTrade> {
        self.trades().get(id as usize)
    }
}

pub fn report(account: &GdprAccount) -> Report {
    let trades = &account.trades;
    let (sales, revenue, best_sale) = sum(trades, GdprTradeKind::Sale);
    let (purchases, expenses, largest_purchase) = sum(trades, GdprTradeKind::Purchase);
    let (swaps, _, _) = sum(trades, GdprTradeKind::Swap);
    let credits: i64 = trades.iter().map(|trade| trade.credits).sum();
    let profit = revenue - expenses;
    let sold = quantities(
        trades
            .iter()
            .filter(|trade| trade.kind == GdprTradeKind::Sale)
            .flat_map(|trade| &trade.gave)
            .map(|item| (item.name.as_str(), item.quantity, 0)),
    );
    let bought = quantities(
        trades
            .iter()
            .filter(|trade| trade.kind == GdprTradeKind::Purchase)
            .flat_map(|trade| &trade.received)
            .map(|item| (item.name.as_str(), item.quantity, 0)),
    );
    let table = |title: &'static str, rows: Vec<(String, i64, i64)>| Table {
        title,
        name: "Item",
        columns: vec!["Quantity"],
        empty: "No items.",
        rows: rows
            .into_iter()
            .take(5)
            .map(|(name, quantity, _)| (name, vec![number(quantity)]))
            .collect(),
    };
    let mut monthly = Monthly::new(
        "Platinum per month",
        vec![("Sales", Ink::Positive), ("Purchases", Ink::Negative)],
    );
    for trade in trades {
        let series = match trade.kind {
            GdprTradeKind::Sale => 0,
            GdprTradeKind::Purchase => 1,
            GdprTradeKind::Swap => continue,
        };
        monthly.add(&trade.time, series, trade.platinum);
    }
    Report {
        stats: vec![
            Stat {
                label: "Revenue",
                value: number(revenue),
                note: format!(
                    "{}, largest {}",
                    counted(sales, "sale", "sales"),
                    number(best_sale)
                ),
                tone: Tone::Plain,
            },
            Stat {
                label: "Expenses",
                value: number(expenses),
                note: format!(
                    "{}, largest {}",
                    counted(purchases, "purchase", "purchases"),
                    number(largest_purchase)
                ),
                tone: Tone::Plain,
            },
            Stat {
                label: "Profit",
                value: number(profit),
                note: "revenue less expenses, in platinum".to_owned(),
                tone: Tone::of_profit(profit),
            },
            Stat {
                label: "Swaps",
                value: number(swaps),
                note: format!("{} credits given in all trades", number(credits)),
                tone: Tone::Plain,
            },
        ],
        tables: vec![
            table("Most sold items", sold),
            table("Most bought items", bought),
        ],
        monthly,
        note: None,
    }
}

impl Catalog for Trades {
    type Row = TradeRow;
    type Action = TradeAction;
    type Data = ();

    const NOUN: &'static str = "trades";
    const SEARCH_PLACEHOLDER: &'static str = "Search traded items";
    const EMPTY_HINT: &'static str = "This account's export lists no trades.";
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[];

    fn load(&self, _: &DataSource) -> Load<()> {
        ready()
    }

    fn apply(&mut self, (): ()) {}

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("date", "Date", 168.).sortable(),
            ColumnSpec::new("kind", "Trade", 116.).sortable(),
            ColumnSpec::new("items", "Items", 260.).flex().sortable(),
            ColumnSpec::new("platinum", "Platinum", 104.).end().sortable(),
            ColumnSpec::new("credits", "Credits", 112.)
                .end()
                .sortable()
                .droppable(1),
            ColumnSpec::new("actions", "", 52.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<TradeRow> {
        let kind = query.filter("kind");
        let in_year = query.filter("year");
        self.trades()
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, trade)| kind.is_none_or(|key| trade.kind.key() == key))
            .filter(|(_, trade)| in_year.is_none_or(|key| year(&trade.time) == key))
            .filter(|(_, trade)| {
                query.matches(&format!(
                    "{} {}",
                    item_text(&trade.gave),
                    item_text(&trade.received)
                ))
            })
            .map(|(index, trade)| TradeRow {
                id: index as u64,
                trade: trade.clone(),
            })
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        let trades = self.trades();
        let tooltips = [
            "You gave items and received platinum.",
            "You gave platinum and received items.",
            "Items for items, or platinum on both sides.",
        ];
        vec![
            ChipGroup {
                id: "kind",
                chips: GdprTradeKind::ALL
                    .into_iter()
                    .zip(tooltips)
                    .map(|(kind, tooltip)| ChipSpec {
                        key: kind.key().to_owned(),
                        label: format!("{}s", kind.label()),
                        count: trades.iter().filter(|trade| trade.kind == kind).count(),
                        color: Some(kind_color(kind, cx)),
                        tooltip: tooltip.to_owned(),
                    })
                    .collect(),
            },
            year_chips(trades.iter().map(|trade| trade.time.as_str())),
        ]
    }

    fn summary(&self) -> Vec<Stat> {
        let trades = self.trades();
        let (sales, revenue, _) = sum(trades, GdprTradeKind::Sale);
        let (purchases, expenses, _) = sum(trades, GdprTradeKind::Purchase);
        let (swaps, _, _) = sum(trades, GdprTradeKind::Swap);
        let net = revenue - expenses;
        vec![
            Stat {
                label: "Platinum received",
                value: number(revenue),
                note: counted(sales, "sale", "sales"),
                tone: Tone::Plain,
            },
            Stat {
                label: "Platinum given",
                value: number(expenses),
                note: counted(purchases, "purchase", "purchases"),
                tone: Tone::Plain,
            },
            Stat {
                label: "Net platinum",
                value: number(net),
                note: "received less given".to_owned(),
                tone: Tone::of_profit(net),
            },
            Stat {
                label: "Trades",
                value: number(trades.len() as i64),
                note: counted(swaps, "swap", "swaps"),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<TradeAction>> {
        Vec::new()
    }

    fn primary(&self) -> Option<PrimaryAction<TradeAction>> {
        None
    }

    fn perform(&mut self, action: TradeAction, _: &DataSource) -> Outcome<TradeAction> {
        match action {
            TradeAction::Details(id) => Outcome::ShowPanel(id),
            TradeAction::CopyItems(id) => match self.row(id) {
                Some(trade) => Outcome::Copy {
                    text: items(trade),
                    what: "the item names".to_owned(),
                },
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &TradeRow) -> u64 {
        row.id
    }

    fn compare(key: &str, a: &TradeRow, b: &TradeRow) -> Ordering {
        match key {
            "kind" => a.trade.kind.cmp(&b.trade.kind),
            "items" => items(&a.trade).cmp(&items(&b.trade)),
            "platinum" => platinum(&a.trade).cmp(&platinum(&b.trade)),
            "credits" => a.trade.credits.cmp(&b.trade.credits),
            _ => a.trade.time.cmp(&b.trade.time),
        }
    }

    fn cell(row: &TradeRow, key: &str, emit: &Emit<TradeAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let trade = &row.trade;
        match key {
            "date" => cells::text(date_time_with_year(&trade.time)),
            "kind" => cells::status(kind_color(trade.kind, cx), trade.kind.label(), cx),
            "items" => cells::name(items(trade), &[], cx),
            "platinum" => match platinum(trade) {
                Some(value) => numeric(signed(value))
                    .text_color(Tone::of_profit(value).color(theme))
                    .into_any_element(),
                None => cells::amount(None, "None", cx),
            },
            "credits" => cells::amount((trade.credits > 0).then_some(trade.credits), "None", cx),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(overflow_button(
                        row.id,
                        move || Self::row_menu(&menu_row),
                        emit,
                    ))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &TradeRow, key: &str) -> Option<String> {
        let trade = &row.trade;
        match key {
            "date" => Some(date_time_with_year(&trade.time)),
            "kind" => Some(trade.kind.label().to_owned()),
            "items" => Some(items(trade)),
            "platinum" => platinum(trade).map(signed),
            "credits" => Some(number(trade.credits)),
            _ => None,
        }
    }

    fn row_menu(row: &TradeRow) -> Vec<MenuEntry<TradeAction>> {
        vec![
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                TradeAction::Details(row.id),
            ),
            MenuEntry::new(
                "Copy item names",
                IconName::Copy,
                TradeAction::CopyItems(row.id),
            ),
        ]
    }

    fn panel(row: &TradeRow, cx: &App) -> PanelModel<TradeAction> {
        let trade = &row.trade;
        let side = |verb: &str, items: &[crate::model::GdprItem], platinum: i64, credits: i64| {
            let mut lines: Vec<(String, Tone)> = items
                .iter()
                .map(|item| {
                    (
                        format!("{verb} {} {}", number(item.quantity), item.name),
                        Tone::Plain,
                    )
                })
                .collect();
            if platinum > 0 {
                lines.push((format!("{verb} {} platinum", number(platinum)), Tone::Plain));
            }
            if credits > 0 {
                lines.push((format!("{verb} {} credits", number(credits)), Tone::Muted));
            }
            lines
        };
        let (gave_platinum, received_platinum) = match trade.kind {
            GdprTradeKind::Sale => (0, trade.platinum),
            GdprTradeKind::Purchase => (trade.platinum, 0),
            GdprTradeKind::Swap => (0, 0),
        };
        let mut lines = side("Gave", &trade.gave, gave_platinum, trade.credits);
        lines.extend(side("Received", &trade.received, received_platinum, 0));
        PanelModel {
            title: items(trade),
            subtitle: date_time_with_year(&trade.time),
            status: Some((
                trade.kind.label().to_owned(),
                kind_color(trade.kind, cx),
            )),
            note: None,
            history: Vec::new(),
            lines: Some(("Exchanged", lines)),
            facts: vec![
                match platinum(trade) {
                    Some(value) => {
                        Fact::new("Platinum", signed(value)).tone(Tone::of_profit(value))
                    }
                    None => Fact::new("Platinum", "None").tone(Tone::Muted),
                },
                Fact::new("Credits", number(trade.credits)),
                Fact::new(
                    "Items given",
                    number(trade.gave.iter().map(|item| item.quantity).sum()),
                ),
                Fact::new(
                    "Items received",
                    number(trade.received.iter().map(|item| item.quantity).sum()),
                ),
            ],
            primary: Vec::new(),
            secondary: vec![PanelAction::new(
                "Copy item names",
                IconName::Copy,
                TradeAction::CopyItems(row.id),
            )],
        }
    }
}
