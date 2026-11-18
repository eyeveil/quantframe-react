//! The Items tab: the stock the live scraper sells.

use std::cmp::Ordering;

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div, px};
use gpui_kit_assets::IconName;

use super::{
    StockData, bulk_toolbar, every_row, ids, limit_cell, limit_field, load_stock, market_url, slug,
    status_chips, status_filter, stepped,
};
use crate::{
    data::{DataKind, DataSource},
    model::{NewStockItem, StockItem, StockItemUpdate, StockStatus, SubType, TradableItem},
    theme::Theme,
    ui::{
        IconButton, cells, choices, day_and_time,
        form::{Field, FormSpec, FormValues},
        grid::{
            Catalog, ChipGroup, ColumnSpec, Emit, Load, MenuEntry, Outcome, PrimaryAction, Query,
            Stat, Tone, ToolbarItem, overflow_button, with_cache,
        },
        number,
        panel::{Fact, PanelAction, PanelModel},
        prompts::{self, MessageItem},
        row_button,
    },
};

#[derive(Clone, Debug)]
pub enum ItemAction {
    Add,
    Create(NewStockItem),
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
        update: StockItemUpdate,
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

#[derive(Default)]
pub struct Items {
    data: StockData<StockItem>,
    /// The cache's tradable items for the add dialog, loaded once.
    catalog: Vec<TradableItem>,
}

/// An item's add dialog: a select over the cache's items, or a typed name
/// when the cache list is missing.
pub(super) fn item_field(catalog: &[TradableItem], example: &'static str) -> Field {
    if catalog.is_empty() {
        Field::text("name", "Item", "", example).hint("The item's name on Warframe Market.")
    } else {
        Field::search("item", "Item", choices::items(catalog), None)
    }
}

/// The Warframe Market slug an add dialog chose or the user typed.
pub(super) fn chosen_item(values: &FormValues) -> Option<String> {
    match values.choice("item") {
        Some(slug) => Some(slug),
        None => Some(slug(&values.text("name"))).filter(|slug| !slug.is_empty()),
    }
}

impl Items {
    fn item(&self, id: u64) -> Option<StockItem> {
        self.data
            .rows
            .iter()
            .find(|item| item.id as u64 == id)
            .cloned()
    }

    fn bulk_tradable(&self, item: &StockItem) -> bool {
        self.catalog
            .iter()
            .any(|entry| entry.wfm_url == item.wfm_url && entry.bulk_tradable)
    }

    fn edit_prompt(&self, ids: &[u64]) -> Option<FormSpec> {
        let single = match ids {
            [id] => Some(self.item(*id)?),
            [] => return None,
            _ => None,
        };
        let properties = single.as_ref().map(|item| item.properties.clone());
        let bought = Field::platinum("bought", "Bought", single.as_ref().map(|item| item.bought))
            .hint("Average paid per unit.")
            .half();
        let owned = Field::number("owned", "Owned", single.as_ref().map(|item| item.owned))
            .min(1)
            .half();
        let min_price = Field::platinum(
            "min_price",
            "Minimum price",
            properties.as_ref().and_then(|p| p.min_price),
        )
        .optional()
        .hint("The scraper never lists below it. Empty sets no floor.");
        let min_profit = Field::platinum(
            "min_profit",
            "Minimum profit",
            properties.as_ref().and_then(|p| p.min_profit),
        )
        .min(-1)
        .optional()
        .half()
        .hint("Empty uses the scraper setting.");
        let min_sma = Field::platinum(
            "min_sma",
            "Minimum moving average",
            properties.as_ref().and_then(|p| p.min_sma),
        )
        .min(-1)
        .optional()
        .half()
        .hint("Empty uses the scraper setting.");
        Some(match &single {
            Some(item) => {
                let spec = FormSpec::new(format!("Edit {}", item.item_name), "Save")
                    .width(480.)
                    .field(bought)
                    .field(owned)
                    .field(min_price)
                    .field(min_profit)
                    .field(min_sma);
                if self.bulk_tradable(item) {
                    spec.field(Field::toggle(
                        "is_bulk",
                        "Trade in batches",
                        item.properties.is_bulk,
                    ))
                } else {
                    spec
                }
            }
            None => FormSpec::new(format!("Edit {} items", ids.len()), "Save")
                .width(480.)
                .description(
                    "Fill in only the fields to change. Empty fields keep each item's value.",
                )
                .field(bought.optional())
                .field(owned.optional())
                .field(min_price)
                .field(min_profit)
                .field(min_sma),
        })
    }

    fn saved(ids: &[u64], values: &FormValues, bulk: bool) -> ItemAction {
        let single = ids.len() == 1;
        ItemAction::Save {
            ids: ids.to_vec(),
            update: StockItemUpdate {
                bought: values.number("bought"),
                owned: values.number("owned"),
                min_price: limit_field(values, "min_price", single),
                min_profit: limit_field(values, "min_profit", single),
                min_sma: limit_field(values, "min_sma", single),
                is_bulk: (single && bulk).then(|| values.flag("is_bulk")),
                ..StockItemUpdate::default()
            },
        }
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
    type Data = (StockData<StockItem>, Option<Vec<TradableItem>>);

    const NOUN: &'static str = "stock items";
    const SEARCH_PLACEHOLDER: &'static str = "Search items";
    const EMPTY_HINT: &'static str =
        "Add an item you own and the live scraper will price and list it.";
    const WATCH: &'static [DataKind] = &[DataKind::StockItems];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        with_cache(
            load_stock(
                source.get_stock_item_pagination(&every_row()),
                source.get_stock_item_status_counts(),
                source.get_stock_item_financial_report(),
            ),
            self.catalog
                .is_empty()
                .then(|| source.cache_get_tradable_items()),
        )
    }

    fn apply(&mut self, (data, catalog): Self::Data) {
        self.data = data;
        if let Some(catalog) = catalog {
            self.catalog = catalog;
        }
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("item", "Item", 200.).flex().sortable(),
            ColumnSpec::new("rank", "Rank", 104.),
            ColumnSpec::new("bought", "Bought", 88.)
                .end()
                .sortable()
                .droppable(2),
            ColumnSpec::new("list_price", "List price", 104.)
                .end()
                .sortable(),
            ColumnSpec::new("min_price", "Min price", 124.)
                .sortable()
                .droppable(3),
            ColumnSpec::new("owned", "Owned", 78.)
                .end()
                .sortable()
                .droppable(1),
            ColumnSpec::new("status", "Status", 156.).sortable(),
            ColumnSpec::new("actions", "", 176.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<StockItem> {
        let status = status_filter(query);
        self.data
            .rows
            .iter()
            .filter(|item| status.is_none_or(|status| item.status == status))
            .filter(|item| query.matches(&item.item_name))
            .cloned()
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        vec![status_chips(&self.data.counts, cx)]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = &self.data.rows;
        let report = &self.data.report;
        let units: i64 = rows.iter().map(|item| item.owned).sum();
        let live = self
            .data
            .counts
            .get(&StockStatus::Live)
            .copied()
            .unwrap_or(0);
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
                note: format!("{:.1}% return on cost", report.roi),
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
            Some(ItemAction::Edit),
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

    fn perform(&mut self, action: ItemAction, source: &DataSource) -> Outcome<ItemAction> {
        match action {
            ItemAction::Add => Outcome::ask(
                FormSpec::new("Add item", "Add item")
                    .field(item_field(&self.catalog, "Arcane Energize"))
                    .field(
                        Field::number("rank", "Rank", None)
                            .optional()
                            .hint("For mods and arcanes."),
                    )
                    .field(Field::number("quantity", "Quantity", Some(1)).min(1).half())
                    .field(
                        Field::platinum("bought", "Total paid", Some(0))
                            .half()
                            .hint("For all units. 0 records no purchase."),
                    ),
                |values| {
                    Some(ItemAction::Create(NewStockItem {
                        raw: chosen_item(values)?,
                        sub_type: values.number("rank").map(SubType::rank),
                        quantity: values.number("quantity")?,
                        bought: values.number("bought")?,
                    }))
                },
            ),
            ItemAction::Create(input) => {
                let name = self
                    .catalog
                    .iter()
                    .find(|item| item.wfm_url == input.raw)
                    .map_or(input.raw.clone(), |item| item.name.clone());
                let notice = format!("Added {} {name} to stock.", number(input.quantity));
                Outcome::run(source.stock_item_create(input), notice)
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
                Some(price) => self.perform(
                    ItemAction::Sold {
                        id,
                        quantity: 1,
                        total: price,
                    },
                    source,
                ),
                None => Outcome::Done,
            },
            ItemAction::Sold {
                id,
                quantity,
                total,
            } => match self.item(id) {
                Some(item) => {
                    let notice = format!(
                        "Sold {} {} for {} platinum.",
                        number(quantity),
                        item.item_name,
                        number(total)
                    );
                    Outcome::run(
                        source.stock_item_sell(item.wfm_url, item.sub_type, quantity, total),
                        notice,
                    )
                }
                None => Outcome::Done,
            },
            ItemAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => {
                    let bulk = match ids.as_slice() {
                        [id] => self.item(*id).is_some_and(|item| self.bulk_tradable(&item)),
                        _ => false,
                    };
                    Outcome::ask(spec, move |values| Some(Self::saved(&ids, values, bulk)))
                }
                None => Outcome::Done,
            },
            ItemAction::StepMinPrice { id, step } => {
                match self
                    .item(id)
                    .and_then(|item| Some((item.id, stepped(item.properties.min_price, step)?)))
                {
                    Some((id, min_price)) => Outcome::run_quietly(source.stock_item_update(
                        id,
                        StockItemUpdate {
                            min_price: Some(min_price),
                            ..StockItemUpdate::default()
                        },
                    )),
                    None => Outcome::Done,
                }
            }
            ItemAction::Save { ids: rows, update } => match rows.as_slice() {
                [id] => Outcome::run_quietly(source.stock_item_update(*id as i64, update)),
                _ => Outcome::run_quietly(source.stock_item_update_multiple(ids(&rows), update)),
            },
            ItemAction::ToggleHidden(id) => match self.item(id) {
                Some(item) => Outcome::run_quietly(source.stock_item_update(
                    item.id,
                    StockItemUpdate {
                        is_hidden: Some(!item.is_hidden),
                        ..StockItemUpdate::default()
                    },
                )),
                None => Outcome::Done,
            },
            ItemAction::Delete(rows) if rows.is_empty() => Outcome::Done,
            ItemAction::Delete(rows) => Outcome::ask(
                prompts::delete(rows.len(), "stock item", "stock items"),
                move |_| Some(ItemAction::Deleted(rows.clone())),
            ),
            ItemAction::Deleted(rows) => match rows.as_slice() {
                [id] => Outcome::run_quietly(source.stock_item_delete(*id as i64)),
                _ => Outcome::run_quietly(source.stock_item_delete_multiple(ids(&rows))),
            },
            ItemAction::Details(id) => Outcome::ShowPanel(id),
            ItemAction::CopyName(id) => match self.item(id) {
                Some(item) => Outcome::Copy {
                    what: item.item_name.clone(),
                    text: item.item_name,
                },
                None => Outcome::Done,
            },
            ItemAction::OpenMarket(id) => match self.item(id) {
                Some(item) => Outcome::OpenUrl(market_url(&item.wfm_url)),
                None => Outcome::Done,
            },
            ItemAction::Message(rows) => {
                let items = self.message_items(&rows);
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
            ItemAction::Export => {
                let source = source.clone();
                Outcome::export("stock-items.json", move |path| {
                    Box::pin(source.export_stock_item_json(&every_row(), path))
                })
            }
        }
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
                if row.has_override() {
                    flags.push((
                        IconName::Tag,
                        "Has its own profit settings for the live scraper",
                    ));
                }
                cells::name(row.item_name.clone(), &flags, cx)
            }
            "rank" => cells::text(row.rank_label().unwrap_or_default()),
            "bought" => cells::amount(Some(row.bought), "", cx),
            "list_price" => cells::amount(row.list_price, "Not listed", cx),
            "min_price" => limit_cell(
                id,
                row.properties.min_price,
                |step| ItemAction::StepMinPrice { id, step },
                emit,
                cx,
            ),
            "owned" => cells::amount(Some(row.owned), "", cx),
            "status" => cells::status(theme.status_color(row.status), row.status.label(), cx),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .children(row.properties.cooldown.as_ref().map(|cooldown| {
                        IconButton::new(
                            ("cooldown", id),
                            IconName::Clock,
                            format!(
                                "On cooldown after a price drop until {}",
                                day_and_time(&cooldown.end_time)
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
                        IconButton::new(("edit", id), IconName::SquarePen, "Edit")
                            .in_row()
                            .on_click(emit.on_click(ItemAction::Edit(vec![id]))),
                    )
                    .child(overflow_button(id, move || Self::row_menu(&menu_row), emit))
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
        let mut menu = vec![MenuEntry::new(
            "Sell",
            IconName::HandCoins,
            ItemAction::Sell(id),
        )];
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
                "Show details",
                IconName::PanelRight,
                ItemAction::Details(id),
            ),
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
        let properties = &row.properties;
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
                Fact::amount("Minimum price", properties.min_price, "Not set"),
                Fact::amount("Minimum profit", properties.min_profit, "Scraper setting"),
                Fact::amount(
                    "Minimum moving average",
                    properties.min_sma,
                    "Scraper setting",
                ),
                Fact::new(
                    "Trades in batches",
                    if properties.is_bulk { "Yes" } else { "No" },
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
