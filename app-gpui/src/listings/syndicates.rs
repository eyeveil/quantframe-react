//! The Syndicates tab: what the syndicates sell for standing, listed on
//! Warframe Market by the live scraper.

use std::cmp::Ordering;

use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement, Styled, div, px, rgb};
use gpui_kit_assets::IconName;

use super::{
    EXPORT_NOT_CONNECTED, MIN_PRICE_STEP, bulk_toolbar, market_url,
    model::{Store, SyndicateItem},
    status_chips, status_filter,
};
use crate::{
    model::StockStatus,
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

#[derive(Clone, Debug, Default)]
pub struct SyndicateEdit {
    min_price: Option<i64>,
    min_profit: Option<i64>,
    min_sma: Option<i64>,
}

#[derive(Clone, Debug)]
pub enum SyndicateAction {
    Import,
    Sell(u64),
    Sold { id: u64, price: i64 },
    SellAtList(u64),
    Edit(Vec<u64>),
    Save { ids: Vec<u64>, edit: SyndicateEdit },
    EditMinPrice(u64),
    SetMinPrice { id: u64, price: Option<i64> },
    StepMinPrice { id: u64, step: i64 },
    Delete(Vec<u64>),
    Deleted(Vec<u64>),
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
    Export,
}

pub struct Syndicates {
    store: Store<SyndicateItem>,
}

/// The colour each syndicate flies in the game.
fn syndicate_color(name: &str) -> Hsla {
    let hex = match name {
        "Steel Meridian" => 0xc4622d,
        "Arbiters of Hexis" => 0x9aa3ad,
        "Cephalon Suda" => 0x4f7fd9,
        "The Perrin Sequence" => 0x3fa7b5,
        "Red Veil" => 0xc2413b,
        "New Loka" => 0x5aa55f,
        _ => 0x8e8e96,
    };
    rgb(hex).into()
}

fn platinum_per_standing(item: &SyndicateItem) -> Option<String> {
    item.platinum_per_thousand_standing()
        .map(|rate| format!("{rate:.2}"))
}

impl Syndicates {
    pub fn new(rows: Vec<SyndicateItem>) -> Self {
        Self {
            store: Store::new(rows),
        }
    }

    fn item(&self, id: u64) -> Option<SyndicateItem> {
        self.store.get(id as i64).cloned()
    }

    fn edit_prompt(&self, ids: &[u64]) -> Option<FormSpec> {
        let single = match ids {
            [id] => Some(self.item(*id)?),
            [] => return None,
            _ => None,
        };
        let value = |pick: fn(&SyndicateItem) -> Option<i64>| single.as_ref().and_then(pick);
        let spec = match &single {
            Some(item) => FormSpec::new(format!("Edit {}", item.item_name), "Save"),
            None => FormSpec::new(format!("Edit {} syndicate items", ids.len()), "Save")
                .description(
                    "Fill in only the fields to change. Empty fields keep each item's value.",
                ),
        };
        Some(
            spec.width(480.)
                .field(
                    Field::platinum(
                        "min_price",
                        "Minimum price",
                        value(|item| item.properties.min_price),
                    )
                    .optional()
                    .hint("The live scraper never lists below this."),
                )
                .field(
                    Field::platinum(
                        "min_profit",
                        "Minimum profit",
                        value(|item| item.properties.min_profit),
                    )
                    .optional()
                    .half()
                    .hint("Replaces the live scraper setting."),
                )
                .field(
                    Field::number(
                        "min_sma",
                        "Minimum moving average",
                        value(|item| item.properties.min_sma),
                    )
                    .optional()
                    .half()
                    .hint("Replaces the live scraper setting."),
                ),
        )
    }
}

impl Catalog for Syndicates {
    type Row = SyndicateItem;
    type Action = SyndicateAction;

    const NOUN: &'static str = "syndicate items";
    const SEARCH_PLACEHOLDER: &'static str = "Search syndicate items";
    const EMPTY_HINT: &'static str =
        "Import the syndicate offerings and the live scraper will price them for you.";

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("item", "Item", 180.).flex().sortable(),
            ColumnSpec::new("syndicate", "Syndicate", 176.).sortable(),
            ColumnSpec::new("standing", "Standing", 96.).end().sortable().droppable(1),
            ColumnSpec::new("list_price", "List price", 100.).end().sortable(),
            ColumnSpec::new("rate", "Per 1,000 standing", 136.).end().sortable().droppable(2),
            ColumnSpec::new("min_price", "Min price", 136.).sortable().droppable(3),
            ColumnSpec::new("status", "Status", 168.).sortable(),
            ColumnSpec::new("actions", "", 150.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<SyndicateItem> {
        let syndicate = query.filter("syndicate");
        self.store
            .list(&query.text, status_filter(&self.store, query))
            .into_iter()
            .filter(|item| syndicate.is_none_or(|name| item.syndicate_name == name))
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        vec![
            status_chips(&self.store, cx),
            ChipGroup {
                id: "syndicate",
                chips: self
                    .store
                    .syndicate_counts()
                    .into_iter()
                    .map(|(name, count)| ChipSpec {
                        color: Some(syndicate_color(&name)),
                        key: name.clone(),
                        label: name,
                        count,
                        tooltip: String::new(),
                    })
                    .collect(),
            },
        ]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = self.store.rows();
        let listed: i64 = rows.iter().filter_map(|item| item.list_price).sum();
        let standing: i64 = rows
            .iter()
            .filter(|item| item.list_price.is_some())
            .map(|item| item.standing_cost)
            .sum();
        let best = rows
            .iter()
            .filter_map(|item| Some((item.platinum_per_thousand_standing()?, item)))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        let live = rows
            .iter()
            .filter(|item| item.status == StockStatus::Live)
            .count();
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
                tone: Tone::Positive,
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
            SyndicateAction::Edit,
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

    fn perform(&mut self, action: SyndicateAction) -> Outcome<SyndicateAction> {
        match action {
            SyndicateAction::Import => Outcome::Notice(
                "Importing syndicate offerings isn't connected to the backend yet.".to_owned(),
            ),
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
                    Some(price) => self.perform(SyndicateAction::Sold { id, price }),
                    None => Outcome::Done,
                }
            }
            SyndicateAction::Sold { id, price } => match self.item(id) {
                Some(item) => Outcome::Notice(format!(
                    "Sold {} for {} platinum.",
                    item.item_name,
                    number(price)
                )),
                None => Outcome::Done,
            },
            SyndicateAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => Outcome::ask(spec, move |values| {
                    Some(SyndicateAction::Save {
                        ids: ids.clone(),
                        edit: SyndicateEdit {
                            min_price: values.number("min_price"),
                            min_profit: values.number("min_profit"),
                            min_sma: values.number("min_sma"),
                        },
                    })
                }),
                None => Outcome::Done,
            },
            SyndicateAction::Save { ids, edit } => {
                let single = ids.len() == 1;
                for id in &ids {
                    self.store.update(*id as i64, |item| {
                        let properties = &mut item.properties;
                        if single {
                            properties.min_price = edit.min_price;
                            properties.min_profit = edit.min_profit;
                            properties.min_sma = edit.min_sma;
                        } else {
                            properties.min_price = edit.min_price.or(properties.min_price);
                            properties.min_profit = edit.min_profit.or(properties.min_profit);
                            properties.min_sma = edit.min_sma.or(properties.min_sma);
                        }
                    });
                }
                Outcome::Done
            }
            SyndicateAction::EditMinPrice(id) => match self.item(id) {
                Some(item) => Outcome::ask(
                    prompts::price_limit(
                        &format!("Minimum price for {}", item.item_name),
                        "Minimum price",
                        "The live scraper never lists below this. Leave it empty for no minimum.",
                        item.properties.min_price,
                    ),
                    move |values| {
                        Some(SyndicateAction::SetMinPrice {
                            id,
                            price: values.number("price"),
                        })
                    },
                ),
                None => Outcome::Done,
            },
            SyndicateAction::SetMinPrice { id, price } => {
                self.store.update(id as i64, |item| {
                    item.properties.min_price = price.filter(|price| *price > 0);
                });
                Outcome::Done
            }
            SyndicateAction::StepMinPrice { id, step } => {
                self.store.update(id as i64, |item| {
                    let next = item.properties.min_price.unwrap_or(0) + step;
                    item.properties.min_price = (next > 0).then_some(next);
                });
                Outcome::Done
            }
            SyndicateAction::Delete(ids) if ids.is_empty() => Outcome::Done,
            SyndicateAction::Delete(ids) => Outcome::ask(
                prompts::delete(ids.len(), "syndicate item", "syndicate items"),
                move |_| Some(SyndicateAction::Deleted(ids.clone())),
            ),
            SyndicateAction::Deleted(ids) => {
                let ids: Vec<i64> = ids.iter().map(|id| *id as i64).collect();
                self.store.delete(&ids);
                Outcome::Done
            }
            SyndicateAction::Details(id) => Outcome::ShowPanel(id),
            SyndicateAction::CopyName(id) => match self.item(id) {
                Some(item) => Outcome::Copy {
                    what: item.item_name.clone(),
                    text: item.item_name,
                },
                None => Outcome::Done,
            },
            SyndicateAction::OpenMarket(id) => match self.item(id) {
                Some(item) => Outcome::OpenUrl(market_url("items", &item.wfm_url)),
                None => Outcome::Done,
            },
            SyndicateAction::Export => Outcome::Notice(EXPORT_NOT_CONNECTED.to_owned()),
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
            "rate" => a
                .platinum_per_thousand_standing()
                .unwrap_or(-1.0)
                .total_cmp(&b.platinum_per_thousand_standing().unwrap_or(-1.0)),
            "min_price" => a.properties.min_price.cmp(&b.properties.min_price),
            "status" => a.status.label().cmp(b.status.label()),
            _ => a.item_name.cmp(&b.item_name),
        }
    }

    fn cell(row: &SyndicateItem, key: &str, emit: &Emit<SyndicateAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let id = row.id as u64;
        match key {
            "item" => cells::name(row.item_name.clone(), &[], cx),
            "syndicate" => cells::status(
                syndicate_color(&row.syndicate_name),
                row.syndicate_name.clone(),
                cx,
            ),
            "standing" => cells::amount(Some(row.standing_cost), "", cx),
            "list_price" => cells::amount(row.list_price, "Not listed", cx),
            "rate" => match platinum_per_standing(row) {
                Some(rate) => crate::ui::numeric(rate)
                    .text_color(colors.text)
                    .into_any_element(),
                None => cells::amount(None, "", cx),
            },
            "min_price" => cells::stepped_price(
                id,
                row.properties.min_price,
                MIN_PRICE_STEP,
                SyndicateAction::StepMinPrice {
                    id,
                    step: -MIN_PRICE_STEP,
                },
                SyndicateAction::StepMinPrice {
                    id,
                    step: MIN_PRICE_STEP,
                },
                emit,
                cx,
            ),
            "status" => cells::status(theme.status_color(row.status), row.status.label(), cx),
            _ => {
                let edit = if row.has_override() {
                    IconButton::new(
                        ("edit", id),
                        IconName::SquarePen,
                        "Edit. This item replaces the live scraper's profit settings.",
                    )
                    .tone(colors.warning)
                    .hover_tone(colors.warning)
                } else {
                    IconButton::new(("edit", id), IconName::SquarePen, "Edit")
                };
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
                        edit.in_row()
                            .on_click(emit.on_click(SyndicateAction::Edit(vec![id]))),
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
            "syndicate" => Some(row.syndicate_name.clone()),
            "standing" => Some(number(row.standing_cost)),
            "list_price" => row.list_price.map(number),
            "rate" => platinum_per_standing(row),
            "min_price" => row.properties.min_price.map(number),
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
            MenuEntry::new("Edit", IconName::SquarePen, SyndicateAction::Edit(vec![id])),
            MenuEntry::new(
                "Set minimum price",
                IconName::Tag,
                SyndicateAction::EditMinPrice(id),
            ),
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
            subtitle: row.syndicate_name.clone(),
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
                match platinum_per_standing(row) {
                    Some(rate) => Fact::new("Platinum per 1,000 standing", rate),
                    None => Fact::new("Platinum per 1,000 standing", "Not listed").tone(Tone::Muted),
                },
                Fact::amount("Minimum price", row.properties.min_price, "Not set"),
                Fact::amount("Minimum profit", row.properties.min_profit, "Scraper setting"),
                Fact::amount(
                    "Minimum moving average",
                    row.properties.min_sma,
                    "Scraper setting",
                ),
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
                PanelAction::new("Edit", IconName::SquarePen, SyndicateAction::Edit(vec![id])),
                PanelAction::new(
                    "Set minimum price",
                    IconName::Tag,
                    SyndicateAction::EditMinPrice(id),
                ),
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
