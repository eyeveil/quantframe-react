//! The Wish list tab: the items the live scraper keeps buy orders up for.

use std::cmp::Ordering;

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div, px};
use gpui_kit_assets::IconName;

use super::{
    EXPORT_NOT_CONNECTED, MIN_PRICE_STEP, bulk_toolbar, market_url,
    model::{Store, WishListItem, WishListProperties},
    status_chips, status_filter,
};
use crate::{
    model::{StockStatus, SubType},
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
        prompts::{self, MessageItem},
        row_button,
    },
};

#[derive(Clone, Debug, Default)]
pub struct WishEdit {
    quantity: Option<i64>,
    max_price: Option<i64>,
    is_bulk: bool,
}

#[derive(Clone, Debug)]
pub enum WishAction {
    Add,
    Create {
        name: String,
        rank: Option<i64>,
        quantity: i64,
        max_price: Option<i64>,
    },
    Buy(u64),
    Bought {
        id: u64,
        quantity: i64,
        total: i64,
    },
    BuyAtList(u64),
    Edit(Vec<u64>),
    Save {
        ids: Vec<u64>,
        edit: WishEdit,
    },
    EditMaxPrice(u64),
    SetMaxPrice {
        id: u64,
        price: Option<i64>,
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

pub struct WishList {
    store: Store<WishListItem>,
}

impl WishList {
    pub fn new(rows: Vec<WishListItem>) -> Self {
        Self {
            store: Store::new(rows),
        }
    }

    fn wish(&self, id: u64) -> Option<WishListItem> {
        self.store.get(id as i64).cloned()
    }

    fn edit_prompt(&self, ids: &[u64]) -> Option<FormSpec> {
        let single = match ids {
            [id] => Some(self.wish(*id)?),
            [] => return None,
            _ => None,
        };
        let spec = match &single {
            Some(wish) => FormSpec::new(format!("Edit {}", wish.item_name), "Save"),
            None => FormSpec::new(format!("Edit {} wishes", ids.len()), "Save").description(
                "Fill in only the fields to change. Empty fields keep each wish's value.",
            ),
        };
        let quantity = Field::number(
            "quantity",
            "Quantity",
            single.as_ref().map(|wish| wish.quantity),
        )
        .min(1)
        .half();
        let spec = spec
            .field(if single.is_some() {
                quantity
            } else {
                quantity.optional()
            })
            .field(
                Field::platinum(
                    "max_price",
                    "Maximum price",
                    single.as_ref().and_then(|wish| wish.properties.max_price),
                )
                .optional()
                .half()
                .hint("The live scraper never bids above this."),
            );
        Some(match &single {
            Some(wish) => spec.field(
                Field::toggle("is_bulk", "Trade in batches", wish.properties.is_bulk)
                    .hint("Asks for up to 6 units in one trade."),
            ),
            None => spec,
        })
    }

    fn message_items(&self, ids: &[u64]) -> Vec<MessageItem> {
        ids.iter()
            .filter_map(|id| self.wish(*id))
            .filter(|wish| wish.list_price.is_some())
            .map(|wish| MessageItem {
                rank: wish.rank_label(),
                price: wish.list_price,
                name: wish.item_name,
            })
            .collect()
    }
}

impl Catalog for WishList {
    type Row = WishListItem;
    type Action = WishAction;

    const NOUN: &'static str = "wishes";
    const SEARCH_PLACEHOLDER: &'static str = "Search the wish list";
    const EMPTY_HINT: &'static str =
        "Add an item you want and the live scraper will keep a buy order up for it.";

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("item", "Item", 200.).flex().sortable(),
            ColumnSpec::new("rank", "Rank", 104.),
            ColumnSpec::new("quantity", "Quantity", 88.).end().sortable(),
            ColumnSpec::new("max_price", "Max price", 136.).sortable().droppable(1),
            ColumnSpec::new("list_price", "Bid", 104.).end().sortable(),
            ColumnSpec::new("status", "Status", 156.).sortable(),
            ColumnSpec::new("actions", "", 184.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<WishListItem> {
        self.store
            .list(&query.text, status_filter(&self.store, query))
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        vec![status_chips(&self.store, cx)]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = self.store.rows();
        let units: i64 = rows.iter().map(|wish| wish.quantity).sum();
        let bids: i64 = rows
            .iter()
            .map(|wish| wish.list_price.unwrap_or(0) * wish.quantity)
            .sum();
        let ceiling: i64 = rows
            .iter()
            .map(|wish| wish.properties.max_price.unwrap_or(0) * wish.quantity)
            .sum();
        let live = rows
            .iter()
            .filter(|wish| wish.status == StockStatus::Live)
            .count();
        vec![
            Stat {
                label: "Bids",
                value: number(bids),
                note: "platinum in open buy orders".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Budget",
                value: number(ceiling),
                note: "if every wish fills at its maximum".to_owned(),
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
            WishAction::Edit,
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

    fn perform(&mut self, action: WishAction) -> Outcome<WishAction> {
        match action {
            WishAction::Add => Outcome::ask(
                FormSpec::new("Add wish", "Add wish")
                    .field(Field::choice("name", "Item", preview::tradable_items(), None))
                    .field(
                        Field::number("rank", "Rank", None)
                            .optional()
                            .hint("For mods and arcanes."),
                    )
                    .field(Field::number("quantity", "Quantity", Some(1)).min(1).half())
                    .field(
                        Field::platinum("max_price", "Maximum price", None)
                            .optional()
                            .half()
                            .hint("Per unit."),
                    ),
                |values| {
                    Some(WishAction::Create {
                        name: values.choice("name")?,
                        rank: values.number("rank"),
                        quantity: values.number("quantity")?,
                        max_price: values.number("max_price"),
                    })
                },
            ),
            WishAction::Create {
                name,
                rank,
                quantity,
                max_price,
            } => {
                self.store.insert(WishListItem {
                    id: 0,
                    wfm_url: name.to_lowercase().replace(' ', "_"),
                    item_name: name.clone(),
                    sub_type: rank.map(SubType::rank),
                    quantity,
                    list_price: None,
                    is_hidden: false,
                    status: StockStatus::Pending,
                    price_history: Vec::new(),
                    updated_at: String::new(),
                    properties: WishListProperties {
                        max_price,
                        is_bulk: false,
                    },
                });
                Outcome::Notice(format!("Added {name} to the wish list."))
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
            WishAction::BuyAtList(id) => match self.wish(id).and_then(|wish| wish.list_price) {
                Some(price) => self.perform(WishAction::Bought {
                    id,
                    quantity: 1,
                    total: price,
                }),
                None => Outcome::Done,
            },
            WishAction::Bought {
                id,
                quantity,
                total,
            } => match self.wish(id) {
                Some(wish) => {
                    let quantity = quantity.min(wish.quantity);
                    self.store.bought(id as i64, quantity);
                    Outcome::Notice(format!(
                        "Bought {} {} for {} platinum.",
                        number(quantity),
                        wish.item_name,
                        number(total)
                    ))
                }
                None => Outcome::Done,
            },
            WishAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => Outcome::ask(spec, move |values| {
                    Some(WishAction::Save {
                        ids: ids.clone(),
                        edit: WishEdit {
                            quantity: values.number("quantity"),
                            max_price: values.number("max_price"),
                            is_bulk: values.flag("is_bulk"),
                        },
                    })
                }),
                None => Outcome::Done,
            },
            WishAction::Save { ids, edit } => {
                let single = ids.len() == 1;
                for id in &ids {
                    self.store.update(*id as i64, |wish| {
                        wish.quantity = edit.quantity.unwrap_or(wish.quantity);
                        if single {
                            wish.properties.max_price = edit.max_price;
                            wish.properties.is_bulk = edit.is_bulk;
                        } else {
                            wish.properties.max_price =
                                edit.max_price.or(wish.properties.max_price);
                        }
                    });
                }
                Outcome::Done
            }
            WishAction::EditMaxPrice(id) => match self.wish(id) {
                Some(wish) => Outcome::ask(
                    prompts::price_limit(
                        &format!("Maximum price for {}", wish.item_name),
                        "Maximum price",
                        "The live scraper never bids above this. Leave it empty for no maximum.",
                        wish.properties.max_price,
                    ),
                    move |values| {
                        Some(WishAction::SetMaxPrice {
                            id,
                            price: values.number("price"),
                        })
                    },
                ),
                None => Outcome::Done,
            },
            WishAction::SetMaxPrice { id, price } => {
                self.store.update(id as i64, |wish| {
                    wish.properties.max_price = price.filter(|price| *price > 0);
                });
                Outcome::Done
            }
            WishAction::StepMaxPrice { id, step } => {
                self.store.update(id as i64, |wish| {
                    let next = wish.properties.max_price.unwrap_or(0) + step;
                    wish.properties.max_price = (next > 0).then_some(next);
                });
                Outcome::Done
            }
            WishAction::ToggleHidden(id) => {
                self.store
                    .update(id as i64, |wish| wish.is_hidden = !wish.is_hidden);
                Outcome::Done
            }
            WishAction::Delete(ids) if ids.is_empty() => Outcome::Done,
            WishAction::Delete(ids) => Outcome::ask(
                prompts::delete(ids.len(), "wish", "wishes"),
                move |_| Some(WishAction::Deleted(ids.clone())),
            ),
            WishAction::Deleted(ids) => {
                let ids: Vec<i64> = ids.iter().map(|id| *id as i64).collect();
                self.store.delete(&ids);
                Outcome::Done
            }
            WishAction::Details(id) => Outcome::ShowPanel(id),
            WishAction::CopyName(id) => match self.wish(id) {
                Some(wish) => Outcome::Copy {
                    what: wish.item_name.clone(),
                    text: wish.item_name,
                },
                None => Outcome::Done,
            },
            WishAction::OpenMarket(id) => match self.wish(id) {
                Some(wish) => Outcome::OpenUrl(market_url("items", &wish.wfm_url)),
                None => Outcome::Done,
            },
            WishAction::Message(ids) => {
                let items = self.message_items(&ids);
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
            WishAction::Export => Outcome::Notice(EXPORT_NOT_CONNECTED.to_owned()),
        }
    }

    fn row_id(row: &WishListItem) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &WishListItem, b: &WishListItem) -> Ordering {
        match key {
            "quantity" => a.quantity.cmp(&b.quantity),
            "max_price" => a.properties.max_price.cmp(&b.properties.max_price),
            "list_price" => a.list_price.cmp(&b.list_price),
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
            "rank" => cells::text(row.rank_label().unwrap_or_default()),
            "quantity" => cells::amount(Some(row.quantity), "", cx),
            "max_price" => cells::stepped_price(
                id,
                row.properties.max_price,
                MIN_PRICE_STEP,
                WishAction::StepMaxPrice {
                    id,
                    step: -MIN_PRICE_STEP,
                },
                WishAction::StepMaxPrice {
                    id,
                    step: MIN_PRICE_STEP,
                },
                emit,
                cx,
            ),
            "list_price" => cells::amount(row.list_price, "No bid", cx),
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
                            ("buy-list", id),
                            IconName::ShoppingCart,
                            "Bought one at the bid price",
                        )
                        .in_row()
                        .disabled(row.list_price.is_none())
                        .on_click(emit.on_click(WishAction::BuyAtList(id))),
                    )
                    .child(
                        IconButton::new(("edit", id), IconName::SquarePen, "Edit")
                            .in_row()
                            .on_click(emit.on_click(WishAction::Edit(vec![id]))),
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

    fn cell_text(row: &WishListItem, key: &str) -> Option<String> {
        match key {
            "item" => Some(row.item_name.clone()),
            "rank" => row.rank_label(),
            "quantity" => Some(number(row.quantity)),
            "max_price" => row.properties.max_price.map(number),
            "list_price" => row.list_price.map(number),
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
                WishAction::BuyAtList(id),
            ));
        }
        menu.extend([
            MenuEntry::new("Edit", IconName::SquarePen, WishAction::Edit(vec![id])),
            MenuEntry::new(
                "Set maximum price",
                IconName::Tag,
                WishAction::EditMaxPrice(id),
            ),
            MenuEntry::new("Show details", IconName::PanelRight, WishAction::Details(id)),
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
        let room = match (row.properties.max_price, row.list_price) {
            (Some(max), Some(bid)) => Some(max - bid),
            _ => None,
        };
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
                Fact::new("Quantity", number(row.quantity)),
                Fact::amount("Maximum price", row.properties.max_price, "Not set"),
                Fact::amount("Bid", row.list_price, "No bid"),
                match room {
                    Some(room) => {
                        Fact::new("Room under the maximum", number(room)).tone(Tone::of_profit(room))
                    }
                    None => Fact::new("Room under the maximum", "None to show").tone(Tone::Muted),
                },
                Fact::amount(
                    "Cost at the bid",
                    row.list_price.map(|price| price * row.quantity),
                    "No bid",
                ),
                Fact::new("Last checked", day_and_time(&row.updated_at)),
            ],
            primary: vec![
                PanelAction::new("Bought", IconName::HandCoins, WishAction::Buy(id)),
                PanelAction::new(
                    "Bought one at the bid price",
                    IconName::ShoppingCart,
                    WishAction::BuyAtList(id),
                )
                .enabled(row.list_price.is_some()),
            ],
            secondary: vec![
                PanelAction::new("Edit", IconName::SquarePen, WishAction::Edit(vec![id])),
                PanelAction::new(
                    "Set maximum price",
                    IconName::Tag,
                    WishAction::EditMaxPrice(id),
                ),
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
