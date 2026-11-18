//! The Trade messages screen: priced lists of items, custom entries and
//! rivens that become trade chat messages.

pub mod model;

use std::cmp::Ordering;

use gpui::{AnyElement, App, AppContext, Entity, IntoElement, ParentElement, Styled, Window, div, px};
use gpui_kit_assets::IconName;

use crate::{
    model::SubType,
    preview,
    theme::Theme,
    ui::{
        IconButton, cells, day_and_time,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ChipSpec, ColumnSpec, Emit, MenuEntry, Outcome, PrimaryAction,
            Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number,
        page::{ListPage, PreviewState},
        panel::{Fact, PanelAction, PanelModel},
        prompts::{self, MessageItem},
        row_button,
        screen::TabbedScreen,
    },
};
use model::{RivenSearch, TradeEntry, TradeGroup, TradeStore};

/// The step of the buttons beside a price, in platinum.
const PRICE_STEP: i64 = 5;

pub fn screen(state: PreviewState, window: &mut Window, cx: &mut App) -> Entity<TabbedScreen> {
    let load = state.load_state();
    let entries = state.rows(preview::trade_entries());
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("trade-tab");
        for group in TradeGroup::ALL {
            let rows = entries
                .iter()
                .filter(|entry| entry.group == group)
                .cloned()
                .collect();
            let page = cx.new(|cx| {
                ListPage::new(
                    TradeEntries {
                        store: TradeStore::new(rows, preview::riven_candidates()),
                        group,
                    },
                    load.clone(),
                    window,
                    cx,
                )
            });
            screen.add(group.as_str(), group.label(), page, cx);
        }
        screen
    })
}

#[derive(Clone, Debug)]
pub enum TradeAction {
    Add,
    Create {
        name: String,
        rank: Option<i64>,
        price: i64,
        tags: Vec<String>,
    },
    Message(Vec<u64>),
    CopyMessage(String),
    FindRivens,
    FoundRivens(RivenSearch),
    Edit(Vec<u64>),
    Save {
        ids: Vec<u64>,
        price: Option<i64>,
        tags: Option<Vec<String>>,
    },
    StepPrice {
        id: u64,
        step: i64,
    },
    Delete(Vec<u64>),
    Deleted(Vec<u64>),
    Details(u64),
    CopyName(u64),
    Export,
}

pub struct TradeEntries {
    store: TradeStore,
    group: TradeGroup,
}

impl TradeEntries {
    fn entry(&self, id: u64) -> Option<TradeEntry> {
        self.store.get(id as i64).cloned()
    }

    fn all(&self) -> Vec<TradeEntry> {
        self.store.list(self.group, "", None)
    }

    /// Rivens are bought, items and custom entries are sold.
    const fn prefix(&self) -> &'static str {
        match self.group {
            TradeGroup::Riven => "WTB",
            TradeGroup::Item | TradeGroup::Custom => "WTS",
        }
    }

    const fn singular(&self) -> &'static str {
        match self.group {
            TradeGroup::Item => "item",
            TradeGroup::Custom => "entry",
            TradeGroup::Riven => "riven",
        }
    }

    fn add_prompt(&self) -> FormSpec {
        let spec = FormSpec::new(format!("Add {}", self.singular()), "Add");
        let spec = match self.group {
            TradeGroup::Item => spec
                .field(Field::choice("name", "Item", preview::tradable_items(), None))
                .field(
                    Field::number("rank", "Rank", None)
                        .optional()
                        .half()
                        .hint("For mods and arcanes."),
                ),
            TradeGroup::Custom => {
                spec.field(Field::text("name", "Name", "", "Maxed Serration"))
            }
            TradeGroup::Riven => spec.field(Field::choice(
                "name",
                "Weapon",
                preview::riven_weapons(),
                None,
            )),
        };
        spec.field(Field::platinum("price", "Price", Some(1)).min(1).half())
            .field(
                Field::text("tags", "Tags", "", "arcanes, prime sets")
                    .optional()
                    .hint("Separate tags with commas."),
            )
    }

    fn edit_prompt(&self, ids: &[u64]) -> Option<FormSpec> {
        let single = match ids {
            [id] => Some(self.entry(*id)?),
            [] => return None,
            _ => None,
        };
        let price = Field::platinum("price", "Price", single.as_ref().map(|entry| entry.price))
            .min(1);
        let tags = Field::text(
            "tags",
            "Tags",
            &single
                .as_ref()
                .map(|entry| entry.tags.join(", "))
                .unwrap_or_default(),
            "arcanes, prime sets",
        )
        .optional()
        .hint("Separate tags with commas.");
        Some(match &single {
            Some(entry) => FormSpec::new(format!("Edit {}", entry.name), "Save")
                .field(price)
                .field(tags),
            None => FormSpec::new(format!("Edit {} entries", ids.len()), "Save")
                .description(
                    "Fill in only the fields to change. Empty fields keep each entry's value.",
                )
                .field(price.optional())
                .field(tags),
        })
    }

    fn message_items(&self, ids: &[u64]) -> Vec<MessageItem> {
        let entries = if ids.is_empty() {
            self.all()
        } else {
            ids.iter().filter_map(|id| self.entry(*id)).collect()
        };
        entries
            .into_iter()
            .map(|entry| MessageItem {
                rank: entry.rank_label(),
                price: Some(entry.price),
                name: entry.name,
            })
            .collect()
    }
}

impl Catalog for TradeEntries {
    type Row = TradeEntry;
    type Action = TradeAction;

    const NOUN: &'static str = "trade entries";
    const SEARCH_PLACEHOLDER: &'static str = "Search entries";
    const EMPTY_HINT: &'static str =
        "Add what you trade with a price, then write a trade chat message from the list.";

    fn columns(&self) -> Vec<ColumnSpec> {
        let mut columns = vec![ColumnSpec::new("name", "Name", 200.).flex().sortable()];
        if self.group == TradeGroup::Item {
            columns.push(ColumnSpec::new("rank", "Rank", 104.));
        }
        columns.push(ColumnSpec::new("price", "Price", 136.).sortable());
        if self.group != TradeGroup::Custom {
            columns.push(ColumnSpec::new("min_price", "Market low", 104.).end().sortable().droppable(1));
            columns.push(ColumnSpec::new("profit", "Potential profit", 124.).end().sortable());
        }
        columns.extend([
            ColumnSpec::new("tags", "Tags", 190.).droppable(2),
            ColumnSpec::new("updated", "Updated", 128.).sortable().droppable(3),
            ColumnSpec::new("actions", "", 176.),
        ]);
        columns
    }

    fn rows(&self, query: &Query) -> Vec<TradeEntry> {
        self.store
            .list(self.group, &query.text, query.filter("tag"))
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        vec![ChipGroup {
            id: "tag",
            chips: self
                .store
                .tag_counts(self.group)
                .into_iter()
                .map(|(tag, count)| ChipSpec {
                    key: tag.clone(),
                    label: tag,
                    count,
                    color: None,
                    tooltip: String::new(),
                })
                .collect(),
        }]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = self.all();
        let asking: i64 = rows.iter().map(|entry| entry.price).sum();
        let profit: i64 = rows
            .iter()
            .filter_map(|entry| entry.properties.potential_profit)
            .sum();
        let mut stats = vec![
            Stat {
                label: "Entries",
                value: number(rows.len() as i64),
                note: "in this list".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: match self.group {
                    TradeGroup::Riven => "Offering",
                    TradeGroup::Item | TradeGroup::Custom => "Asking",
                },
                value: number(asking),
                note: "platinum for one of each".to_owned(),
                tone: Tone::Plain,
            },
        ];
        if self.group != TradeGroup::Custom {
            stats.push(Stat {
                label: "Potential profit",
                value: number(profit),
                note: "against the market lows".to_owned(),
                tone: Tone::of_profit(profit),
            });
        }
        stats
    }

    fn toolbar(&self, checked: &[u64]) -> Vec<ToolbarItem<TradeAction>> {
        let selection = (!checked.is_empty()).then(|| checked.to_vec());
        let mut toolbar = vec![ToolbarItem {
            id: "message",
            icon: IconName::MessageSquare,
            tooltip: if checked.is_empty() {
                "Write a trade chat message for every entry".to_owned()
            } else {
                "Write a trade chat message for the selected entries".to_owned()
            },
            action: Some(TradeAction::Message(checked.to_vec())),
            danger: false,
        }];
        if self.group == TradeGroup::Riven {
            toolbar.push(ToolbarItem {
                id: "find-rivens",
                icon: IconName::ScanSearch,
                tooltip: "Find rivens worth buying from the last hour of prices".to_owned(),
                action: Some(TradeAction::FindRivens),
                danger: false,
            });
        }
        toolbar.extend([
            ToolbarItem {
                id: "export",
                icon: IconName::Download,
                tooltip: "Export the list as JSON".to_owned(),
                action: Some(TradeAction::Export),
                danger: false,
            },
            ToolbarItem {
                id: "edit-selected",
                icon: IconName::SquarePen,
                tooltip: "Edit the selected entries".to_owned(),
                action: selection.clone().map(TradeAction::Edit),
                danger: false,
            },
            ToolbarItem {
                id: "delete-selected",
                icon: IconName::Trash,
                tooltip: "Delete the selected entries".to_owned(),
                action: selection.map(TradeAction::Delete),
                danger: true,
            },
        ]);
        toolbar
    }

    fn primary(&self) -> Option<PrimaryAction<TradeAction>> {
        Some(PrimaryAction {
            label: match self.group {
                TradeGroup::Item => "Add item",
                TradeGroup::Custom => "Add entry",
                TradeGroup::Riven => "Add riven",
            },
            icon: IconName::Plus,
            action: TradeAction::Add,
        })
    }

    fn perform(&mut self, action: TradeAction) -> Outcome<TradeAction> {
        match action {
            TradeAction::Add => {
                let named = self.group == TradeGroup::Custom;
                Outcome::ask(self.add_prompt(), move |values| {
                    Some(TradeAction::Create {
                        name: if named {
                            values.text("name")
                        } else {
                            values.choice("name")?
                        },
                        rank: values.number("rank"),
                        price: values.number("price")?,
                        tags: values.list("tags"),
                    })
                })
            }
            TradeAction::Create {
                name,
                rank,
                price,
                tags,
            } => {
                self.store
                    .create(self.group, name, rank.map(SubType::rank), price, tags);
                Outcome::Done
            }
            TradeAction::Message(ids) => {
                let items = self.message_items(&ids);
                if items.is_empty() {
                    return Outcome::Notice("There are no entries to write about.".to_owned());
                }
                Outcome::ask(
                    prompts::message(self.prefix(), items.clone()),
                    move |values| {
                        Some(TradeAction::CopyMessage(prompts::compose_message(
                            values, &items,
                        )))
                    },
                )
            }
            TradeAction::CopyMessage(text) => Outcome::Copy {
                text,
                what: "the trade chat message".to_owned(),
            },
            TradeAction::FindRivens => Outcome::ask(
                FormSpec::new("Find rivens", "Add to the list")
                    .description(
                        "Looks through the last hour of riven prices and adds every weapon that passes these limits, priced below its market low.",
                    )
                    .width(480.)
                    .field(Field::platinum("min_price", "Market low at least", Some(50)).half())
                    .field(
                        Field::number("min_volume", "Trades at least", Some(10))
                            .unit("an hour")
                            .half(),
                    )
                    .field(
                        Field::number("discount", "Discount", Some(25))
                            .unit("percent")
                            .half(),
                    )
                    .field(
                        Field::platinum("round_to", "Round down to", Some(5))
                            .min(1)
                            .half(),
                    )
                    .field(
                        Field::text("tags", "Tags", "found", "found")
                            .optional()
                            .hint("Separate tags with commas."),
                    )
                    .field(Field::toggle(
                        "override_existing",
                        "Reprice rivens already in the list",
                        false,
                    )),
                |values| {
                    Some(TradeAction::FoundRivens(RivenSearch {
                        min_price: values.number("min_price")?,
                        min_volume: values.number("min_volume")?,
                        discount_percent: values.number("discount")?.min(99),
                        round_to: values.number("round_to")?,
                        override_existing: values.flag("override_existing"),
                        tags: values.list("tags"),
                    }))
                },
            ),
            TradeAction::FoundRivens(search) => {
                let count = self.store.find_rivens(&search);
                Outcome::Notice(match count {
                    0 => "No riven passed the limits.".to_owned(),
                    1 => "Added or repriced 1 riven.".to_owned(),
                    count => format!("Added or repriced {count} rivens."),
                })
            }
            TradeAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => {
                    let single = ids.len() == 1;
                    Outcome::ask(spec, move |values| {
                        let tags = values.list("tags");
                        Some(TradeAction::Save {
                            ids: ids.clone(),
                            price: values.number("price"),
                            tags: (single || !tags.is_empty()).then_some(tags),
                        })
                    })
                }
                None => Outcome::Done,
            },
            TradeAction::Save { ids, price, tags } => {
                for id in &ids {
                    self.store.update(*id as i64, |entry| {
                        entry.price = price.unwrap_or(entry.price);
                        if let Some(tags) = &tags {
                            entry.tags.clone_from(tags);
                        }
                    });
                }
                Outcome::Done
            }
            TradeAction::StepPrice { id, step } => {
                self.store
                    .update(id as i64, |entry| entry.price = (entry.price + step).max(1));
                Outcome::Done
            }
            TradeAction::Delete(ids) if ids.is_empty() => Outcome::Done,
            TradeAction::Delete(ids) => Outcome::ask(
                prompts::delete(ids.len(), "trade entry", "trade entries"),
                move |_| Some(TradeAction::Deleted(ids.clone())),
            ),
            TradeAction::Deleted(ids) => {
                let ids: Vec<i64> = ids.iter().map(|id| *id as i64).collect();
                self.store.delete(&ids);
                Outcome::Done
            }
            TradeAction::Details(id) => Outcome::ShowPanel(id),
            TradeAction::CopyName(id) => match self.entry(id) {
                Some(entry) => Outcome::Copy {
                    what: entry.name.clone(),
                    text: entry.name,
                },
                None => Outcome::Done,
            },
            TradeAction::Export => {
                Outcome::Notice("Export isn't connected to the backend yet.".to_owned())
            }
        }
    }

    fn row_id(row: &TradeEntry) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &TradeEntry, b: &TradeEntry) -> Ordering {
        match key {
            "price" => a.price.cmp(&b.price),
            "min_price" => a.properties.min_price.cmp(&b.properties.min_price),
            "profit" => a
                .properties
                .potential_profit
                .cmp(&b.properties.potential_profit),
            "updated" => a.updated_at.cmp(&b.updated_at),
            _ => a.name.cmp(&b.name),
        }
    }

    fn cell(row: &TradeEntry, key: &str, emit: &Emit<TradeAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let id = row.id as u64;
        match key {
            "name" => cells::name(row.name.clone(), &[], cx),
            "rank" => cells::text(row.rank_label().unwrap_or_default()),
            "price" => cells::stepped_price(
                id,
                Some(row.price),
                PRICE_STEP,
                TradeAction::StepPrice {
                    id,
                    step: -PRICE_STEP,
                },
                TradeAction::StepPrice {
                    id,
                    step: PRICE_STEP,
                },
                emit,
                cx,
            ),
            "min_price" => cells::amount(row.properties.min_price, "Unknown", cx),
            "profit" => {
                let profit = row.properties.potential_profit;
                cells::toned_amount(profit, "Unknown", Tone::of_profit(profit.unwrap_or(0)), cx)
            }
            "tags" => cells::tags(&row.tags, cx),
            "updated" => cells::text(if row.updated_at.is_empty() {
                "Just now".to_owned()
            } else {
                day_and_time(&row.updated_at)
            }),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(
                        div().pr(px(5.)).child(
                            row_button(("message", id), "Message", cx)
                                .on_click(emit.on_click(TradeAction::Message(vec![id]))),
                        ),
                    )
                    .child(
                        IconButton::new(("edit", id), IconName::SquarePen, "Edit")
                            .in_row()
                            .on_click(emit.on_click(TradeAction::Edit(vec![id]))),
                    )
                    .child(
                        IconButton::new(("delete", id), IconName::Trash, "Delete")
                            .in_row()
                            .hover_tone(theme.colors.negative)
                            .on_click(emit.on_click(TradeAction::Delete(vec![id]))),
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

    fn cell_text(row: &TradeEntry, key: &str) -> Option<String> {
        match key {
            "name" => Some(row.name.clone()),
            "rank" => row.rank_label(),
            "price" => Some(number(row.price)),
            "min_price" => row.properties.min_price.map(number),
            "profit" => row.properties.potential_profit.map(number),
            "tags" => Some(row.tags.join(", ")),
            "updated" => Some(day_and_time(&row.updated_at)),
            _ => None,
        }
    }

    fn row_menu(row: &TradeEntry) -> Vec<MenuEntry<TradeAction>> {
        let id = row.id as u64;
        vec![
            MenuEntry::new(
                "Write a trade chat message",
                IconName::MessageSquare,
                TradeAction::Message(vec![id]),
            ),
            MenuEntry::new("Edit", IconName::SquarePen, TradeAction::Edit(vec![id])),
            MenuEntry::new("Show details", IconName::PanelRight, TradeAction::Details(id)),
            MenuEntry::new("Copy name", IconName::Copy, TradeAction::CopyName(id)),
            MenuEntry::new("Delete", IconName::Trash, TradeAction::Delete(vec![id])).separated(),
        ]
    }

    fn panel(row: &TradeEntry, _: &App) -> PanelModel<TradeAction> {
        let id = row.id as u64;
        let profit = row.properties.potential_profit;
        PanelModel {
            title: row.name.clone(),
            subtitle: match row.rank_label() {
                Some(rank) => format!("{rank}, {}", row.group.label().to_lowercase()),
                None => row.group.label().to_owned(),
            },
            status: None,
            note: None,
            history: Vec::new(),
            lines: None,
            facts: vec![
                Fact::new("Price", number(row.price)),
                Fact::amount("Market low", row.properties.min_price, "Unknown"),
                match profit {
                    Some(profit) => {
                        Fact::new("Potential profit", number(profit)).tone(Tone::of_profit(profit))
                    }
                    None => Fact::new("Potential profit", "Unknown").tone(Tone::Muted),
                },
                if row.tags.is_empty() {
                    Fact::new("Tags", "None").tone(Tone::Muted)
                } else {
                    Fact::new("Tags", row.tags.join(", "))
                },
                Fact::new(
                    "Updated",
                    if row.updated_at.is_empty() {
                        "Just now".to_owned()
                    } else {
                        day_and_time(&row.updated_at)
                    },
                ),
            ],
            primary: vec![PanelAction::new(
                "Write a trade chat message",
                IconName::MessageSquare,
                TradeAction::Message(vec![id]),
            )],
            secondary: vec![
                PanelAction::new("Edit", IconName::SquarePen, TradeAction::Edit(vec![id])),
                PanelAction::new("Copy name", IconName::Copy, TradeAction::CopyName(id)),
                PanelAction::new("Delete", IconName::Trash, TradeAction::Delete(vec![id])).danger(),
            ],
        }
    }
}
