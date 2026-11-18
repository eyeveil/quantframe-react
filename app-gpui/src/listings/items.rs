//! The Items tab: the stock the live scraper sells.

use std::cmp::Ordering;

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div, px};
use gpui_kit_assets::IconName;

use super::{
    EXPORT_NOT_CONNECTED, MIN_PRICE_STEP, bulk_toolbar, market_url, model::Store, status_chips,
    status_filter,
};
use crate::{
    model::{FinancialReport, StockItem, StockItemProperties, StockStatus, SubType},
    preview,
    theme::Theme,
    ui::{
        IconButton, cells, day_and_time,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ColumnSpec, Emit, MenuEntry, Outcome, PrimaryAction, Query, Stat,
            Tone, ToolbarItem, overflow_button,
        },
        number,
        panel::{Fact, PanelAction, PanelModel},
        percent,
        prompts::{self, MessageItem},
        row_button,
    },
};

/// The fields of the edit prompt. A bulk edit changes only the filled fields.
#[derive(Clone, Debug, Default)]
pub struct ItemEdit {
    bought: Option<i64>,
    owned: Option<i64>,
    min_price: Option<i64>,
    min_sma: Option<i64>,
    min_profit: Option<i64>,
    is_bulk: bool,
}

#[derive(Clone, Debug)]
pub enum ItemAction {
    Add,
    Create {
        name: String,
        rank: Option<i64>,
        quantity: i64,
        bought: i64,
    },
    Sell(u64),
    Sold {
        id: u64,
        quantity: i64,
        total: i64,
    },
    SellAtList(u64),
    Edit(Vec<u64>),
    Save {
        ids: Vec<u64>,
        edit: ItemEdit,
    },
    EditMinPrice(u64),
    SetMinPrice {
        id: u64,
        price: Option<i64>,
    },
    StepMinPrice {
        id: u64,
        step: i64,
    },
    ToggleHidden(u64),
    Delete(Vec<u64>),
    Deleted(Vec<u64>),
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
    Message(Vec<u64>),
    CopyMessage(String),
    Export,
}

pub struct Items {
    store: Store<StockItem>,
    /// The backend's report for the stock. A change made here recomputes
    /// the totals the summary shows until the backend sends a new report.
    report: FinancialReport,
}

impl Items {
    pub fn new(rows: Vec<StockItem>, report: FinancialReport) -> Self {
        Self {
            store: Store::new(rows),
            report,
        }
    }

    fn apply(&mut self, action: ItemAction) -> Outcome<ItemAction> {
        match action {
            ItemAction::Add => Outcome::ask(
                FormSpec::new("Add item", "Add item")
                    .field(Field::choice("name", "Item", preview::tradable_items(), None))
                    .field(
                        Field::number("rank", "Rank", None)
                            .optional()
                            .hint("For mods and arcanes."),
                    )
                    .field(Field::number("quantity", "Quantity", Some(1)).min(1).half())
                    .field(
                        Field::platinum("bought", "Bought", Some(0))
                            .half()
                            .hint("Per unit."),
                    ),
                |values| {
                    Some(ItemAction::Create {
                        name: values.choice("name")?,
                        rank: values.number("rank"),
                        quantity: values.number("quantity")?,
                        bought: values.number("bought")?,
                    })
                },
            ),
            ItemAction::Create {
                name,
                rank,
                quantity,
                bought,
            } => {
                self.store.insert(StockItem {
                    id: 0,
                    wfm_id: String::new(),
                    wfm_url: name.to_lowercase().replace(' ', "_"),
                    item_unique_name: String::new(),
                    item_name: name.clone(),
                    sub_type: rank.map(SubType::rank),
                    bought,
                    list_price: None,
                    owned: quantity,
                    is_hidden: false,
                    status: StockStatus::Pending,
                    price_history: Vec::new(),
                    updated_at: String::new(),
                    created_at: String::new(),
                    properties: StockItemProperties::default(),
                });
                Outcome::Notice(format!("Added {} {name} to stock.", number(quantity)))
            }
            ItemAction::Sell(id) => match self.item(id) {
                Some(item) => Outcome::ask(
                    prompts::sold(
                        &item.item_name,
                        Some(item.owned),
                        item.list_price.map(|price| price * item.owned),
                    ),
                    move |values| {
                        Some(ItemAction::Sold {
                            id,
                            quantity: values.number("quantity")?,
                            total: values.number("price")?,
                        })
                    },
                ),
                None => Outcome::Done,
            },
            ItemAction::SellAtList(id) => match self.item(id).and_then(|item| item.list_price) {
                Some(price) => self.apply(ItemAction::Sold {
                    id,
                    quantity: 1,
                    total: price,
                }),
                None => Outcome::Done,
            },
            ItemAction::Sold {
                id,
                quantity,
                total,
            } => match self.item(id) {
                Some(item) => {
                    let quantity = quantity.min(item.owned);
                    self.store.sell(id as i64, quantity);
                    Outcome::Notice(format!(
                        "Sold {} {} for {} platinum.",
                        number(quantity),
                        item.item_name,
                        number(total)
                    ))
                }
                None => Outcome::Done,
            },
            ItemAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => Outcome::ask(spec, move |values| {
                    Some(ItemAction::Save {
                        ids: ids.clone(),
                        edit: ItemEdit {
                            bought: values.number("bought"),
                            owned: values.number("owned"),
                            min_price: values.number("min_price"),
                            min_sma: values.number("min_sma"),
                            min_profit: values.number("min_profit"),
                            is_bulk: values.flag("is_bulk"),
                        },
                    })
                }),
                None => Outcome::Done,
            },
            ItemAction::Save { ids, edit } => {
                let single = ids.len() == 1;
                for id in &ids {
                    self.store.update(*id as i64, |item| {
                        item.bought = edit.bought.unwrap_or(item.bought);
                        item.owned = edit.owned.unwrap_or(item.owned);
                        let properties = &mut item.properties;
                        if single {
                            properties.min_price = edit.min_price;
                            properties.min_sma = edit.min_sma;
                            properties.min_profit = edit.min_profit;
                            properties.is_bulk = edit.is_bulk;
                        } else {
                            properties.min_price = edit.min_price.or(properties.min_price);
                            properties.min_sma = edit.min_sma.or(properties.min_sma);
                            properties.min_profit = edit.min_profit.or(properties.min_profit);
                        }
                    });
                }
                Outcome::Done
            }
            ItemAction::EditMinPrice(id) => match self.item(id) {
                Some(item) => Outcome::ask(
                    prompts::price_limit(
                        &format!("Minimum price for {}", item.item_name),
                        "Minimum price",
                        "The live scraper never lists below this. Leave it empty for no minimum.",
                        item.properties.min_price,
                    ),
                    move |values| {
                        Some(ItemAction::SetMinPrice {
                            id,
                            price: values.number("price"),
                        })
                    },
                ),
                None => Outcome::Done,
            },
            ItemAction::SetMinPrice { id, price } => {
                self.store.update(id as i64, |item| {
                    item.properties.min_price = price.filter(|price| *price > 0);
                });
                Outcome::Done
            }
            ItemAction::StepMinPrice { id, step } => {
                self.store.update(id as i64, |item| {
                    let next = item.properties.min_price.unwrap_or(0) + step;
                    item.properties.min_price = (next > 0).then_some(next);
                });
                Outcome::Done
            }
            ItemAction::ToggleHidden(id) => {
                self.store
                    .update(id as i64, |item| item.is_hidden = !item.is_hidden);
                Outcome::Done
            }
            ItemAction::Delete(ids) if ids.is_empty() => Outcome::Done,
            ItemAction::Delete(ids) => Outcome::ask(
                prompts::delete(ids.len(), "stock item", "stock items"),
                move |_| Some(ItemAction::Deleted(ids.clone())),
            ),
            ItemAction::Deleted(ids) => {
                let ids: Vec<i64> = ids.iter().map(|id| *id as i64).collect();
                self.store.delete(&ids);
                Outcome::Done
            }
            ItemAction::Details(id) => Outcome::ShowPanel(id),
            ItemAction::CopyName(id) => match self.item(id) {
                Some(item) => Outcome::Copy {
                    what: item.item_name.clone(),
                    text: item.item_name,
                },
                None => Outcome::Done,
            },
            ItemAction::OpenMarket(id) => match self.item(id) {
                Some(item) => Outcome::OpenUrl(market_url("items", &item.wfm_url)),
                None => Outcome::Done,
            },
            ItemAction::Message(ids) => {
                let items = self.message_items(&ids);
                if items.is_empty() {
                    return Outcome::Notice(
                        "None of the selected items has a list price yet.".to_owned(),
                    );
                }
                Outcome::ask(prompts::message("WTS", items.clone()), move |values| {
                    Some(ItemAction::CopyMessage(prompts::compose_message(
                        values, &items,
                    )))
                })
            }
            ItemAction::CopyMessage(text) => Outcome::Copy {
                text,
                what: "the trade chat message".to_owned(),
            },
            ItemAction::Export => Outcome::Notice(EXPORT_NOT_CONNECTED.to_owned()),
        }
    }

    fn recompute_report(&mut self) {
        let rows = self.store.rows();
        let expenses: i64 = rows.iter().map(|item| item.bought * item.owned).sum();
        let revenue: i64 = rows
            .iter()
            .map(|item| item.list_price.unwrap_or(0) * item.owned)
            .sum();
        let total_profit = revenue - expenses;
        self.report = FinancialReport {
            expenses,
            revenue,
            total_profit,
            roi: if expenses > 0 {
                total_profit as f64 / expenses as f64
            } else {
                0.0
            },
            ..self.report.clone()
        };
    }

    fn item(&self, id: u64) -> Option<StockItem> {
        self.store.get(id as i64).cloned()
    }

    fn edit_prompt(&self, ids: &[u64]) -> Option<FormSpec> {
        let single = match ids {
            [id] => Some(self.item(*id)?),
            [] => return None,
            _ => None,
        };
        let value = |pick: fn(&StockItem) -> Option<i64>| single.as_ref().and_then(pick);
        let bulk = single.is_none();
        let optional = |field: Field| if bulk { field.optional() } else { field };
        let spec = match &single {
            Some(item) => FormSpec::new(format!("Edit {}", item.item_name), "Save"),
            None => FormSpec::new(format!("Edit {} items", ids.len()), "Save")
                .description("Fill in only the fields to change. Empty fields keep each item's value."),
        };
        let spec = spec
            .width(480.)
            .field(optional(
                Field::platinum("bought", "Bought", value(|item| Some(item.bought))).half(),
            ))
            .field(optional(
                Field::number("owned", "Owned", value(|item| Some(item.owned)))
                    .min(1)
                    .half(),
            ))
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
            );
        Some(match &single {
            Some(item) => spec.field(
                Field::toggle("is_bulk", "Trade in batches", item.properties.is_bulk)
                    .hint("Lists up to 6 units in one trade."),
            ),
            None => spec,
        })
    }

    fn message_items(&self, ids: &[u64]) -> Vec<MessageItem> {
        ids.iter()
            .filter_map(|id| self.item(*id))
            .filter(|item| item.list_price.is_some())
            .map(|item| MessageItem {
                rank: item.rank_label(),
                price: item.list_price,
                name: item.item_name,
            })
            .collect()
    }
}

impl Catalog for Items {
    type Row = StockItem;
    type Action = ItemAction;

    const NOUN: &'static str = "stock items";
    const SEARCH_PLACEHOLDER: &'static str = "Search items";
    const EMPTY_HINT: &'static str = "Add an item you own and the live scraper will price and list it.";

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("item", "Item", 200.).flex().sortable(),
            ColumnSpec::new("rank", "Rank", 104.),
            ColumnSpec::new("bought", "Bought", 88.).end().sortable().droppable(2),
            ColumnSpec::new("list_price", "List price", 104.).end().sortable(),
            ColumnSpec::new("min_price", "Min price", 136.).sortable().droppable(3),
            ColumnSpec::new("owned", "Owned", 78.).end().sortable().droppable(1),
            ColumnSpec::new("status", "Status", 156.).sortable(),
            ColumnSpec::new("actions", "", 176.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<StockItem> {
        self.store
            .list(&query.text, status_filter(&self.store, query))
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        vec![status_chips(&self.store, cx)]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = self.store.rows();
        let units: i64 = rows.iter().map(|item| item.owned).sum();
        let report = &self.report;
        let live = rows
            .iter()
            .filter(|item| item.status == StockStatus::Live)
            .count();
        vec![
            Stat {
                label: "Bought",
                value: number(report.expenses),
                note: format!("{} units in stock", number(units)),
                tone: Tone::Plain,
            },
            Stat {
                label: "Listed",
                value: number(report.revenue),
                note: "at current list prices".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Profit",
                value: number(report.total_profit),
                note: format!("{} return on cost", percent(report.roi)),
                tone: Tone::of_profit(report.total_profit),
            },
            Stat {
                label: "Items",
                value: number(rows.len() as i64),
                note: format!("{live} live on Warframe Market"),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, checked: &[u64]) -> Vec<ToolbarItem<ItemAction>> {
        bulk_toolbar(
            checked,
            "items",
            ItemAction::Export,
            ItemAction::Edit,
            Some(ItemAction::Message),
            ItemAction::Delete,
        )
    }

    fn primary(&self) -> Option<PrimaryAction<ItemAction>> {
        Some(PrimaryAction {
            label: "Add item",
            icon: IconName::Plus,
            action: ItemAction::Add,
        })
    }

    fn perform(&mut self, action: ItemAction) -> Outcome<ItemAction> {
        let outcome = self.apply(action);
        self.recompute_report();
        outcome
    }

    fn row_id(row: &StockItem) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &StockItem, b: &StockItem) -> Ordering {
        match key {
            "bought" => a.bought.cmp(&b.bought),
            "list_price" => a.list_price.cmp(&b.list_price),
            "min_price" => a.properties.min_price.cmp(&b.properties.min_price),
            "owned" => a.owned.cmp(&b.owned),
            "status" => a.status.label().cmp(b.status.label()),
            _ => a.item_name.cmp(&b.item_name),
        }
    }

    fn cell(row: &StockItem, key: &str, emit: &Emit<ItemAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let id = row.id as u64;
        match key {
            "item" => {
                let mut flags = Vec::new();
                if row.is_hidden {
                    flags.push((IconName::EyeOff, "Hidden from Warframe Market"));
                }
                if row.properties.is_bulk {
                    flags.push((IconName::Handshake, "Trades in batches"));
                }
                cells::name(row.item_name.clone(), &flags, cx)
            }
            "rank" => cells::text(row.rank_label().unwrap_or_default()),
            "bought" => cells::amount(Some(row.bought), "", cx),
            "list_price" => cells::amount(row.list_price, "Not listed", cx),
            "min_price" => cells::stepped_price(
                id,
                row.properties.min_price,
                MIN_PRICE_STEP,
                ItemAction::StepMinPrice {
                    id,
                    step: -MIN_PRICE_STEP,
                },
                ItemAction::StepMinPrice {
                    id,
                    step: MIN_PRICE_STEP,
                },
                emit,
                cx,
            ),
            "owned" => cells::amount(Some(row.owned), "", cx),
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
                    .children(row.properties.cooldown.as_ref().map(|cooldown| {
                        IconButton::new(
                            ("cooldown", id),
                            IconName::Clock,
                            format!(
                                "On cooldown after a price drop until {}",
                                cooldown.end_time.get(11..16).unwrap_or("later")
                            ),
                        )
                        .in_row()
                        .tone(colors.warning)
                        .hover_tone(colors.warning)
                    }))
                    .child(
                        div().pr(px(5.)).child(
                            row_button(("sell", id), "Sell", cx)
                                .on_click(emit.on_click(ItemAction::Sell(id))),
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
                        .on_click(emit.on_click(ItemAction::SellAtList(id))),
                    )
                    .child(
                        edit.in_row()
                            .on_click(emit.on_click(ItemAction::Edit(vec![id]))),
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

    fn cell_text(row: &StockItem, key: &str) -> Option<String> {
        match key {
            "item" => Some(row.item_name.clone()),
            "rank" => row.rank_label(),
            "bought" => Some(number(row.bought)),
            "list_price" => row.list_price.map(number),
            "min_price" => row.properties.min_price.map(number),
            "owned" => Some(number(row.owned)),
            "status" => Some(row.status.label().to_owned()),
            _ => None,
        }
    }

    fn row_menu(row: &StockItem) -> Vec<MenuEntry<ItemAction>> {
        let id = row.id as u64;
        let mut menu = vec![MenuEntry::new("Sell", IconName::HandCoins, ItemAction::Sell(id))];
        if row.list_price.is_some() {
            menu.push(MenuEntry::new(
                "Sell one at the list price",
                IconName::Gavel,
                ItemAction::SellAtList(id),
            ));
        }
        menu.extend([
            MenuEntry::new("Edit", IconName::SquarePen, ItemAction::Edit(vec![id])),
            MenuEntry::new(
                "Set minimum price",
                IconName::Tag,
                ItemAction::EditMinPrice(id),
            ),
            MenuEntry::new("Show details", IconName::PanelRight, ItemAction::Details(id)),
            if row.is_hidden {
                MenuEntry::new(
                    "Show on Warframe Market",
                    IconName::Eye,
                    ItemAction::ToggleHidden(id),
                )
            } else {
                MenuEntry::new(
                    "Hide from Warframe Market",
                    IconName::EyeOff,
                    ItemAction::ToggleHidden(id),
                )
            }
            .separated(),
            MenuEntry::new(
                "Write a trade chat message",
                IconName::MessageSquare,
                ItemAction::Message(vec![id]),
            ),
            MenuEntry::new("Copy name", IconName::Copy, ItemAction::CopyName(id)),
            MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                ItemAction::OpenMarket(id),
            ),
            MenuEntry::new("Delete", IconName::Trash, ItemAction::Delete(vec![id])).separated(),
        ]);
        menu
    }

    fn panel(row: &StockItem, cx: &App) -> PanelModel<ItemAction> {
        let theme = Theme::global(cx);
        let id = row.id as u64;
        let profit = row.profit_per_unit();
        PanelModel {
            title: row.item_name.clone(),
            subtitle: match row.rank_label() {
                Some(rank) => format!("{rank}, {}", row.wfm_url),
                None => row.wfm_url.clone(),
            },
            status: Some((
                row.status.label().to_owned(),
                theme.status_color(row.status),
            )),
            note: Some(row.status.detail().to_owned()),
            history: row.price_history.iter().map(|check| check.price).collect(),
            lines: None,
            facts: vec![
                Fact::new("Bought", number(row.bought)),
                Fact::amount("List price", row.list_price, "Not listed"),
                Fact::amount("Minimum price", row.properties.min_price, "Not set"),
                match profit {
                    Some(profit) => {
                        Fact::new("Profit per unit", number(profit)).tone(Tone::of_profit(profit))
                    }
                    None => Fact::new("Profit per unit", "Not listed").tone(Tone::Muted),
                },
                Fact::new("Owned", number(row.owned)),
                Fact::amount(
                    "Stock value",
                    row.list_price.map(|price| price * row.owned),
                    "Not listed",
                ),
                Fact::new("Last checked", day_and_time(&row.updated_at)),
            ],
            primary: vec![
                PanelAction::new("Sell", IconName::HandCoins, ItemAction::Sell(id)),
                PanelAction::new(
                    "Sell one at the list price",
                    IconName::Gavel,
                    ItemAction::SellAtList(id),
                )
                .enabled(row.list_price.is_some()),
            ],
            secondary: vec![
                PanelAction::new("Edit", IconName::SquarePen, ItemAction::Edit(vec![id])),
                PanelAction::new(
                    "Set minimum price",
                    IconName::Tag,
                    ItemAction::EditMinPrice(id),
                ),
                if row.is_hidden {
                    PanelAction::new(
                        "Show on Warframe Market",
                        IconName::Eye,
                        ItemAction::ToggleHidden(id),
                    )
                } else {
                    PanelAction::new(
                        "Hide from Warframe Market",
                        IconName::EyeOff,
                        ItemAction::ToggleHidden(id),
                    )
                },
                PanelAction::new(
                    "Write a trade chat message",
                    IconName::MessageSquare,
                    ItemAction::Message(vec![id]),
                )
                .enabled(row.list_price.is_some()),
                PanelAction::new("Copy name", IconName::Copy, ItemAction::CopyName(id)),
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    ItemAction::OpenMarket(id),
                ),
                PanelAction::new("Delete", IconName::Trash, ItemAction::Delete(vec![id])).danger(),
            ],
        }
    }

    fn dimmed(row: &StockItem) -> bool {
        row.is_hidden
    }
}
