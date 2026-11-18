//! The Trade messages screen: priced lists of items, custom entries and
//! rivens that become trade chat messages.

use std::{
    cmp::Ordering,
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use gpui::{AnyElement, App, AppContext, Entity, IntoElement, ParentElement, Styled, Window, div, px};
use gpui_kit_assets::IconName;

use crate::{
    data::{Backend, DataKind, DataSource},
    model::{
        ListParams, NewTradeEntry, PriceLookupParams, SubType, TradeEntry, TradeEntryGroup,
        TradeEntryListParams, TradeEntryUpdate,
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
        page::ListPage,
        panel::{Fact, PanelAction, PanelModel},
        prompts::{self, MessageItem},
        row_button,
        screen::TabbedScreen,
    },
};

/// The step of the buttons beside a price, in platinum.
const PRICE_STEP: i64 = 5;

const GROUPS: [(TradeEntryGroup, &str); 3] = [
    (TradeEntryGroup::Item, "Items"),
    (TradeEntryGroup::Custom, "Custom"),
    (TradeEntryGroup::Riven, "Rivens"),
];

pub fn screen(backend: &Entity<Backend>, window: &mut Window, cx: &mut App) -> Entity<TabbedScreen> {
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("trade-tab");
        for (group, label) in GROUPS {
            let page = cx.new(|cx| {
                ListPage::new(
                    TradeEntries {
                        rows: Vec::new(),
                        group,
                    },
                    backend,
                    window,
                    cx,
                )
            });
            screen.add(group.as_str(), label, page, cx);
        }
        screen
    })
}

/// The settings of "Find rivens".
#[derive(Clone, Debug)]
pub struct RivenSearch {
    min_price: i64,
    min_volume: i64,
    discount_percent: i64,
    round_to: i64,
    override_existing: bool,
    tags: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum TradeAction {
    Add,
    Create(NewTradeEntry),
    Message(Vec<u64>),
    CopyMessage(String),
    FindRivens,
    FoundRivens(RivenSearch),
    Edit(Vec<u64>),
    Save { ids: Vec<u64>, update: TradeEntryUpdate },
    StepPrice { id: u64, step: i64 },
    Delete(Vec<u64>),
    Deleted(Vec<u64>),
    Details(u64),
    CopyName(u64),
    Export,
}

pub struct TradeEntries {
    rows: Vec<TradeEntry>,
    group: TradeEntryGroup,
}

/// Today's UTC date as `YYYY-MM-DD`, from the days since the Unix epoch.
fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn rank_label(entry: &TradeEntry) -> Option<String> {
    entry.sub_type.as_ref().and_then(SubType::label)
}

fn updated(entry: &TradeEntry) -> String {
    day_and_time(&entry.updated_at)
}

impl TradeEntries {
    fn entry(&self, id: u64) -> Option<TradeEntry> {
        self.rows.iter().find(|entry| entry.id as u64 == id).cloned()
    }

    fn params(&self) -> TradeEntryListParams {
        TradeEntryListParams {
            list: ListParams {
                limit: -1,
                ..ListParams::default()
            },
            group: Some(self.group.as_str().to_owned()),
            tags: Vec::new(),
        }
    }

    /// Rivens are bought, items and custom entries are sold.
    const fn prefix(&self) -> &'static str {
        match self.group {
            TradeEntryGroup::Riven => "WTB",
            TradeEntryGroup::Item | TradeEntryGroup::Custom => "WTS",
        }
    }

    const fn singular(&self) -> &'static str {
        match self.group {
            TradeEntryGroup::Item => "item",
            TradeEntryGroup::Custom => "entry",
            TradeEntryGroup::Riven => "riven",
        }
    }

    fn tag_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for tag in self.rows.iter().flat_map(|entry| &entry.tags) {
            *counts.entry(tag.clone()).or_default() += 1;
        }
        counts
    }

    fn add_prompt(&self) -> FormSpec {
        let spec = FormSpec::new(format!("Add {}", self.singular()), "Add");
        let spec = match self.group {
            TradeEntryGroup::Item => spec
                .field(
                    Field::text("name", "Item", "", "Arcane Energize")
                        .hint("The item's name on Warframe Market."),
                )
                .field(
                    Field::number("rank", "Rank", None)
                        .optional()
                        .half()
                        .hint("For mods and arcanes."),
                ),
            TradeEntryGroup::Custom => {
                spec.field(Field::text("name", "Name", "", "Maxed Serration"))
            }
            TradeEntryGroup::Riven => spec.field(
                Field::text("name", "Weapon", "", "Rubico")
                    .hint("The weapon's name on Warframe Market."),
            ),
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
        let price =
            Field::platinum("price", "Price", single.as_ref().map(|entry| entry.price)).min(1);
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
            self.rows.clone()
        } else {
            ids.iter().filter_map(|id| self.entry(*id)).collect()
        };
        entries
            .into_iter()
            .map(|entry| MessageItem {
                rank: rank_label(&entry),
                price: Some(entry.price),
                name: entry.name,
            })
            .collect()
    }

    /// Looks up today's riven prices and adds a discounted entry for every
    /// weapon that passes the limits.
    fn find_rivens(&self, search: RivenSearch, source: &DataSource) -> Outcome<TradeAction> {
        let source = source.clone();
        let existing = self.rows.clone();
        Outcome::Run(Box::pin(async move {
            let mut params = PriceLookupParams::new(today(), today());
            params.limit = -1;
            let prices = source.riven_prices_lookup(&params).await?.results;
            let round = search.round_to.max(1);
            let mut changed = 0;
            for riven in prices {
                if riven.min_price < search.min_price as f64
                    || riven.volume < search.min_volume as f64
                {
                    continue;
                }
                let discounted =
                    (riven.min_price * (100 - search.discount_percent) as f64 / 100.0) as i64;
                let price = (discounted / round * round).max(1);
                match existing.iter().find(|entry| entry.name == riven.name) {
                    Some(entry) if search.override_existing => {
                        let update = TradeEntryUpdate {
                            price: Some(price),
                            tags: None,
                        };
                        source.trade_entry_update(entry.id, update).await?;
                    }
                    Some(_) => continue,
                    None => {
                        let entry = NewTradeEntry {
                            raw: riven.wfm_url,
                            sub_type: None,
                            price,
                            group: TradeEntryGroup::Riven,
                            tags: search.tags.clone(),
                        };
                        source.trade_entry_create(entry).await?;
                    }
                }
                changed += 1;
            }
            Ok(match changed {
                0 => "No riven passed the limits.".to_owned(),
                1 => "Added or repriced 1 riven.".to_owned(),
                changed => format!("Added or repriced {changed} rivens."),
            })
        }))
    }
}

impl Catalog for TradeEntries {
    type Row = TradeEntry;
    type Action = TradeAction;
    type Data = Vec<TradeEntry>;

    const NOUN: &'static str = "trade entries";
    const SEARCH_PLACEHOLDER: &'static str = "Search entries";
    const EMPTY_HINT: &'static str =
        "Add what you trade with a price, then write a trade chat message from the list.";
    /// The backend sends no event for trade entries. The page loads the list
    /// again after each change it makes.
    const WATCH: &'static [DataKind] = &[];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let rows = source.get_trade_entry_pagination(&self.params());
        Box::pin(async move { Ok(rows.await?.results) })
    }

    fn apply(&mut self, data: Self::Data) {
        self.rows = data;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        let mut columns = vec![ColumnSpec::new("name", "Name", 220.).flex().sortable()];
        if self.group == TradeEntryGroup::Item {
            columns.push(ColumnSpec::new("rank", "Rank", 120.));
        }
        columns.extend([
            ColumnSpec::new("price", "Price", 148.).sortable(),
            ColumnSpec::new("tags", "Tags", 280.).droppable(1),
            ColumnSpec::new("updated", "Updated", 148.)
                .sortable()
                .droppable(2),
            ColumnSpec::new("actions", "", 184.),
        ]);
        columns
    }

    fn rows(&self, query: &Query) -> Vec<TradeEntry> {
        let tag = query.filter("tag");
        self.rows
            .iter()
            .filter(|entry| tag.is_none_or(|tag| entry.tags.iter().any(|own| own == tag)))
            .filter(|entry| query.matches(&entry.name))
            .cloned()
            .collect()
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        vec![ChipGroup {
            id: "tag",
            chips: self
                .tag_counts()
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
        let asking: i64 = self.rows.iter().map(|entry| entry.price).sum();
        let highest = self.rows.iter().max_by_key(|entry| entry.price);
        vec![
            Stat {
                label: "Entries",
                value: number(self.rows.len() as i64),
                note: format!("{} tags", self.tag_counts().len()),
                tone: Tone::Plain,
            },
            Stat {
                label: match self.group {
                    TradeEntryGroup::Riven => "Offering",
                    TradeEntryGroup::Item | TradeEntryGroup::Custom => "Asking",
                },
                value: number(asking),
                note: "platinum for one of each".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Highest price",
                value: highest.map_or("None".to_owned(), |entry| number(entry.price)),
                note: highest.map_or(String::new(), |entry| entry.name.clone()),
                tone: Tone::Plain,
            },
        ]
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
        if self.group == TradeEntryGroup::Riven {
            toolbar.push(ToolbarItem {
                id: "find-rivens",
                icon: IconName::ScanSearch,
                tooltip: "Find rivens worth buying from today's prices".to_owned(),
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
                TradeEntryGroup::Item => "Add item",
                TradeEntryGroup::Custom => "Add entry",
                TradeEntryGroup::Riven => "Add riven",
            },
            icon: IconName::Plus,
            action: TradeAction::Add,
        })
    }

    fn perform(&mut self, action: TradeAction, source: &DataSource) -> Outcome<TradeAction> {
        match action {
            TradeAction::Add => {
                let group = self.group;
                Outcome::ask(self.add_prompt(), move |values| {
                    let name = values.text("name");
                    Some(TradeAction::Create(NewTradeEntry {
                        raw: match group {
                            TradeEntryGroup::Custom => name,
                            TradeEntryGroup::Item | TradeEntryGroup::Riven => {
                                name.to_lowercase().replace(' ', "_")
                            }
                        },
                        sub_type: values.number("rank").map(SubType::rank),
                        price: values.number("price")?,
                        group,
                        tags: values.list("tags"),
                    }))
                })
            }
            TradeAction::Create(entry) => Outcome::run_quietly(source.trade_entry_create(entry)),
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
                        "Looks through today's riven prices and adds every weapon that passes these limits, priced below its market low.",
                    )
                    .width(480.)
                    .field(Field::platinum("min_price", "Market low at least", Some(50)).half())
                    .field(Field::number("min_volume", "Trades at least", Some(10)).half())
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
            TradeAction::FoundRivens(search) => self.find_rivens(search, source),
            TradeAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => {
                    let single = ids.len() == 1;
                    Outcome::ask(spec, move |values| {
                        let tags = values.list("tags");
                        Some(TradeAction::Save {
                            ids: ids.clone(),
                            update: TradeEntryUpdate {
                                price: values.number("price"),
                                tags: (single || !tags.is_empty()).then_some(tags),
                            },
                        })
                    })
                }
                None => Outcome::Done,
            },
            TradeAction::Save { ids, update } => {
                let source = source.clone();
                Outcome::Run(Box::pin(async move {
                    for id in ids {
                        source.trade_entry_update(id as i64, update.clone()).await?;
                    }
                    Ok(String::new())
                }))
            }
            TradeAction::StepPrice { id, step } => match self.entry(id) {
                Some(entry) => Outcome::run_quietly(source.trade_entry_update(
                    entry.id,
                    TradeEntryUpdate {
                        price: Some((entry.price + step).max(1)),
                        tags: None,
                    },
                )),
                None => Outcome::Done,
            },
            TradeAction::Delete(ids) if ids.is_empty() => Outcome::Done,
            TradeAction::Delete(ids) => Outcome::ask(
                prompts::delete(ids.len(), "trade entry", "trade entries"),
                move |_| Some(TradeAction::Deleted(ids.clone())),
            ),
            TradeAction::Deleted(rows) => match rows.as_slice() {
                [id] => Outcome::run_quietly(source.trade_entry_delete(*id as i64)),
                _ => Outcome::run_quietly(source.trade_entry_delete_multiple(
                    rows.iter().map(|id| *id as i64).collect(),
                )),
            },
            TradeAction::Details(id) => Outcome::ShowPanel(id),
            TradeAction::CopyName(id) => match self.entry(id) {
                Some(entry) => Outcome::Copy {
                    what: entry.name.clone(),
                    text: entry.name,
                },
                None => Outcome::Done,
            },
            TradeAction::Export => {
                let source = source.clone();
                let params = self.params();
                Outcome::export("trade-messages.json", move |path| {
                    Box::pin(source.export_trade_entry_json(&params, path))
                })
            }
        }
    }

    fn row_id(row: &TradeEntry) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &TradeEntry, b: &TradeEntry) -> Ordering {
        match key {
            "price" => a.price.cmp(&b.price),
            "updated" => a.updated_at.cmp(&b.updated_at),
            _ => a.name.cmp(&b.name),
        }
    }

    fn cell(row: &TradeEntry, key: &str, emit: &Emit<TradeAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let id = row.id as u64;
        match key {
            "name" => cells::name(row.name.clone(), &[], cx),
            "rank" => cells::text(rank_label(row).unwrap_or_default()),
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
            "tags" => cells::tags(&row.tags, cx),
            "updated" => cells::text(updated(row)),
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
            "rank" => rank_label(row),
            "price" => Some(number(row.price)),
            "tags" => Some(row.tags.join(", ")),
            "updated" => Some(updated(row)),
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
        PanelModel {
            title: row.name.clone(),
            subtitle: rank_label(row).unwrap_or_else(|| "Trade entry".to_owned()),
            status: None,
            note: None,
            history: Vec::new(),
            lines: None,
            facts: vec![
                Fact::new("Price", number(row.price)),
                if row.tags.is_empty() {
                    Fact::new("Tags", "None").tone(Tone::Muted)
                } else {
                    Fact::new("Tags", row.tags.join(", "))
                },
                Fact::new("Updated", updated(row)),
                Fact::new("Created", day_and_time(&row.created_at)),
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
