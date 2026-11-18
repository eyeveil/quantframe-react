//! The Wish list tab: the items the live scraper keeps buy orders up for.

use std::cmp::Ordering;

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div, px};
use gpui_kit_assets::IconName;

use super::{
    StockData, bulk_toolbar, every_row, ids,
    items::{chosen_item, item_field},
    limit_cell, limit_field, load_stock, market_url, rank_label, status_chips, status_filter,
    stepped,
};
use crate::{
    data::{DataKind, DataSource},
    model::{NewStockItem, StockStatus, SubType, TradableItem, WishListItem, WishListUpdate},
    theme::Theme,
    ui::{
        IconButton, cells, day_and_time,
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
pub enum WishAction {
    Add,
    Create(NewStockItem),
    Buy(u64),
    Bought {
        id: u64,
        quantity: i64,
        total: i64,
    },
    BuyAtBid(u64),
    Edit(Vec<u64>),
    Save {
        ids: Vec<u64>,
        update: WishListUpdate,
    },
    StepMaxPrice {
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
pub struct WishList {
    data: StockData<WishListItem>,
    /// The cache's tradable items for the add dialog, loaded once.
    catalog: Vec<TradableItem>,
}

impl WishList {
    fn wish(&self, id: u64) -> Option<WishListItem> {
        self.data
            .rows
            .iter()
            .find(|wish| wish.id as u64 == id)
            .cloned()
    }

    fn edit_prompt(&self, ids: &[u64]) -> Option<FormSpec> {
        let single = match ids {
            [id] => Some(self.wish(*id)?),
            [] => return None,
            _ => None,
        };
        let properties = single.as_ref().map(|wish| wish.properties.clone());
        let quantity = Field::number(
            "quantity",
            "Quantity",
            single.as_ref().map(|wish| wish.quantity),
        )
        .min(1)
        .hint("How many you still want.");
        let max_price = Field::platinum(
            "max_price",
            "Maximum price",
            properties.as_ref().and_then(|p| p.max_price),
        )
        .optional()
        .half()
        .hint("The scraper never bids above it.");
        let min_price = Field::platinum(
            "min_price",
            "Minimum price",
            properties.as_ref().and_then(|p| p.min_price),
        )
        .optional()
        .half()
        .hint("The scraper never bids below it.");
        Some(match &single {
            Some(wish) => {
                let spec = FormSpec::new(format!("Edit {}", wish.item_name), "Save")
                    .width(480.)
                    .field(quantity)
                    .field(max_price)
                    .field(min_price);
                if self.bulk_tradable(wish) {
                    spec.field(Field::toggle(
                        "is_bulk",
                        "Trade in batches",
                        wish.properties.is_bulk,
                    ))
                } else {
                    spec
                }
            }
            None => FormSpec::new(format!("Edit {} wishes", ids.len()), "Save")
                .width(480.)
                .description(
                    "Fill in only the fields to change. Empty fields keep each wish's value.",
                )
                .field(quantity.optional())
                .field(max_price)
                .field(min_price),
        })
    }

    fn bulk_tradable(&self, wish: &WishListItem) -> bool {
        self.catalog
            .iter()
            .any(|entry| entry.wfm_url == wish.wfm_url && entry.bulk_tradable)
    }

    fn saved(ids: &[u64], values: &FormValues, bulk: bool) -> WishAction {
        let single = ids.len() == 1;
        WishAction::Save {
            ids: ids.to_vec(),
            update: WishListUpdate {
                quantity: values.number("quantity"),
                max_price: limit_field(values, "max_price", single),
                min_price: limit_field(values, "min_price", single),
                is_bulk: (single && bulk).then(|| values.flag("is_bulk")),
                ..WishListUpdate::default()
            },
        }
    }

    fn message_items(&self, ids: &[u64]) -> Vec<MessageItem> {
        ids.iter()
            .filter_map(|id| self.wish(*id))
            .filter(|wish| wish.list_price.is_some())
            .map(|wish| MessageItem {
                rank: rank_label(&wish.sub_type),
                price: wish.list_price,
                name: wish.item_name,
            })
            .collect()
    }
}

impl Catalog for WishList {
    type Row = WishListItem;
    type Action = WishAction;
    type Data = (StockData<WishListItem>, Option<Vec<TradableItem>>);

    const NOUN: &'static str = "wishes";
    const SEARCH_PLACEHOLDER: &'static str = "Search the wish list";
    const EMPTY_HINT: &'static str =
        "Add an item you want and the live scraper will keep a buy order up for it.";
    const WATCH: &'static [DataKind] = &[DataKind::WishList];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        with_cache(
            load_stock(
                source.get_wish_list_pagination(&every_row()),
                source.get_wish_list_status_counts(),
                source.get_wish_list_financial_report(),
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
            ColumnSpec::new("rank", "Rank", 120.),
            ColumnSpec::new("quantity", "Quantity", 96.)
                .end()
                .sortable(),
            ColumnSpec::new("max_price", "Max price", 124.)
                .sortable()
                .droppable(2),
            ColumnSpec::new("list_price", "Bid", 96.).end().sortable(),
            ColumnSpec::new("total", "Cost at the bid", 124.)
                .end()
                .sortable()
                .droppable(1),
            ColumnSpec::new("status", "Status", 168.).sortable(),
            ColumnSpec::new("actions", "", 184.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<WishListItem> {
        let status = status_filter(query);
        self.data
            .rows
            .iter()
            .filter(|wish| status.is_none_or(|status| wish.status == status))
            .filter(|wish| query.matches(&wish.item_name))
            .cloned()
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        vec![status_chips(&self.data.counts, cx)]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = &self.data.rows;
        let units: i64 = rows.iter().map(|wish| wish.quantity).sum();
        let bids: i64 = rows
            .iter()
            .map(|wish| wish.list_price.unwrap_or(0) * wish.quantity)
            .sum();
        let live = self
            .data
            .counts
            .get(&StockStatus::Live)
            .copied()
            .unwrap_or(0);
        vec![
            Stat {
                label: "Bids",
                value: number(bids),
                note: "platinum if every bid fills".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Units",
                value: number(units),
                note: "still wanted".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Wishes",
                value: number(rows.len() as i64),
                note: format!("{live} live on Warframe Market"),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, checked: &[u64]) -> Vec<ToolbarItem<WishAction>> {
        bulk_toolbar(
            checked,
            "wishes",
            WishAction::Export,
            Some(WishAction::Edit),
            Some(WishAction::Message),
            WishAction::Delete,
        )
    }

    fn primary(&self) -> Option<PrimaryAction<WishAction>> {
        Some(PrimaryAction {
            label: "Add wish",
            icon: IconName::Plus,
            action: WishAction::Add,
        })
    }

    fn perform(&mut self, action: WishAction, source: &DataSource) -> Outcome<WishAction> {
        match action {
            WishAction::Add => Outcome::ask(
                FormSpec::new("Add wish", "Add wish")
                    .field(item_field(&self.catalog, "Wisp Prime Set"))
                    .field(
                        Field::number("rank", "Rank", None)
                            .optional()
                            .half()
                            .hint("For mods and arcanes."),
                    )
                    .field(Field::number("quantity", "Quantity", Some(1)).min(1).half()),
                |values| {
                    Some(WishAction::Create(NewStockItem {
                        raw: chosen_item(values)?,
                        sub_type: values.number("rank").map(SubType::rank),
                        quantity: values.number("quantity")?,
                        bought: 0,
                    }))
                },
            ),
            WishAction::Create(input) => {
                let name = self
                    .catalog
                    .iter()
                    .find(|item| item.wfm_url == input.raw)
                    .map_or(input.raw.clone(), |item| item.name.clone());
                let notice = format!("Added {name} to the wish list.");
                Outcome::run(source.wish_list_create(input), notice)
            }
            WishAction::Buy(id) => match self.wish(id) {
                Some(wish) => Outcome::ask(
                    prompts::bought(
                        &wish.item_name,
                        Some(wish.quantity),
                        wish.list_price.map(|price| price * wish.quantity),
                    ),
                    move |values| {
                        Some(WishAction::Bought {
                            id,
                            quantity: values.number("quantity")?,
                            total: values.number("price")?,
                        })
                    },
                ),
                None => Outcome::Done,
            },
            WishAction::BuyAtBid(id) => match self.wish(id).and_then(|wish| wish.list_price) {
                Some(price) => self.perform(
                    WishAction::Bought {
                        id,
                        quantity: 1,
                        total: price,
                    },
                    source,
                ),
                None => Outcome::Done,
            },
            WishAction::Bought {
                id,
                quantity,
                total,
            } => match self.wish(id) {
                Some(wish) => {
                    let notice = format!(
                        "Bought {} {} for {} platinum.",
                        number(quantity),
                        wish.item_name,
                        number(total)
                    );
                    Outcome::run(
                        source.wish_list_bought(wish.wfm_url, wish.sub_type, quantity, total),
                        notice,
                    )
                }
                None => Outcome::Done,
            },
            WishAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => {
                    let bulk = match ids.as_slice() {
                        [id] => self.wish(*id).is_some_and(|wish| self.bulk_tradable(&wish)),
                        _ => false,
                    };
                    Outcome::ask(spec, move |values| Some(Self::saved(&ids, values, bulk)))
                }
                None => Outcome::Done,
            },
            WishAction::StepMaxPrice { id, step } => {
                match self
                    .wish(id)
                    .and_then(|wish| Some((wish.id, stepped(wish.properties.max_price, step)?)))
                {
                    Some((id, max_price)) => Outcome::run_quietly(source.wish_list_update(
                        id,
                        WishListUpdate {
                            max_price: Some(max_price),
                            ..WishListUpdate::default()
                        },
                    )),
                    None => Outcome::Done,
                }
            }
            WishAction::Save { ids: rows, update } => match rows.as_slice() {
                [id] => Outcome::run_quietly(source.wish_list_update(*id as i64, update)),
                _ => Outcome::run_quietly(source.wish_list_update_multiple(ids(&rows), update)),
            },
            WishAction::ToggleHidden(id) => match self.wish(id) {
                Some(wish) => Outcome::run_quietly(source.wish_list_update(
                    wish.id,
                    WishListUpdate {
                        is_hidden: Some(!wish.is_hidden),
                        ..WishListUpdate::default()
                    },
                )),
                None => Outcome::Done,
            },
            WishAction::Delete(rows) if rows.is_empty() => Outcome::Done,
            WishAction::Delete(rows) => {
                Outcome::ask(prompts::delete(rows.len(), "wish", "wishes"), move |_| {
                    Some(WishAction::Deleted(rows.clone()))
                })
            }
            WishAction::Deleted(rows) => match rows.as_slice() {
                [id] => Outcome::run_quietly(source.wish_list_delete(*id as i64)),
                _ => Outcome::run_quietly(source.wish_list_delete_multiple(ids(&rows))),
            },
            WishAction::Details(id) => Outcome::ShowPanel(id),
            WishAction::CopyName(id) => match self.wish(id) {
                Some(wish) => Outcome::Copy {
                    what: wish.item_name.clone(),
                    text: wish.item_name,
                },
                None => Outcome::Done,
            },
            WishAction::OpenMarket(id) => match self.wish(id) {
                Some(wish) => Outcome::OpenUrl(market_url(&wish.wfm_url)),
                None => Outcome::Done,
            },
            WishAction::Message(rows) => {
                let items = self.message_items(&rows);
                if items.is_empty() {
                    return Outcome::Notice(
                        "None of the selected wishes has a bid yet.".to_owned(),
                    );
                }
                Outcome::ask(prompts::message("WTB", items.clone()), move |values| {
                    Some(WishAction::CopyMessage(prompts::compose_message(
                        values, &items,
                    )))
                })
            }
            WishAction::CopyMessage(text) => Outcome::Copy {
                text,
                what: "the trade chat message".to_owned(),
            },
            WishAction::Export => {
                let source = source.clone();
                Outcome::export("wish-list.json", move |path| {
                    Box::pin(source.export_wish_list_json(&every_row(), path))
                })
            }
        }
    }

    fn row_id(row: &WishListItem) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &WishListItem, b: &WishListItem) -> Ordering {
        let total = |wish: &WishListItem| wish.list_price.map(|price| price * wish.quantity);
        match key {
            "quantity" => a.quantity.cmp(&b.quantity),
            "max_price" => a.properties.max_price.cmp(&b.properties.max_price),
            "list_price" => a.list_price.cmp(&b.list_price),
            "total" => total(a).cmp(&total(b)),
            "status" => a.status.label().cmp(b.status.label()),
            _ => a.item_name.cmp(&b.item_name),
        }
    }

    fn cell(row: &WishListItem, key: &str, emit: &Emit<WishAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
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
            "rank" => cells::text(rank_label(&row.sub_type).unwrap_or_default()),
            "quantity" => cells::amount(Some(row.quantity), "", cx),
            "max_price" => limit_cell(
                id,
                row.properties.max_price,
                |step| WishAction::StepMaxPrice { id, step },
                emit,
                cx,
            ),
            "list_price" => cells::amount(row.list_price, "No bid", cx),
            "total" => cells::amount(row.list_price.map(|price| price * row.quantity), "", cx),
            "status" => cells::status(theme.status_color(row.status), row.status.label(), cx),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(
                        div().pr(px(5.)).child(
                            row_button(("buy", id), "Bought", cx)
                                .on_click(emit.on_click(WishAction::Buy(id))),
                        ),
                    )
                    .child(
                        IconButton::new(
                            ("buy-bid", id),
                            IconName::ShoppingCart,
                            "Bought one at the bid price",
                        )
                        .in_row()
                        .disabled(row.list_price.is_none())
                        .on_click(emit.on_click(WishAction::BuyAtBid(id))),
                    )
                    .child(
                        IconButton::new(("edit", id), IconName::SquarePen, "Edit")
                            .in_row()
                            .on_click(emit.on_click(WishAction::Edit(vec![id]))),
                    )
                    .child(overflow_button(id, move || Self::row_menu(&menu_row), emit))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &WishListItem, key: &str) -> Option<String> {
        match key {
            "item" => Some(row.item_name.clone()),
            "rank" => rank_label(&row.sub_type),
            "quantity" => Some(number(row.quantity)),
            "max_price" => row.properties.max_price.map(number),
            "list_price" => row.list_price.map(number),
            "total" => row.list_price.map(|price| number(price * row.quantity)),
            "status" => Some(row.status.label().to_owned()),
            _ => None,
        }
    }

    fn row_menu(row: &WishListItem) -> Vec<MenuEntry<WishAction>> {
        let id = row.id as u64;
        let mut menu = vec![MenuEntry::new(
            "Bought",
            IconName::HandCoins,
            WishAction::Buy(id),
        )];
        if row.list_price.is_some() {
            menu.push(MenuEntry::new(
                "Bought one at the bid price",
                IconName::ShoppingCart,
                WishAction::BuyAtBid(id),
            ));
        }
        menu.extend([
            MenuEntry::new("Edit", IconName::SquarePen, WishAction::Edit(vec![id])),
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                WishAction::Details(id),
            ),
            if row.is_hidden {
                MenuEntry::new(
                    "Show on Warframe Market",
                    IconName::Eye,
                    WishAction::ToggleHidden(id),
                )
            } else {
                MenuEntry::new(
                    "Hide from Warframe Market",
                    IconName::EyeOff,
                    WishAction::ToggleHidden(id),
                )
            }
            .separated(),
            MenuEntry::new(
                "Write a trade chat message",
                IconName::MessageSquare,
                WishAction::Message(vec![id]),
            ),
            MenuEntry::new("Copy name", IconName::Copy, WishAction::CopyName(id)),
            MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                WishAction::OpenMarket(id),
            ),
            MenuEntry::new("Delete", IconName::Trash, WishAction::Delete(vec![id])).separated(),
        ]);
        menu
    }

    fn panel(row: &WishListItem, cx: &App) -> PanelModel<WishAction> {
        let theme = Theme::global(cx);
        let id = row.id as u64;
        PanelModel {
            title: row.item_name.clone(),
            subtitle: match rank_label(&row.sub_type) {
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
                Fact::new("Quantity", number(row.quantity)),
                Fact::amount("Bid", row.list_price, "No bid"),
                Fact::amount(
                    "Cost at the bid",
                    row.list_price.map(|price| price * row.quantity),
                    "No bid",
                ),
                Fact::amount("Maximum price", row.properties.max_price, "Not set"),
                Fact::amount("Minimum price", row.properties.min_price, "Not set"),
                Fact::new(
                    "Trades in batches",
                    if row.properties.is_bulk { "Yes" } else { "No" },
                ),
                Fact::new("Last checked", day_and_time(&row.updated_at)),
            ],
            primary: vec![
                PanelAction::new("Bought", IconName::HandCoins, WishAction::Buy(id)),
                PanelAction::new(
                    "Bought one at the bid price",
                    IconName::ShoppingCart,
                    WishAction::BuyAtBid(id),
                )
                .enabled(row.list_price.is_some()),
            ],
            secondary: vec![
                PanelAction::new("Edit", IconName::SquarePen, WishAction::Edit(vec![id])),
                if row.is_hidden {
                    PanelAction::new(
                        "Show on Warframe Market",
                        IconName::Eye,
                        WishAction::ToggleHidden(id),
                    )
                } else {
                    PanelAction::new(
                        "Hide from Warframe Market",
                        IconName::EyeOff,
                        WishAction::ToggleHidden(id),
                    )
                },
                PanelAction::new(
                    "Write a trade chat message",
                    IconName::MessageSquare,
                    WishAction::Message(vec![id]),
                )
                .enabled(row.list_price.is_some()),
                PanelAction::new("Copy name", IconName::Copy, WishAction::CopyName(id)),
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    WishAction::OpenMarket(id),
                ),
                PanelAction::new("Delete", IconName::Trash, WishAction::Delete(vec![id])).danger(),
            ],
        }
    }

    fn dimmed(row: &WishListItem) -> bool {
        row.is_hidden
    }
}
