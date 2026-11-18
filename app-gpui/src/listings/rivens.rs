//! The Rivens tab: the riven mods the live scraper sells as auctions.

use std::cmp::Ordering;

use gpui::{
    AnyElement, App, HighlightStyle, IntoElement, ParentElement, SharedString, Styled, StyledText,
    div, prelude::*, px,
};
use gpui_component::tooltip::Tooltip;
use gpui_kit_assets::IconName;

use super::{
    EXPORT_NOT_CONNECTED, MIN_PRICE_STEP, bulk_toolbar,
    model::{
        MinMax, RivenAttribute, StockRiven, StockRivenFilter, StockRivenProperties, Store,
    },
    status_chips, status_filter,
};
use crate::{
    model::{StockStatus, SubType},
    preview,
    theme::{Theme, tabular_figures},
    ui::{
        IconButton, cells, day_and_time,
        form::{Field, FormSpec, FormValues},
        grid::{
            Catalog, ChipGroup, ColumnSpec, Emit, MenuEntry, Outcome, PrimaryAction, Query, Stat,
            Tone, ToolbarItem, overflow_button,
        },
        icon, number,
        panel::{Fact, PanelAction, PanelModel},
        percent,
        prompts::{self, MessageItem},
        row_button,
    },
};

const POLARITIES: [&str; 3] = ["Madurai", "Vazarin", "Naramon"];
const ATTRIBUTE_KEYS: [&str; 4] = ["attribute_1", "attribute_2", "attribute_3", "attribute_4"];
const VALUE_KEYS: [&str; 4] = ["value_1", "value_2", "value_3", "value_4"];
const MAX_RANK: i64 = 8;

#[derive(Clone, Debug, Default)]
pub struct RivenEdit {
    bought: Option<i64>,
    min_price: Option<i64>,
    rank: Option<i64>,
    re_rolls: Option<i64>,
    mastery_rank: Option<i64>,
}

#[derive(Clone, Debug)]
pub enum RivenAction {
    Add,
    Create(Box<StockRiven>),
    Sell(u64),
    Sold { id: u64, price: i64 },
    SellAtList(u64),
    Edit(Vec<u64>),
    Save { ids: Vec<u64>, edit: RivenEdit },
    EditMinPrice(u64),
    SetMinPrice { id: u64, price: Option<i64> },
    StepMinPrice { id: u64, step: i64 },
    EditFilter(u64),
    SetFilter { id: u64, filter: StockRivenFilter },
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

pub struct Rivens {
    store: Store<StockRiven>,
}

impl Rivens {
    pub fn new(rows: Vec<StockRiven>) -> Self {
        Self {
            store: Store::new(rows),
        }
    }

    fn riven(&self, id: u64) -> Option<StockRiven> {
        self.store.get(id as i64).cloned()
    }

    fn create_prompt() -> FormSpec {
        let mut spec = FormSpec::new("Add riven", "Add riven")
            .width(560.)
            .field(Field::choice("weapon", "Weapon", preview::riven_weapons(), None).half())
            .field(Field::text("name", "Riven name", "", "Crita-visican").half())
            .field(Field::number("mastery_rank", "Mastery rank", Some(8)).min(8).half())
            .field(Field::number("re_rolls", "Re-rolls", Some(0)).half())
            .field(Field::number("rank", "Rank", Some(0)).half())
            .field(
                Field::choice(
                    "polarity",
                    "Polarity",
                    POLARITIES.map(str::to_owned).to_vec(),
                    Some(0),
                )
                .half(),
            )
            .field(Field::platinum("bought", "Bought", Some(0)));
        for (index, (attribute, value)) in ATTRIBUTE_KEYS.into_iter().zip(VALUE_KEYS).enumerate() {
            let number = index + 1;
            let choice = Field::choice(
                attribute,
                &format!("Attribute {number}"),
                preview::riven_attributes(),
                None,
            )
            .half();
            let amount = Field::text(value, "Value in percent", "", "120.5 or -40").half();
            spec = if index < 2 {
                spec.field(choice).field(amount)
            } else {
                spec.field(choice.optional()).field(amount.optional())
            };
        }
        spec
    }

    fn created(values: &FormValues) -> Option<RivenAction> {
        let weapon = values.choice("weapon")?;
        let attributes = ATTRIBUTE_KEYS
            .into_iter()
            .zip(VALUE_KEYS)
            .filter_map(|(attribute, value)| {
                let name = values.choice(attribute)?;
                let value = values.text(value).replace('%', "").parse::<f64>().ok()?;
                Some(RivenAttribute {
                    url_name: name.to_lowercase().replace(' ', "_"),
                    name,
                    value,
                    positive: value >= 0.0,
                })
            })
            .collect();
        Some(RivenAction::Create(Box::new(StockRiven {
            id: 0,
            wfm_weapon_url: weapon.to_lowercase().replace(' ', "_"),
            weapon_name: weapon,
            weapon_type: String::new(),
            mod_name: values.text("name"),
            attributes,
            mastery_rank: values.number("mastery_rank")?,
            re_rolls: values.number("re_rolls")?,
            polarity: values.choice("polarity")?,
            sub_type: Some(SubType::rank(values.number("rank")?.min(MAX_RANK))),
            bought: values.number("bought")?,
            list_price: None,
            is_hidden: false,
            status: StockStatus::Pending,
            price_history: Vec::new(),
            updated_at: String::new(),
            comment: String::new(),
            filter: StockRivenFilter::default(),
            properties: StockRivenProperties::default(),
        })))
    }

    fn edit_prompt(&self, ids: &[u64]) -> Option<FormSpec> {
        let single = match ids {
            [id] => Some(self.riven(*id)?),
            [] => return None,
            _ => None,
        };
        let value = |pick: fn(&StockRiven) -> Option<i64>| single.as_ref().and_then(pick);
        let bulk = single.is_none();
        let optional = |field: Field| if bulk { field.optional() } else { field };
        let spec = match &single {
            Some(riven) => FormSpec::new(format!("Edit {}", riven.full_name()), "Save"),
            None => FormSpec::new(format!("Edit {} rivens", ids.len()), "Save").description(
                "Fill in only the fields to change. Empty fields keep each riven's value.",
            ),
        };
        Some(
            spec.width(480.)
                .field(optional(
                    Field::platinum("bought", "Bought", value(|riven| Some(riven.bought))).half(),
                ))
                .field(
                    Field::platinum(
                        "min_price",
                        "Minimum price",
                        value(|riven| riven.properties.min_price),
                    )
                    .optional()
                    .half(),
                )
                .field(optional(
                    Field::number("rank", "Rank", value(|riven| Some(riven.rank()))).half(),
                ))
                .field(optional(
                    Field::number("re_rolls", "Re-rolls", value(|riven| Some(riven.re_rolls)))
                        .half(),
                ))
                .field(optional(
                    Field::number(
                        "mastery_rank",
                        "Mastery rank",
                        value(|riven| Some(riven.mastery_rank)),
                    )
                    .min(8)
                    .half(),
                )),
        )
    }

    fn filter_prompt(riven: &StockRiven) -> FormSpec {
        let filter = &riven.filter;
        FormSpec::new(format!("Auction filter for {}", riven.full_name()), "Save")
            .description(
                "The live scraper prices this riven against auctions that pass the filter. With the filter off it compares every auction for the weapon.",
            )
            .width(480.)
            .field(Field::toggle("enabled", "Use the filter", filter.enabled))
            .field(
                Field::number("similarity", "Similarity at least", filter.similarity)
                    .unit("percent")
                    .optional()
                    .half(),
            )
            .field(
                Field::number("max_re_rolls", "Re-rolls at most", filter.re_rolls.max)
                    .optional()
                    .half(),
            )
            .field(
                Field::number(
                    "max_mastery_rank",
                    "Mastery rank at most",
                    filter.mastery_rank.max,
                )
                .optional()
                .half(),
            )
            .field(Field::toggle(
                "required_negative",
                "Require a negative attribute",
                filter.required_negative,
            ))
    }

    fn message_items(&self, ids: &[u64]) -> Vec<MessageItem> {
        ids.iter()
            .filter_map(|id| self.riven(*id))
            .filter(|riven| riven.list_price.is_some())
            .map(|riven| MessageItem {
                name: riven.full_name(),
                rank: None,
                price: riven.list_price,
            })
            .collect()
    }
}

/// The attributes of a riven on one line, with the negative ones in red.
/// A line too long for the cell ends in an ellipsis, and the tooltip has
/// every attribute.
pub fn attributes_cell(id: u64, attributes: &[RivenAttribute], cx: &App) -> AnyElement {
    let colors = Theme::global(cx).colors;
    let mut line = String::new();
    let mut highlights = Vec::new();
    for attribute in attributes {
        if !line.is_empty() {
            line.push_str("   ");
        }
        let start = line.len();
        line.push_str(&attribute.label());
        if !attribute.positive {
            highlights.push((
                start..line.len(),
                HighlightStyle {
                    color: Some(colors.negative),
                    ..HighlightStyle::default()
                },
            ));
        }
    }
    let tooltip = SharedString::from(
        attributes
            .iter()
            .map(RivenAttribute::label)
            .collect::<Vec<_>>()
            .join(", "),
    );
    div()
        .id(("attributes", id))
        .size_full()
        .flex()
        .items_center()
        .min_w_0()
        .child(
            div()
                .truncate()
                .font_features(tabular_figures())
                .child(StyledText::new(line).with_highlights(highlights)),
        )
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .into_any_element()
}

impl Catalog for Rivens {
    type Row = StockRiven;
    type Action = RivenAction;

    const NOUN: &'static str = "rivens";
    const SEARCH_PLACEHOLDER: &'static str = "Search rivens";
    const EMPTY_HINT: &'static str = "Add a riven you own and the live scraper will price it and put it up for auction.";

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("riven", "Riven", 196.).sortable(),
            ColumnSpec::new("attributes", "Attributes", 200.).flex(),
            ColumnSpec::new("re_rolls", "Rolls", 62.).end().sortable().droppable(2),
            ColumnSpec::new("bought", "Bought", 84.).end().sortable().droppable(1),
            ColumnSpec::new("list_price", "List price", 108.).end().sortable(),
            ColumnSpec::new("min_price", "Min price", 136.).sortable().droppable(3),
            ColumnSpec::new("status", "Status", 134.).sortable(),
            ColumnSpec::new("actions", "", 150.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<StockRiven> {
        self.store
            .list(&query.text, status_filter(&self.store, query))
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        vec![status_chips(&self.store, cx)]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = self.store.rows();
        let bought: i64 = rows.iter().map(|riven| riven.bought).sum();
        let listed: i64 = rows.iter().filter_map(|riven| riven.list_price).sum();
        let listed_cost: i64 = rows
            .iter()
            .filter(|riven| riven.list_price.is_some())
            .map(|riven| riven.bought)
            .sum();
        let profit = listed - listed_cost;
        let live = rows
            .iter()
            .filter(|riven| riven.status == StockStatus::Live)
            .count();
        vec![
            Stat {
                label: "Bought",
                value: number(bought),
                note: format!("{} rivens in stock", rows.len()),
                tone: Tone::Plain,
            },
            Stat {
                label: "Listed",
                value: number(listed),
                note: "at current list prices".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Profit",
                value: number(profit),
                note: if listed_cost > 0 {
                    format!(
                        "{} return on listed rivens",
                        percent(profit as f64 / listed_cost as f64)
                    )
                } else {
                    String::new()
                },
                tone: Tone::of_profit(profit),
            },
            Stat {
                label: "Rivens",
                value: number(rows.len() as i64),
                note: format!("{live} live on Warframe Market"),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, checked: &[u64]) -> Vec<ToolbarItem<RivenAction>> {
        bulk_toolbar(
            checked,
            "rivens",
            RivenAction::Export,
            RivenAction::Edit,
            Some(RivenAction::Message),
            RivenAction::Delete,
        )
    }

    fn primary(&self) -> Option<PrimaryAction<RivenAction>> {
        Some(PrimaryAction {
            label: "Add riven",
            icon: IconName::Plus,
            action: RivenAction::Add,
        })
    }

    fn perform(&mut self, action: RivenAction) -> Outcome<RivenAction> {
        match action {
            RivenAction::Add => Outcome::ask(Self::create_prompt(), Self::created),
            RivenAction::Create(riven) => {
                let name = riven.full_name();
                self.store.insert(*riven);
                Outcome::Notice(format!("Added {name} to stock."))
            }
            RivenAction::Sell(id) => match self.riven(id) {
                Some(riven) => Outcome::ask(
                    prompts::sold(&riven.full_name(), None, riven.list_price),
                    move |values| {
                        Some(RivenAction::Sold {
                            id,
                            price: values.number("price")?,
                        })
                    },
                ),
                None => Outcome::Done,
            },
            RivenAction::SellAtList(id) => match self.riven(id).and_then(|riven| riven.list_price)
            {
                Some(price) => self.perform(RivenAction::Sold { id, price }),
                None => Outcome::Done,
            },
            RivenAction::Sold { id, price } => match self.riven(id) {
                Some(riven) => {
                    self.store.delete(&[id as i64]);
                    Outcome::Notice(format!(
                        "Sold {} for {} platinum.",
                        riven.full_name(),
                        number(price)
                    ))
                }
                None => Outcome::Done,
            },
            RivenAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => Outcome::ask(spec, move |values| {
                    Some(RivenAction::Save {
                        ids: ids.clone(),
                        edit: RivenEdit {
                            bought: values.number("bought"),
                            min_price: values.number("min_price"),
                            rank: values.number("rank"),
                            re_rolls: values.number("re_rolls"),
                            mastery_rank: values.number("mastery_rank"),
                        },
                    })
                }),
                None => Outcome::Done,
            },
            RivenAction::Save { ids, edit } => {
                let single = ids.len() == 1;
                for id in &ids {
                    self.store.update(*id as i64, |riven| {
                        riven.bought = edit.bought.unwrap_or(riven.bought);
                        riven.re_rolls = edit.re_rolls.unwrap_or(riven.re_rolls);
                        riven.mastery_rank = edit.mastery_rank.unwrap_or(riven.mastery_rank);
                        if let Some(rank) = edit.rank {
                            riven.sub_type = Some(SubType::rank(rank.min(MAX_RANK)));
                        }
                        riven.properties.min_price = if single {
                            edit.min_price
                        } else {
                            edit.min_price.or(riven.properties.min_price)
                        };
                    });
                }
                Outcome::Done
            }
            RivenAction::EditMinPrice(id) => match self.riven(id) {
                Some(riven) => Outcome::ask(
                    prompts::price_limit(
                        &format!("Minimum price for {}", riven.full_name()),
                        "Minimum price",
                        "The live scraper never lists below this. Leave it empty for no minimum.",
                        riven.properties.min_price,
                    ),
                    move |values| {
                        Some(RivenAction::SetMinPrice {
                            id,
                            price: values.number("price"),
                        })
                    },
                ),
                None => Outcome::Done,
            },
            RivenAction::SetMinPrice { id, price } => {
                self.store.update(id as i64, |riven| {
                    riven.properties.min_price = price.filter(|price| *price > 0);
                });
                Outcome::Done
            }
            RivenAction::StepMinPrice { id, step } => {
                self.store.update(id as i64, |riven| {
                    let next = riven.properties.min_price.unwrap_or(0) + step;
                    riven.properties.min_price = (next > 0).then_some(next);
                });
                Outcome::Done
            }
            RivenAction::EditFilter(id) => match self.riven(id) {
                Some(riven) => Outcome::ask(Self::filter_prompt(&riven), move |values| {
                    Some(RivenAction::SetFilter {
                        id,
                        filter: StockRivenFilter {
                            enabled: values.flag("enabled"),
                            re_rolls: MinMax {
                                min: None,
                                max: values.number("max_re_rolls"),
                            },
                            mastery_rank: MinMax {
                                min: None,
                                max: values.number("max_mastery_rank"),
                            },
                            required_negative: values.flag("required_negative"),
                            similarity: values.number("similarity"),
                        },
                    })
                }),
                None => Outcome::Done,
            },
            RivenAction::SetFilter { id, filter } => {
                self.store.update(id as i64, |riven| riven.filter = filter);
                Outcome::Done
            }
            RivenAction::ToggleHidden(id) => {
                self.store
                    .update(id as i64, |riven| riven.is_hidden = !riven.is_hidden);
                Outcome::Done
            }
            RivenAction::Delete(ids) if ids.is_empty() => Outcome::Done,
            RivenAction::Delete(ids) => Outcome::ask(
                prompts::delete(ids.len(), "riven", "rivens"),
                move |_| Some(RivenAction::Deleted(ids.clone())),
            ),
            RivenAction::Deleted(ids) => {
                let ids: Vec<i64> = ids.iter().map(|id| *id as i64).collect();
                self.store.delete(&ids);
                Outcome::Done
            }
            RivenAction::Details(id) => Outcome::ShowPanel(id),
            RivenAction::CopyName(id) => match self.riven(id) {
                Some(riven) => Outcome::Copy {
                    what: riven.full_name(),
                    text: riven.full_name(),
                },
                None => Outcome::Done,
            },
            RivenAction::OpenMarket(id) => match self.riven(id) {
                Some(riven) => Outcome::OpenUrl(format!(
                    "https://warframe.market/auctions/search?type=riven&weapon_url_name={}",
                    riven.wfm_weapon_url
                )),
                None => Outcome::Done,
            },
            RivenAction::Message(ids) => {
                let items = self.message_items(&ids);
                if items.is_empty() {
                    return Outcome::Notice(
                        "None of the selected rivens has a list price yet.".to_owned(),
                    );
                }
                Outcome::ask(prompts::message("WTS", items.clone()), move |values| {
                    Some(RivenAction::CopyMessage(prompts::compose_message(
                        values, &items,
                    )))
                })
            }
            RivenAction::CopyMessage(text) => Outcome::Copy {
                text,
                what: "the trade chat message".to_owned(),
            },
            RivenAction::Export => Outcome::Notice(EXPORT_NOT_CONNECTED.to_owned()),
        }
    }

    fn row_id(row: &StockRiven) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &StockRiven, b: &StockRiven) -> Ordering {
        match key {
            "re_rolls" => a.re_rolls.cmp(&b.re_rolls),
            "bought" => a.bought.cmp(&b.bought),
            "list_price" => a.list_price.cmp(&b.list_price),
            "min_price" => a.properties.min_price.cmp(&b.properties.min_price),
            "status" => a.status.label().cmp(b.status.label()),
            _ => a.full_name().cmp(&b.full_name()),
        }
    }

    fn cell(row: &StockRiven, key: &str, emit: &Emit<RivenAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let id = row.id as u64;
        match key {
            "riven" => {
                let mut flags = Vec::new();
                if row.is_hidden {
                    flags.push((IconName::EyeOff, "Hidden from Warframe Market"));
                }
                if row.filter.enabled {
                    flags.push((IconName::Funnel, "The auction filter is on"));
                }
                cells::name(row.full_name(), &flags, cx)
            }
            "attributes" => attributes_cell(id, &row.attributes, cx),
            "re_rolls" => cells::amount(Some(row.re_rolls), "", cx),
            "bought" => cells::amount(Some(row.bought), "", cx),
            "list_price" => match row.list_price {
                Some(price) if row.properties.is_predicted => div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(6.))
                    .child(
                        div()
                            .id(("predicted", id))
                            .child(icon(
                                IconName::Bot,
                                theme.metrics.icon - px(2.),
                                colors.positive,
                            ))
                            .tooltip(|window, cx| {
                                Tooltip::new("Priced by the riven price model").build(window, cx)
                            }),
                    )
                    .child(
                        div()
                            .font_features(tabular_figures())
                            .text_color(colors.text)
                            .child(number(price)),
                    )
                    .into_any_element(),
                price => cells::amount(price, "Not listed", cx),
            },
            "min_price" => cells::stepped_price(
                id,
                row.properties.min_price,
                MIN_PRICE_STEP,
                RivenAction::StepMinPrice {
                    id,
                    step: -MIN_PRICE_STEP,
                },
                RivenAction::StepMinPrice {
                    id,
                    step: MIN_PRICE_STEP,
                },
                emit,
                cx,
            ),
            "status" => cells::status(theme.status_color(row.status), row.status.label(), cx),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(
                        div().pr(px(5.)).child(
                            row_button(("sell", id), "Sell", cx)
                                .on_click(emit.on_click(RivenAction::Sell(id))),
                        ),
                    )
                    .child(
                        IconButton::new(
                            ("sell-list", id),
                            IconName::Gavel,
                            "Sell at the list price",
                        )
                        .in_row()
                        .disabled(row.list_price.is_none())
                        .on_click(emit.on_click(RivenAction::SellAtList(id))),
                    )
                    .child(
                        IconButton::new(("edit", id), IconName::SquarePen, "Edit")
                            .in_row()
                            .on_click(emit.on_click(RivenAction::Edit(vec![id]))),
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

    fn cell_text(row: &StockRiven, key: &str) -> Option<String> {
        match key {
            "riven" => Some(row.full_name()),
            "attributes" => Some(
                row.attributes
                    .iter()
                    .map(RivenAttribute::label)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            "re_rolls" => Some(number(row.re_rolls)),
            "bought" => Some(number(row.bought)),
            "list_price" => row.list_price.map(number),
            "min_price" => row.properties.min_price.map(number),
            "status" => Some(row.status.label().to_owned()),
            _ => None,
        }
    }

    fn row_menu(row: &StockRiven) -> Vec<MenuEntry<RivenAction>> {
        let id = row.id as u64;
        let mut menu = vec![MenuEntry::new(
            "Sell",
            IconName::HandCoins,
            RivenAction::Sell(id),
        )];
        if row.list_price.is_some() {
            menu.push(MenuEntry::new(
                "Sell at the list price",
                IconName::Gavel,
                RivenAction::SellAtList(id),
            ));
        }
        menu.extend([
            MenuEntry::new("Edit", IconName::SquarePen, RivenAction::Edit(vec![id])),
            MenuEntry::new(
                "Set minimum price",
                IconName::Tag,
                RivenAction::EditMinPrice(id),
            ),
            MenuEntry::new(
                "Set auction filter",
                IconName::Funnel,
                RivenAction::EditFilter(id),
            ),
            MenuEntry::new("Show details", IconName::PanelRight, RivenAction::Details(id)),
            if row.is_hidden {
                MenuEntry::new(
                    "Show on Warframe Market",
                    IconName::Eye,
                    RivenAction::ToggleHidden(id),
                )
            } else {
                MenuEntry::new(
                    "Hide from Warframe Market",
                    IconName::EyeOff,
                    RivenAction::ToggleHidden(id),
                )
            }
            .separated(),
            MenuEntry::new(
                "Write a trade chat message",
                IconName::MessageSquare,
                RivenAction::Message(vec![id]),
            ),
            MenuEntry::new("Copy name", IconName::Copy, RivenAction::CopyName(id)),
            MenuEntry::new(
                "Find auctions for this weapon",
                IconName::ExternalLink,
                RivenAction::OpenMarket(id),
            ),
            MenuEntry::new("Delete", IconName::Trash, RivenAction::Delete(vec![id])).separated(),
        ]);
        menu
    }

    fn panel(row: &StockRiven, cx: &App) -> PanelModel<RivenAction> {
        let theme = Theme::global(cx);
        let id = row.id as u64;
        let profit = row.list_price.map(|price| price - row.bought);
        let filter = &row.filter;
        PanelModel {
            title: row.full_name(),
            subtitle: format!("{}, {} polarity", row.weapon_type, row.polarity),
            status: Some((
                row.status.label().to_owned(),
                theme.status_color(row.status),
            )),
            note: Some(row.status.detail().to_owned()),
            history: row.price_history.iter().map(|check| check.price).collect(),
            lines: Some((
                "Attributes",
                row.attributes
                    .iter()
                    .map(|attribute| {
                        (
                            attribute.label(),
                            if attribute.positive {
                                Tone::Plain
                            } else {
                                Tone::Negative
                            },
                        )
                    })
                    .collect(),
            )),
            facts: vec![
                Fact::new("Bought", number(row.bought)),
                if row.properties.is_predicted {
                    Fact::new(
                        "List price",
                        row.list_price
                            .map_or("Not listed".to_owned(), |price| {
                                format!("{}, from the price model", number(price))
                            }),
                    )
                } else {
                    Fact::amount("List price", row.list_price, "Not listed")
                },
                Fact::amount("Minimum price", row.properties.min_price, "Not set"),
                match profit {
                    Some(profit) => Fact::new("Profit", number(profit)).tone(Tone::of_profit(profit)),
                    None => Fact::new("Profit", "Not listed").tone(Tone::Muted),
                },
                Fact::new("Mastery rank", number(row.mastery_rank)),
                Fact::new("Rank", format!("{} of {MAX_RANK}", row.rank())),
                Fact::new("Re-rolls", number(row.re_rolls)),
                if filter.enabled {
                    Fact::new(
                        "Auction filter",
                        match filter.similarity {
                            Some(similarity) => format!("On, {similarity}% similar or more"),
                            None => "On".to_owned(),
                        },
                    )
                } else {
                    Fact::new("Auction filter", "Off").tone(Tone::Muted)
                },
                Fact::new("Last checked", day_and_time(&row.updated_at)),
            ],
            primary: vec![
                PanelAction::new("Sell", IconName::HandCoins, RivenAction::Sell(id)),
                PanelAction::new(
                    "Sell at the list price",
                    IconName::Gavel,
                    RivenAction::SellAtList(id),
                )
                .enabled(row.list_price.is_some()),
            ],
            secondary: vec![
                PanelAction::new("Edit", IconName::SquarePen, RivenAction::Edit(vec![id])),
                PanelAction::new(
                    "Set minimum price",
                    IconName::Tag,
                    RivenAction::EditMinPrice(id),
                ),
                PanelAction::new(
                    "Set auction filter",
                    IconName::Funnel,
                    RivenAction::EditFilter(id),
                ),
                if row.is_hidden {
                    PanelAction::new(
                        "Show on Warframe Market",
                        IconName::Eye,
                        RivenAction::ToggleHidden(id),
                    )
                } else {
                    PanelAction::new(
                        "Hide from Warframe Market",
                        IconName::EyeOff,
                        RivenAction::ToggleHidden(id),
                    )
                },
                PanelAction::new(
                    "Write a trade chat message",
                    IconName::MessageSquare,
                    RivenAction::Message(vec![id]),
                )
                .enabled(row.list_price.is_some()),
                PanelAction::new("Copy name", IconName::Copy, RivenAction::CopyName(id)),
                PanelAction::new(
                    "Find auctions for this weapon",
                    IconName::ExternalLink,
                    RivenAction::OpenMarket(id),
                ),
                PanelAction::new("Delete", IconName::Trash, RivenAction::Delete(vec![id])).danger(),
            ],
        }
    }

    fn dimmed(row: &StockRiven) -> bool {
        row.is_hidden
    }
}
