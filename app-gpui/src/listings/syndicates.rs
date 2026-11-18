//! The Syndicates tab: what the syndicates sell for standing, listed on
//! Warframe Market by the live scraper.

use std::{cmp::Ordering, collections::BTreeMap};

use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement, Styled, div, px, rgb};
use gpui_kit_assets::IconName;

use super::{
    StockData, bulk_toolbar, every_row, ids, market_url, rank_label, status_chips, status_filter,
};
use crate::{
    data::{DataKind, DataSource},
    model::{StockStatus, SyndicateItem, SyndicateItemListParams, SyndicateItemUpdate},
    theme::Theme,
    ui::{
        IconButton, cells, day_and_time,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ChipSpec, ColumnSpec, Emit, Load, MenuEntry, Outcome,
            PrimaryAction, Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number, numeric,
        panel::{Fact, PanelAction, PanelModel},
        prompts, row_button,
    },
};

#[derive(Clone, Debug)]
pub enum SyndicateAction {
    Import,
    Sell(u64),
    Sold { id: u64, price: i64 },
    SellAtList(u64),
    Edit(u64),
    Save { id: u64, update: SyndicateItemUpdate },
    Delete(Vec<u64>),
    Deleted(Vec<u64>),
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
    Export,
}

#[derive(Default)]
pub struct SyndicateData {
    stock: StockData<SyndicateItem>,
    /// Item count per syndicate name.
    syndicates: BTreeMap<String, usize>,
}

#[derive(Default)]
pub struct Syndicates {
    data: SyndicateData,
}

/// The colour each syndicate flies in the game.
fn syndicate_color(name: &str) -> Hsla {
    let hex = match name {
        "Steel Meridian" => 0xc4622d,
        "Arbiters of Hexis" => 0x9aa3ad,
        "Cephalon Suda" => 0x4f7fd9,
        "The Perrin Sequence" | "Perrin Sequence" => 0x3fa7b5,
        "Red Veil" => 0xc2413b,
        "New Loka" => 0x5aa55f,
        _ => 0x8e8e96,
    };
    rgb(hex).into()
}

/// Platinum earned per 1,000 standing spent, at the list price.
fn rate(item: &SyndicateItem) -> Option<f64> {
    let price = item.list_price?;
    (item.standing_cost > 0).then(|| price as f64 * 1000.0 / item.standing_cost as f64)
}

fn every_item() -> SyndicateItemListParams {
    SyndicateItemListParams {
        stock: every_row(),
        syndicate: None,
    }
}

impl Syndicates {
    fn item(&self, id: u64) -> Option<SyndicateItem> {
        self.data
            .stock
            .rows
            .iter()
            .find(|item| item.id as u64 == id)
            .cloned()
    }
}

impl Catalog for Syndicates {
    type Row = SyndicateItem;
    type Action = SyndicateAction;
    type Data = SyndicateData;

    const NOUN: &'static str = "syndicate items";
    const SEARCH_PLACEHOLDER: &'static str = "Search syndicate items";
    const EMPTY_HINT: &'static str =
        "Import the syndicate offerings and the live scraper will price them for you.";
    const WATCH: &'static [DataKind] = &[DataKind::SyndicateItems];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let rows = source.get_syndicate_item_pagination(&every_item());
        let counts = source.get_syndicate_item_status_counts();
        let report = source.get_syndicate_item_financial_report();
        let syndicates = source.get_syndicate_item_syndicate_counts();
        Box::pin(async move {
            Ok(SyndicateData {
                stock: StockData {
                    rows: rows.await?.results,
                    counts: counts.await?,
                    report: report.await?,
                },
                syndicates: syndicates.await?,
            })
        })
    }

    fn apply(&mut self, data: Self::Data) {
        self.data = data;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("item", "Item", 180.).flex().sortable(),
            ColumnSpec::new("rank", "Rank", 96.).droppable(3),
            ColumnSpec::new("syndicate", "Syndicate", 184.).sortable(),
            ColumnSpec::new("standing", "Standing", 104.)
                .end()
                .sortable()
                .droppable(1),
            ColumnSpec::new("list_price", "List price", 104.)
                .end()
                .sortable(),
            ColumnSpec::new("rate", "Per 1,000 standing", 144.)
                .end()
                .sortable()
                .droppable(2),
            ColumnSpec::new("status", "Status", 176.).sortable(),
            ColumnSpec::new("actions", "", 150.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<SyndicateItem> {
        let status = status_filter(query);
        let syndicate = query.filter("syndicate");
        self.data
            .stock
            .rows
            .iter()
            .filter(|item| status.is_none_or(|status| item.status == status))
            .filter(|item| syndicate.is_none_or(|name| item.syndicate_name == name))
            .filter(|item| query.matches(&format!("{} {}", item.item_name, item.syndicate_name)))
            .cloned()
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        vec![
            status_chips(&self.data.stock.counts, cx),
            ChipGroup {
                id: "syndicate",
                chips: self
                    .data
                    .syndicates
                    .iter()
                    .map(|(name, count)| ChipSpec {
                        color: Some(syndicate_color(name)),
                        key: name.clone(),
                        label: name.clone(),
                        count: *count,
                        tooltip: String::new(),
                    })
                    .collect(),
            },
        ]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = &self.data.stock.rows;
        let listed: i64 = rows.iter().filter_map(|item| item.list_price).sum();
        let standing: i64 = rows
            .iter()
            .filter(|item| item.list_price.is_some())
            .map(|item| item.standing_cost)
            .sum();
        let best = rows
            .iter()
            .filter_map(|item| Some((rate(item)?, item)))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        let live = self
            .data
            .stock
            .counts
            .get(&StockStatus::Live)
            .copied()
            .unwrap_or(0);
        vec![
            Stat {
                label: "Listed",
                value: number(listed),
                note: "one of each priced item".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Standing",
                value: number(standing),
                note: "to buy one of each".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Best rate",
                value: best.map_or("None".to_owned(), |(rate, _)| format!("{rate:.2}")),
                note: best.map_or(String::new(), |(_, item)| {
                    format!("per 1,000 standing, {}", item.item_name)
                }),
                tone: Tone::Plain,
            },
            Stat {
                label: "Items",
                value: number(rows.len() as i64),
                note: format!("{live} live on Warframe Market"),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, checked: &[u64]) -> Vec<ToolbarItem<SyndicateAction>> {
        bulk_toolbar(
            checked,
            "syndicate items",
            SyndicateAction::Export,
            None,
            None,
            SyndicateAction::Delete,
        )
    }

    fn primary(&self) -> Option<PrimaryAction<SyndicateAction>> {
        Some(PrimaryAction {
            label: "Import offerings",
            icon: IconName::Download,
            action: SyndicateAction::Import,
        })
    }

    fn perform(&mut self, action: SyndicateAction, source: &DataSource) -> Outcome<SyndicateAction> {
        match action {
            SyndicateAction::Import => {
                let import = source.syndicate_item_import_items();
                Outcome::Run(Box::pin(async move {
                    let count = import.await?;
                    Ok(format!("Imported {} syndicate items.", number(count)))
                }))
            }
            SyndicateAction::Sell(id) => match self.item(id) {
                Some(item) => Outcome::ask(
                    prompts::sold(&item.item_name, None, item.list_price),
                    move |values| {
                        Some(SyndicateAction::Sold {
                            id,
                            price: values.number("price")?,
                        })
                    },
                ),
                None => Outcome::Done,
            },
            SyndicateAction::SellAtList(id) => {
                match self.item(id).and_then(|item| item.list_price) {
                    Some(price) => self.perform(SyndicateAction::Sold { id, price }, source),
                    None => Outcome::Done,
                }
            }
            SyndicateAction::Sold { id, price } => match self.item(id) {
                Some(item) => {
                    let notice = format!(
                        "Sold {} for {} platinum.",
                        item.item_name,
                        number(price)
                    );
                    Outcome::run(
                        source.syndicate_item_sell(
                            item.wfm_url,
                            item.sub_type,
                            item.syndicate_unique_name,
                            price,
                        ),
                        notice,
                    )
                }
                None => Outcome::Done,
            },
            SyndicateAction::Edit(id) => match self.item(id) {
                Some(item) => Outcome::ask(
                    FormSpec::new(format!("Edit {}", item.item_name), "Save").field(
                        Field::number("standing_cost", "Standing cost", Some(item.standing_cost))
                            .unit("standing")
                            .hint("What the syndicate charges for one."),
                    ),
                    move |values| {
                        Some(SyndicateAction::Save {
                            id,
                            update: SyndicateItemUpdate {
                                standing_cost: values.number("standing_cost"),
                                ..SyndicateItemUpdate::default()
                            },
                        })
                    },
                ),
                None => Outcome::Done,
            },
            SyndicateAction::Save { id, update } => {
                Outcome::run_quietly(source.syndicate_item_update(id as i64, update))
            }
            SyndicateAction::Delete(rows) if rows.is_empty() => Outcome::Done,
            SyndicateAction::Delete(rows) => Outcome::ask(
                prompts::delete(rows.len(), "syndicate item", "syndicate items"),
                move |_| Some(SyndicateAction::Deleted(rows.clone())),
            ),
            SyndicateAction::Deleted(rows) => match rows.as_slice() {
                [id] => Outcome::run_quietly(source.syndicate_item_delete(*id as i64)),
                _ => Outcome::run_quietly(source.syndicate_item_delete_multiple(ids(&rows))),
            },
            SyndicateAction::Details(id) => Outcome::ShowPanel(id),
            SyndicateAction::CopyName(id) => match self.item(id) {
                Some(item) => Outcome::Copy {
                    what: item.item_name.clone(),
                    text: item.item_name,
                },
                None => Outcome::Done,
            },
            SyndicateAction::OpenMarket(id) => match self.item(id) {
                Some(item) => Outcome::OpenUrl(market_url(&item.wfm_url)),
                None => Outcome::Done,
            },
            SyndicateAction::Export => {
                let source = source.clone();
                Outcome::export("syndicate-items.json", move |path| {
                    Box::pin(source.export_syndicate_item_json(&every_item(), path))
                })
            }
        }
    }

    fn row_id(row: &SyndicateItem) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &SyndicateItem, b: &SyndicateItem) -> Ordering {
        match key {
            "syndicate" => a.syndicate_name.cmp(&b.syndicate_name),
            "standing" => a.standing_cost.cmp(&b.standing_cost),
            "list_price" => a.list_price.cmp(&b.list_price),
            "rate" => rate(a).unwrap_or(-1.0).total_cmp(&rate(b).unwrap_or(-1.0)),
            "status" => a.status.label().cmp(b.status.label()),
            _ => a.item_name.cmp(&b.item_name),
        }
    }

    fn cell(row: &SyndicateItem, key: &str, emit: &Emit<SyndicateAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let id = row.id as u64;
        match key {
            "item" => cells::name(row.item_name.clone(), &[], cx),
            "rank" => cells::text(rank_label(&row.sub_type).unwrap_or_default()),
            "syndicate" => cells::status(
                syndicate_color(&row.syndicate_name),
                row.syndicate_name.clone(),
                cx,
            ),
            "standing" => cells::amount(Some(row.standing_cost), "", cx),
            "list_price" => cells::amount(row.list_price, "Not listed", cx),
            "rate" => match rate(row) {
                Some(rate) => numeric(format!("{rate:.2}"))
                    .text_color(theme.colors.text)
                    .into_any_element(),
                None => cells::amount(None, "", cx),
            },
            "status" => cells::status(theme.status_color(row.status), row.status.label(), cx),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(
                        div().pr(px(5.)).child(
                            row_button(("sell", id), "Sell", cx)
                                .on_click(emit.on_click(SyndicateAction::Sell(id))),
                        ),
                    )
                    .child(
                        IconButton::new(
                            ("sell-list", id),
                            IconName::Gavel,
                            "Sell one at the list price",
                        )
                        .in_row()
                        .disabled(row.list_price.is_none())
                        .on_click(emit.on_click(SyndicateAction::SellAtList(id))),
                    )
                    .child(
                        IconButton::new(("edit", id), IconName::SquarePen, "Edit")
                            .in_row()
                            .on_click(emit.on_click(SyndicateAction::Edit(id))),
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

    fn cell_text(row: &SyndicateItem, key: &str) -> Option<String> {
        match key {
            "item" => Some(row.item_name.clone()),
            "rank" => rank_label(&row.sub_type),
            "syndicate" => Some(row.syndicate_name.clone()),
            "standing" => Some(number(row.standing_cost)),
            "list_price" => row.list_price.map(number),
            "rate" => rate(row).map(|rate| format!("{rate:.2}")),
            "status" => Some(row.status.label().to_owned()),
            _ => None,
        }
    }

    fn row_menu(row: &SyndicateItem) -> Vec<MenuEntry<SyndicateAction>> {
        let id = row.id as u64;
        let mut menu = vec![MenuEntry::new(
            "Sell",
            IconName::HandCoins,
            SyndicateAction::Sell(id),
        )];
        if row.list_price.is_some() {
            menu.push(MenuEntry::new(
                "Sell one at the list price",
                IconName::Gavel,
                SyndicateAction::SellAtList(id),
            ));
        }
        menu.extend([
            MenuEntry::new("Edit", IconName::SquarePen, SyndicateAction::Edit(id)),
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                SyndicateAction::Details(id),
            ),
            MenuEntry::new("Copy name", IconName::Copy, SyndicateAction::CopyName(id)).separated(),
            MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                SyndicateAction::OpenMarket(id),
            ),
            MenuEntry::new("Delete", IconName::Trash, SyndicateAction::Delete(vec![id]))
                .separated(),
        ]);
        menu
    }

    fn panel(row: &SyndicateItem, cx: &App) -> PanelModel<SyndicateAction> {
        let theme = Theme::global(cx);
        let id = row.id as u64;
        PanelModel {
            title: row.item_name.clone(),
            subtitle: match rank_label(&row.sub_type) {
                Some(rank) => format!("{rank}, {}", row.syndicate_name),
                None => row.syndicate_name.clone(),
            },
            status: Some((
                row.status.label().to_owned(),
                theme.status_color(row.status),
            )),
            note: Some(row.status.detail().to_owned()),
            history: row.price_history.iter().map(|check| check.price).collect(),
            lines: None,
            facts: vec![
                Fact::new("Standing cost", number(row.standing_cost)),
                Fact::amount("List price", row.list_price, "Not listed"),
                match rate(row) {
                    Some(rate) => Fact::new("Platinum per 1,000 standing", format!("{rate:.2}")),
                    None => {
                        Fact::new("Platinum per 1,000 standing", "Not listed").tone(Tone::Muted)
                    }
                },
                Fact::new("Last checked", day_and_time(&row.updated_at)),
            ],
            primary: vec![
                PanelAction::new("Sell", IconName::HandCoins, SyndicateAction::Sell(id)),
                PanelAction::new(
                    "Sell one at the list price",
                    IconName::Gavel,
                    SyndicateAction::SellAtList(id),
                )
                .enabled(row.list_price.is_some()),
            ],
            secondary: vec![
                PanelAction::new("Edit", IconName::SquarePen, SyndicateAction::Edit(id)),
                PanelAction::new("Copy name", IconName::Copy, SyndicateAction::CopyName(id)),
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    SyndicateAction::OpenMarket(id),
                ),
                PanelAction::new("Delete", IconName::Trash, SyndicateAction::Delete(vec![id]))
                    .danger(),
            ],
        }
    }
}
