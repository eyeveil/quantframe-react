//! The Transactions tab: the ledger of sales and purchases.

use std::cmp::Ordering;

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled};
use gpui_kit_assets::IconName;

use crate::{
    data::{DataKind, DataSource},
    kit,
    model::{
        FinancialReport, ListParams, Transaction, TransactionItemType, TransactionListParams,
        TransactionType, TransactionUpdate,
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
        prompts,
    },
};

#[derive(Clone, Debug)]
pub enum TransactionAction {
    Edit(u64),
    Edited { id: i64, update: TransactionUpdate },
    Delete(u64),
    Deleted(u64),
    DeleteMany(Vec<u64>),
    DeletedMany(Vec<u64>),
    Export,
    CalculateTax,
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
}

#[derive(Default)]
pub struct TransactionsData {
    rows: Vec<Transaction>,
    report: FinancialReport,
}

#[derive(Default)]
pub struct Transactions {
    data: TransactionsData,
}

/// Every transaction, newest first.
pub fn all_transactions() -> TransactionListParams {
    TransactionListParams {
        list: ListParams {
            limit: -1,
            sort_by: Some("created_at".to_owned()),
            ..ListParams::default()
        },
        ..TransactionListParams::default()
    }
}

pub fn trade_label(kind: TransactionType) -> &'static str {
    match kind {
        TransactionType::Sale => "Sale",
        TransactionType::Purchase => "Purchase",
    }
}

fn trade_key(kind: TransactionType) -> &'static str {
    match kind {
        TransactionType::Sale => "sale",
        TransactionType::Purchase => "purchase",
    }
}

fn kind_label(kind: TransactionItemType) -> &'static str {
    match kind {
        TransactionItemType::Item => "Item",
        TransactionItemType::Riven => "Riven",
    }
}

fn kind_key(kind: TransactionItemType) -> &'static str {
    match kind {
        TransactionItemType::Item => "item",
        TransactionItemType::Riven => "riven",
    }
}

fn name(row: &Transaction) -> String {
    match row.sub_type.as_ref().and_then(|sub| sub.label()) {
        Some(sub) => format!("{}, {}", row.item_name, sub.to_lowercase()),
        None => row.item_name.clone(),
    }
}

impl Transactions {
    fn row(&self, key: u64) -> Option<&Transaction> {
        self.data.rows.iter().find(|row| row.id as u64 == key)
    }

    fn count(&self, matches: impl Fn(&Transaction) -> bool) -> usize {
        self.data.rows.iter().filter(|row| matches(row)).count()
    }
}

impl Catalog for Transactions {
    type Row = Transaction;
    type Action = TransactionAction;
    type Data = TransactionsData;

    const NOUN: &'static str = "transactions";
    const SEARCH_PLACEHOLDER: &'static str = "Search transactions";
    const EMPTY_HINT: &'static str =
        "A sale or a purchase you record on the Listings screen is added here.";
    const WATCH: &'static [DataKind] = &[DataKind::Transactions];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let params = all_transactions();
        let rows = source.get_transaction_pagination(&params);
        let report = source.get_transaction_financial_report(&params);
        Box::pin(async move {
            Ok(TransactionsData {
                rows: rows.await?.results,
                report: report.await?,
            })
        })
    }

    fn apply(&mut self, data: Self::Data) {
        self.data = data;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("item", "Item", 220.).flex().sortable(),
            ColumnSpec::new("trade", "Trade", 104.).sortable(),
            ColumnSpec::new("kind", "Type", 76.).sortable().droppable(4),
            ColumnSpec::new("partner", "Trader", 150.)
                .sortable()
                .droppable(2),
            ColumnSpec::new("quantity", "Quantity", 84.).end().sortable(),
            ColumnSpec::new("price", "Price", 84.).end().sortable(),
            ColumnSpec::new("profit", "Profit", 84.).end().sortable(),
            ColumnSpec::new("credits", "Credits", 96.)
                .end()
                .sortable()
                .droppable(3),
            ColumnSpec::new("created", "Date", 132.).sortable().droppable(1),
            ColumnSpec::new("actions", "", 104.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<Transaction> {
        let trade = query.filter("trade");
        let kind = query.filter("kind");
        self.data
            .rows
            .iter()
            .filter(|row| trade.is_none_or(|key| trade_key(row.transaction_type) == key))
            .filter(|row| kind.is_none_or(|key| kind_key(row.item_type) == key))
            .filter(|row| query.matches(&format!("{} {}", row.item_name, row.user_name)))
            .cloned()
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        let colors = Theme::global(cx).colors;
        let trade = [
            (TransactionType::Sale, colors.positive, "You sold the item."),
            (
                TransactionType::Purchase,
                colors.info,
                "You bought the item.",
            ),
        ];
        let kinds = [TransactionItemType::Item, TransactionItemType::Riven];
        vec![
            ChipGroup {
                id: "trade",
                chips: trade
                    .into_iter()
                    .map(|(kind, color, tooltip)| ChipSpec {
                        key: trade_key(kind).to_owned(),
                        label: format!("{}s", trade_label(kind)),
                        count: self.count(|row| row.transaction_type == kind),
                        color: Some(color),
                        tooltip: tooltip.to_owned(),
                    })
                    .collect(),
            },
            ChipGroup {
                id: "kind",
                chips: kinds
                    .into_iter()
                    .map(|kind| ChipSpec {
                        key: kind_key(kind).to_owned(),
                        label: format!("{}s", kind_label(kind)),
                        count: self.count(|row| row.item_type == kind),
                        color: None,
                        tooltip: String::new(),
                    })
                    .collect(),
            },
        ]
    }

    fn summary(&self) -> Vec<Stat> {
        let report = &self.data.report;
        vec![
            Stat {
                label: "Revenue",
                value: number(report.revenue),
                note: format!("{} sales", number(report.sale_count)),
                tone: Tone::Plain,
            },
            Stat {
                label: "Expenses",
                value: number(report.expenses),
                note: format!("{} purchases", number(report.purchases_count)),
                tone: Tone::Plain,
            },
            Stat {
                label: "Profit",
                value: number(report.total_profit),
                note: format!("{}% margin", kit::decimal(report.profit_margin)),
                tone: Tone::of_profit(report.total_profit),
            },
            Stat {
                label: "Return on investment",
                value: format!("{}%", kit::decimal(report.roi)),
                note: format!("{} per trade", kit::decimal(report.average_profit)),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, checked: &[u64]) -> Vec<ToolbarItem<TransactionAction>> {
        vec![
            ToolbarItem {
                id: "export",
                icon: IconName::Download,
                tooltip: "Export the transactions as JSON".to_owned(),
                action: Some(TransactionAction::Export),
                danger: false,
            },
            ToolbarItem {
                id: "tax",
                icon: IconName::Calculator,
                tooltip: "Fill in the trade tax of every transaction".to_owned(),
                action: Some(TransactionAction::CalculateTax),
                danger: false,
            },
            ToolbarItem {
                id: "delete",
                icon: IconName::Trash,
                tooltip: "Delete the selected transactions".to_owned(),
                action: (!checked.is_empty())
                    .then(|| TransactionAction::DeleteMany(checked.to_vec())),
                danger: true,
            },
        ]
    }

    fn primary(&self) -> Option<PrimaryAction<TransactionAction>> {
        None
    }

    fn perform(
        &mut self,
        action: TransactionAction,
        source: &DataSource,
    ) -> Outcome<TransactionAction> {
        match action {
            TransactionAction::Edit(key) => match self.row(key) {
                Some(row) => {
                    let id = row.id;
                    Outcome::ask(
                        FormSpec::new(format!("Edit {}", name(row)), "Save")
                            .field(Field::platinum("price", "Price", Some(row.price)).half())
                            .field(
                                Field::number("quantity", "Quantity", Some(row.quantity))
                                    .min(1)
                                    .half(),
                            )
                            .field(
                                Field::text("user_name", "Trader", &row.user_name, "In-game name")
                                    .optional()
                                    .half(),
                            )
                            .field(
                                Field::number("credits", "Credits", Some(row.credits))
                                    .hint("The trade tax paid.")
                                    .half(),
                            ),
                        move |values| {
                            Some(TransactionAction::Edited {
                                id,
                                update: TransactionUpdate {
                                    price: values.number("price"),
                                    quantity: values.number("quantity"),
                                    user_name: Some(values.text("user_name")),
                                    credits: values.number("credits"),
                                },
                            })
                        },
                    )
                }
                None => Outcome::Done,
            },
            TransactionAction::Edited { id, update } => {
                Outcome::run_quietly(source.transaction_update(id, update))
            }
            TransactionAction::Delete(key) => Outcome::ask(
                prompts::delete(1, "transaction", "transactions"),
                move |_| Some(TransactionAction::Deleted(key)),
            ),
            TransactionAction::Deleted(key) => {
                Outcome::run_quietly(source.transaction_delete(key as i64))
            }
            TransactionAction::DeleteMany(keys) => Outcome::ask(
                prompts::delete(keys.len(), "transaction", "transactions"),
                move |_| Some(TransactionAction::DeletedMany(keys.clone())),
            ),
            TransactionAction::DeletedMany(keys) => {
                let count = keys.len();
                Outcome::run(
                    source
                        .transaction_delete_bulk(keys.into_iter().map(|key| key as i64).collect()),
                    format!("Deleted {count} transactions."),
                )
            }
            TransactionAction::Export => {
                let source = source.clone();
                Outcome::export("transactions.json", move |path| {
                    Box::pin(source.export_transaction_json(&all_transactions(), path))
                })
            }
            TransactionAction::CalculateTax => Outcome::run(
                source.transaction_calculate_tax(),
                "Filled in the trade tax of every transaction.",
            ),
            TransactionAction::Details(key) => Outcome::ShowPanel(key),
            TransactionAction::CopyName(key) => match self.row(key) {
                Some(row) => Outcome::Copy {
                    what: row.item_name.clone(),
                    text: row.item_name.clone(),
                },
                None => Outcome::Done,
            },
            TransactionAction::OpenMarket(key) => match self.row(key) {
                Some(row) if !row.wfm_url.is_empty() => Outcome::OpenUrl(match row.item_type {
                    TransactionItemType::Item => {
                        format!("https://warframe.market/items/{}", row.wfm_url)
                    }
                    TransactionItemType::Riven => format!(
                        "https://warframe.market/auctions/search?type=riven&weapon_url_name={}",
                        row.wfm_url
                    ),
                }),
                _ => Outcome::Done,
            },
        }
    }

    fn row_id(row: &Transaction) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &Transaction, b: &Transaction) -> Ordering {
        match key {
            "trade" => trade_key(a.transaction_type).cmp(trade_key(b.transaction_type)),
            "kind" => kind_key(a.item_type).cmp(kind_key(b.item_type)),
            "partner" => a.user_name.to_lowercase().cmp(&b.user_name.to_lowercase()),
            "quantity" => a.quantity.cmp(&b.quantity),
            "price" => a.price.cmp(&b.price),
            "profit" => a.profit.cmp(&b.profit),
            "credits" => a.credits.cmp(&b.credits),
            "created" => a.created_at.cmp(&b.created_at),
            _ => a.item_name.cmp(&b.item_name),
        }
    }

    fn cell(
        row: &Transaction,
        key: &str,
        emit: &Emit<TransactionAction>,
        cx: &App,
    ) -> AnyElement {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let id = row.id as u64;
        match key {
            "item" => cells::name(name(row), &[], cx),
            "trade" => cells::status(
                match row.transaction_type {
                    TransactionType::Sale => colors.positive,
                    TransactionType::Purchase => colors.info,
                },
                trade_label(row.transaction_type),
                cx,
            ),
            "kind" => cells::text(kind_label(row.item_type)),
            "partner" => cells::text(row.user_name.clone()),
            "quantity" => cells::amount(Some(row.quantity), "", cx),
            "price" => cells::amount(Some(row.price), "", cx),
            "profit" => match row.profit {
                Some(profit) => crate::ui::numeric(number(profit))
                    .text_color(Tone::of_profit(profit).color(theme))
                    .into_any_element(),
                None => cells::amount(None, "None", cx),
            },
            "credits" => cells::amount(Some(row.credits), "", cx),
            "created" => cells::text(day_and_time(&row.created_at)),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(
                        IconButton::new(("edit", id), IconName::SquarePen, "Edit this transaction")
                            .in_row()
                            .on_click(emit.on_click(TransactionAction::Edit(id))),
                    )
                    .child(
                        IconButton::new(("delete", id), IconName::Trash, "Delete this transaction")
                            .in_row()
                            .hover_tone(colors.negative)
                            .on_click(emit.on_click(TransactionAction::Delete(id))),
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

    fn cell_text(row: &Transaction, key: &str) -> Option<String> {
        match key {
            "item" => Some(name(row)),
            "trade" => Some(trade_label(row.transaction_type).to_owned()),
            "kind" => Some(kind_label(row.item_type).to_owned()),
            "partner" => Some(row.user_name.clone()),
            "quantity" => Some(number(row.quantity)),
            "price" => Some(number(row.price)),
            "profit" => row.profit.map(number),
            "credits" => Some(number(row.credits)),
            "created" => Some(day_and_time(&row.created_at)),
            _ => None,
        }
    }

    fn row_menu(row: &Transaction) -> Vec<MenuEntry<TransactionAction>> {
        let id = row.id as u64;
        vec![
            MenuEntry::new("Edit", IconName::SquarePen, TransactionAction::Edit(id)),
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                TransactionAction::Details(id),
            ),
            MenuEntry::new("Copy name", IconName::Copy, TransactionAction::CopyName(id)),
            MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                TransactionAction::OpenMarket(id),
            ),
            MenuEntry::new("Delete", IconName::Trash, TransactionAction::Delete(id)).separated(),
        ]
    }

    fn panel(row: &Transaction, cx: &App) -> PanelModel<TransactionAction> {
        let colors = Theme::global(cx).colors;
        let id = row.id as u64;
        let sale = row.transaction_type == TransactionType::Sale;
        PanelModel {
            title: name(row),
            subtitle: match (sale, row.user_name.is_empty()) {
                (_, true) => kind_label(row.item_type).to_owned(),
                (true, false) => format!("Sold to {}", row.user_name),
                (false, false) => format!("Bought from {}", row.user_name),
            },
            status: Some((
                trade_label(row.transaction_type).to_owned(),
                if sale { colors.positive } else { colors.info },
            )),
            note: (!row.tags.is_empty()).then(|| row.tags.join(", ")),
            history: Vec::new(),
            lines: None,
            facts: vec![
                Fact::new("Price", number(row.price)),
                Fact::new("Quantity", number(row.quantity)),
                match row.profit {
                    Some(profit) => {
                        Fact::new("Profit", number(profit)).tone(Tone::of_profit(profit))
                    }
                    None => Fact::new("Profit", "None").tone(Tone::Muted),
                },
                Fact::new("Credits", number(row.credits)),
                Fact::new("Type", kind_label(row.item_type)),
                Fact::new("Date", day_and_time(&row.created_at)),
                Fact::new("Changed", day_and_time(&row.updated_at)),
            ],
            primary: vec![PanelAction::new(
                "Edit",
                IconName::SquarePen,
                TransactionAction::Edit(id),
            )],
            secondary: vec![
                PanelAction::new("Copy name", IconName::Copy, TransactionAction::CopyName(id)),
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    TransactionAction::OpenMarket(id),
                )
                .enabled(!row.wfm_url.is_empty()),
                PanelAction::new("Delete", IconName::Trash, TransactionAction::Delete(id))
                    .danger(),
            ],
        }
    }
}
