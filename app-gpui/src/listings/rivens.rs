use std::cmp::Ordering;

use gpui::{
    AnyElement, App, HighlightStyle, IntoElement, ParentElement, SharedString, Styled, StyledText,
    div, prelude::*, px,
};
use gpui_component::tooltip::Tooltip;
use gpui_kit_assets::IconName;

use super::{
    StockData, bulk_toolbar, every_row, ids, limit_cell, limit_field, load_stock, slug,
    status_chips, status_filter, stepped,
};
use crate::{
    data::{DataKind, DataSource},
    model::{
        MinMax, NewStockRiven, RivenAttribute, RivenFilter, RivenFilterAttribute, RivenStat,
        RivenWeapon, StockRiven, StockRivenUpdate, StockStatus,
    },
    theme::{Theme, tabular_figures},
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

const POLARITIES: [&str; 3] = ["madurai", "vazarin", "naramon"];
const ATTRIBUTE_KEYS: [&str; 4] = ["attribute_1", "attribute_2", "attribute_3", "attribute_4"];
const VALUE_KEYS: [&str; 4] = ["value_1", "value_2", "value_3", "value_4"];
const REQUIRED_KEYS: [&str; 4] = ["required_1", "required_2", "required_3", "required_4"];
const MAX_RANK: i64 = 8;

pub fn words(slug: &str) -> String {
    slug.split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn attribute_label(attribute: &RivenAttribute) -> String {
    let sign = if attribute.value < 0.0 || !attribute.positive {
        "-"
    } else {
        "+"
    };
    let label = if attribute.label.is_empty() {
        words(&attribute.wfm_url)
    } else {
        attribute.label.clone()
    };
    format!("{sign}{:.1}% {label}", attribute.value.abs())
}

pub fn attributes_text(attributes: &[RivenAttribute]) -> Vec<String> {
    attributes.iter().map(attribute_label).collect()
}

pub fn attribute_lines(attributes: &[RivenAttribute]) -> Vec<(String, Tone)> {
    attributes
        .iter()
        .map(|attribute| {
            let tone = if attribute.positive {
                Tone::Plain
            } else {
                Tone::Negative
            };
            (attribute_label(attribute), tone)
        })
        .collect()
}

pub fn attributes_cell(id: u64, attributes: &[RivenAttribute], cx: &App) -> AnyElement {
    let colors = Theme::global(cx).colors;
    let mut line = String::new();
    let mut highlights = Vec::new();
    for attribute in attributes {
        if !line.is_empty() {
            line.push_str("   ");
        }
        let start = line.len();
        line.push_str(&attribute_label(attribute));
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
    let tooltip = SharedString::from(attributes_text(attributes).join(", "));
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

fn list_price_cell(row: &StockRiven, cx: &App) -> AnyElement {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let Some(price) = row.list_price else {
        return cells::amount(None, "Not listed", cx);
    };
    div()
        .id(("list-price", row.id as u64))
        .size_full()
        .flex()
        .items_center()
        .justify_end()
        .gap(px(6.))
        .when(row.properties.is_predicted, |cell| {
            cell.child(crate::ui::icon(
                IconName::Bot,
                theme.metrics.icon - px(2.),
                colors.text_faint,
            ))
            .tooltip(|window, cx| {
                Tooltip::new("Predicted by the price model, not taken from live auctions")
                    .build(window, cx)
            })
        })
        .child(
            div()
                .font_features(tabular_figures())
                .text_color(colors.text)
                .child(number(price)),
        )
        .into_any_element()
}

fn filter_summary(filter: &RivenFilter) -> String {
    if !filter.enabled {
        return "Off".to_owned();
    }
    let required = filter
        .attributes
        .iter()
        .filter(|attribute| attribute.is_required)
        .count();
    match required {
        0 => "On".to_owned(),
        1 => "On, 1 required attribute".to_owned(),
        count => format!("On, {count} required attributes"),
    }
}

#[derive(Clone, Debug)]
pub enum RivenAction {
    Add,
    Create(NewStockRiven),
    Sell(u64),
    Sold {
        id: u64,
        price: i64,
    },
    SellAtList(u64),
    Edit(Vec<u64>),
    Save {
        ids: Vec<u64>,
        update: StockRivenUpdate,
    },
    StepMinPrice {
        id: u64,
        step: i64,
    },
    Filter(u64),
    SaveFilter {
        id: u64,
        filter: RivenFilter,
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

type RivenLists = (Vec<RivenWeapon>, Vec<RivenStat>);

#[derive(Default)]
pub struct Rivens {
    data: StockData<StockRiven>,
    weapons: Vec<RivenWeapon>,
    stats: Vec<RivenStat>,
}

const MIN_MASTERY: i64 = 8;
const MAX_MASTERY: i64 = 16;

impl Rivens {
    fn riven(&self, id: u64) -> Option<StockRiven> {
        self.data
            .rows
            .iter()
            .find(|riven| riven.id as u64 == id)
            .cloned()
    }

    fn create_prompt(&self) -> FormSpec {
        let weapon = if self.weapons.is_empty() {
            Field::text("weapon", "Weapon", "", "Rubico")
                .hint("The weapon's name on Warframe Market.")
        } else {
            Field::search("weapon", "Weapon", choices::weapons(&self.weapons), None)
        };
        let mut spec = FormSpec::new("Add riven", "Add riven")
            .width(560.)
            .field(weapon.half())
            .field(Field::text("name", "Riven name", "", "Crita-visican").half())
            .field(
                Field::number("mastery_rank", "Mastery rank", Some(8))
                    .min(8)
                    .half(),
            )
            .field(Field::number("re_rolls", "Re-rolls", Some(0)).half())
            .field(Field::number("rank", "Rank", Some(0)).half())
            .field(
                Field::choice(
                    "polarity",
                    "Polarity",
                    POLARITIES.iter().map(|polarity| words(polarity)).collect(),
                    Some(0),
                )
                .half(),
            )
            .field(Field::platinum("bought", "Bought", Some(0)));
        let stats = choices::stats(&self.stats);
        for (index, (attribute, value)) in ATTRIBUTE_KEYS.into_iter().zip(VALUE_KEYS).enumerate() {
            let label = format!("Attribute {}", index + 1);
            let choice = if stats.is_empty() {
                Field::text(attribute, &label, "", "critical_chance")
                    .hint("The attribute's slug on Warframe Market.")
            } else {
                Field::search(attribute, &label, stats.clone(), None)
            }
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
        let attributes = ATTRIBUTE_KEYS
            .into_iter()
            .zip(VALUE_KEYS)
            .filter_map(|(attribute, value)| {
                let wfm_url = values
                    .choice(attribute)
                    .or_else(|| Some(slug(&values.text(attribute))).filter(|s| !s.is_empty()))?;
                let value = values.text(value).replace('%', "").parse::<f64>().ok()?;
                Some(RivenAttribute::new(wfm_url, value >= 0.0, value.abs()))
            })
            .collect();
        let weapon = values
            .choice("weapon")
            .unwrap_or_else(|| slug(&values.text("weapon")));
        Some(RivenAction::Create(NewStockRiven {
            wfm_weapon_url: weapon,
            mod_name: values.text("name"),
            mastery_rank: values.number("mastery_rank")?,
            re_rolls: values.number("re_rolls")?,
            rank: values.number("rank")?.min(MAX_RANK),
            polarity: values.choice("polarity")?.to_lowercase(),
            attributes,
            bought: values.number("bought")?,
        }))
    }

    fn edit_prompt(&self, ids: &[u64]) -> Option<FormSpec> {
        let single = match ids {
            [id] => Some(self.riven(*id)?),
            [] => return None,
            _ => None,
        };
        let value = |pick: fn(&StockRiven) -> i64| single.as_ref().map(pick);
        let bulk = single.is_none();
        let optional = |field: Field| if bulk { field.optional() } else { field };
        let spec = match &single {
            Some(riven) => FormSpec::new(format!("Edit {}", riven.title()), "Save"),
            None => FormSpec::new(format!("Edit {} rivens", ids.len()), "Save").description(
                "Fill in only the fields to change. Empty fields keep each riven's value.",
            ),
        };
        Some(
            spec.field(optional(Field::platinum(
                "bought",
                "Bought",
                value(|riven| riven.bought),
            )))
            .field(optional(
                Field::number("re_rolls", "Re-rolls", value(|riven| riven.re_rolls)).half(),
            ))
            .field(optional(
                Field::number(
                    "mastery_rank",
                    "Mastery rank",
                    value(|riven| riven.mastery_rank),
                )
                .min(8)
                .half(),
            ))
            .field(
                Field::platinum(
                    "min_price",
                    "Minimum price",
                    single.as_ref().and_then(|riven| riven.properties.min_price),
                )
                .optional()
                .hint("The scraper never lists below it. Empty sets no floor."),
            ),
        )
    }

    fn filter_prompt(riven: &StockRiven) -> FormSpec {
        let filter = &riven.filter;
        let range = |value: Option<MinMax>| (value.map(|v| v.min), value.and_then(|v| v.max));
        let (rolls_min, rolls_max) = range(filter.re_rolls);
        let (mastery_min, mastery_max) = range(filter.mastery_rank);
        let mut spec = FormSpec::new(format!("Auction filter for {}", riven.title()), "Save")
            .width(520.)
            .description(
                "The live scraper prices this riven against the auctions that pass the filter.",
            )
            .field(Field::toggle("enabled", "Use the filter", filter.enabled))
            .field(
                Field::number("rolls_min", "Re-rolls from", rolls_min)
                    .optional()
                    .half(),
            )
            .field(
                Field::number("rolls_max", "Re-rolls to", rolls_max)
                    .optional()
                    .half(),
            )
            .field(
                Field::number("mastery_min", "Mastery rank from", mastery_min)
                    .min(MIN_MASTERY)
                    .max(MAX_MASTERY)
                    .optional()
                    .half(),
            )
            .field(
                Field::number("mastery_max", "Mastery rank to", mastery_max)
                    .min(MIN_MASTERY)
                    .max(MAX_MASTERY)
                    .optional()
                    .half(),
            )
            .field(
                Field::number(
                    "similarity",
                    "Least similarity",
                    filter.similarity.map(|value| value.round() as i64),
                )
                .max(100)
                .unit("%")
                .optional()
                .half(),
            )
            .field(Field::toggle(
                "required_negative",
                "Needs a negative attribute",
                filter.required_negative,
            ));
        for (index, attribute) in riven
            .attributes
            .iter()
            .enumerate()
            .take(ATTRIBUTE_KEYS.len())
        {
            let required = filter
                .attributes
                .iter()
                .any(|entry| entry.wfm_url == attribute.wfm_url && entry.is_required);
            spec = spec.field(Field::toggle(
                REQUIRED_KEYS[index],
                &format!("Require {}", attribute_label(attribute)),
                required,
            ));
        }
        spec
    }

    fn filter_saved(riven: &StockRiven, values: &FormValues) -> RivenFilter {
        let range = |min: &str, max: &str| {
            values.number(min).map(|low| MinMax {
                min: low,
                max: values.number(max),
            })
        };
        RivenFilter {
            enabled: values.flag("enabled"),
            mastery_rank: range("mastery_min", "mastery_max"),
            re_rolls: range("rolls_min", "rolls_max"),
            polarity: riven.filter.polarity.clone(),
            similarity: values
                .number("similarity")
                .filter(|value| *value > 0)
                .map(|value| value as f64),
            required_negative: values.flag("required_negative"),
            attributes: riven
                .attributes
                .iter()
                .zip(REQUIRED_KEYS)
                .map(|(attribute, key)| RivenFilterAttribute {
                    wfm_url: attribute.wfm_url.clone(),
                    positive: attribute.positive,
                    is_required: values.flag(key),
                })
                .collect(),
        }
    }

    fn message_items(&self, ids: &[u64]) -> Vec<MessageItem> {
        ids.iter()
            .filter_map(|id| self.riven(*id))
            .filter(|riven| riven.list_price.is_some())
            .map(|riven| MessageItem {
                name: riven.title(),
                rank: None,
                price: riven.list_price,
            })
            .collect()
    }
}

impl Catalog for Rivens {
    type Row = StockRiven;
    type Action = RivenAction;
    type Data = (StockData<StockRiven>, Option<RivenLists>);

    const NOUN: &'static str = "rivens";
    const SEARCH_PLACEHOLDER: &'static str = "Search rivens";
    const EMPTY_HINT: &'static str =
        "Add a riven you own and the live scraper will price it and put it up for auction.";
    const WATCH: &'static [DataKind] = &[DataKind::StockRivens];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let lists = (self.weapons.is_empty() || self.stats.is_empty()).then(|| {
            let weapons = source.cache_get_riven_weapons();
            let stats = source.cache_get_riven_attributes();
            async move { Ok((weapons.await?, stats.await?)) }
        });
        with_cache(
            load_stock(
                source.get_stock_riven_pagination(&every_row()),
                source.get_stock_riven_status_counts(),
                source.get_stock_riven_financial_report(),
            ),
            lists,
        )
    }

    fn apply(&mut self, (data, lists): Self::Data) {
        self.data = data;
        if let Some((weapons, stats)) = lists {
            self.weapons = weapons;
            self.stats = stats;
        }
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("riven", "Riven", 204.).sortable(),
            ColumnSpec::new("attributes", "Attributes", 200.).flex(),
            ColumnSpec::new("mastery", "MR", 56.)
                .end()
                .sortable()
                .droppable(3),
            ColumnSpec::new("re_rolls", "Rolls", 62.)
                .end()
                .sortable()
                .droppable(2),
            ColumnSpec::new("bought", "Bought", 84.)
                .end()
                .sortable()
                .droppable(1),
            ColumnSpec::new("list_price", "List price", 100.)
                .end()
                .sortable(),
            ColumnSpec::new("min_price", "Min price", 124.)
                .sortable()
                .droppable(4),
            ColumnSpec::new("status", "Status", 140.).sortable(),
            ColumnSpec::new("actions", "", 178.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<StockRiven> {
        let status = status_filter(query);
        self.data
            .rows
            .iter()
            .filter(|riven| status.is_none_or(|status| riven.status == status))
            .filter(|riven| query.matches(&riven.title()))
            .cloned()
            .collect()
    }

    fn chips(&self, cx: &App) -> Vec<ChipGroup> {
        vec![status_chips(&self.data.counts, cx)]
    }

    fn summary(&self) -> Vec<Stat> {
        let rows = &self.data.rows;
        let report = &self.data.report;
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
                note: format!("{} rivens in stock", rows.len()),
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
            Some(RivenAction::Edit),
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

    fn perform(&mut self, action: RivenAction, source: &DataSource) -> Outcome<RivenAction> {
        match action {
            RivenAction::Add => Outcome::ask(self.create_prompt(), Self::created),
            RivenAction::Create(input) => {
                let weapon = self
                    .weapons
                    .iter()
                    .find(|weapon| weapon.wfm_url == input.wfm_weapon_url)
                    .map_or(input.wfm_weapon_url.clone(), |weapon| weapon.name.clone());
                let notice = format!("Added {weapon} {} to stock.", input.mod_name);
                Outcome::run(source.stock_riven_create(input), notice)
            }
            RivenAction::Sell(id) => match self.riven(id) {
                Some(riven) => Outcome::ask(
                    prompts::sold(&riven.title(), None, riven.list_price),
                    move |values| {
                        Some(RivenAction::Sold {
                            id,
                            price: values.number("price")?,
                        })
                    },
                ),
                None => Outcome::Done,
            },
            RivenAction::SellAtList(id) => {
                match self.riven(id).and_then(|riven| riven.list_price) {
                    Some(price) => self.perform(RivenAction::Sold { id, price }, source),
                    None => Outcome::Done,
                }
            }
            RivenAction::Sold { id, price } => match self.riven(id) {
                Some(riven) => Outcome::run(
                    source.stock_riven_sell(&riven, price),
                    format!("Sold {} for {} platinum.", riven.title(), number(price)),
                ),
                None => Outcome::Done,
            },
            RivenAction::Edit(ids) => match self.edit_prompt(&ids) {
                Some(spec) => Outcome::ask(spec, move |values| {
                    Some(RivenAction::Save {
                        ids: ids.clone(),
                        update: StockRivenUpdate {
                            bought: values.number("bought"),
                            re_rolls: values.number("re_rolls"),
                            mastery_rank: values.number("mastery_rank"),
                            min_price: limit_field(values, "min_price", ids.len() == 1),
                            ..StockRivenUpdate::default()
                        },
                    })
                }),
                None => Outcome::Done,
            },
            RivenAction::StepMinPrice { id, step } => {
                match self
                    .riven(id)
                    .and_then(|riven| Some((riven.id, stepped(riven.properties.min_price, step)?)))
                {
                    Some((id, min_price)) => Outcome::run_quietly(source.stock_riven_update(
                        id,
                        StockRivenUpdate {
                            min_price: Some(min_price),
                            ..StockRivenUpdate::default()
                        },
                    )),
                    None => Outcome::Done,
                }
            }
            RivenAction::Filter(id) => match self.riven(id) {
                Some(riven) => Outcome::ask(Self::filter_prompt(&riven), move |values| {
                    Some(RivenAction::SaveFilter {
                        id,
                        filter: Self::filter_saved(&riven, values),
                    })
                }),
                None => Outcome::Done,
            },
            RivenAction::SaveFilter { id, filter } => {
                Outcome::run_quietly(source.stock_riven_update(
                    id as i64,
                    StockRivenUpdate {
                        filter: Some(filter),
                        ..StockRivenUpdate::default()
                    },
                ))
            }
            RivenAction::Save { ids: rows, update } => match rows.as_slice() {
                [id] => Outcome::run_quietly(source.stock_riven_update(*id as i64, update)),
                _ => Outcome::run_quietly(source.stock_riven_update_multiple(ids(&rows), update)),
            },
            RivenAction::ToggleHidden(id) => match self.riven(id) {
                Some(riven) => Outcome::run_quietly(source.stock_riven_update(
                    riven.id,
                    StockRivenUpdate {
                        is_hidden: Some(!riven.is_hidden),
                        ..StockRivenUpdate::default()
                    },
                )),
                None => Outcome::Done,
            },
            RivenAction::Delete(rows) if rows.is_empty() => Outcome::Done,
            RivenAction::Delete(rows) => {
                Outcome::ask(prompts::delete(rows.len(), "riven", "rivens"), move |_| {
                    Some(RivenAction::Deleted(rows.clone()))
                })
            }
            RivenAction::Deleted(rows) => match rows.as_slice() {
                [id] => Outcome::run_quietly(source.stock_riven_delete(*id as i64)),
                _ => Outcome::run_quietly(source.stock_riven_delete_multiple(ids(&rows))),
            },
            RivenAction::Details(id) => Outcome::ShowPanel(id),
            RivenAction::CopyName(id) => match self.riven(id) {
                Some(riven) => Outcome::Copy {
                    what: riven.title(),
                    text: riven.title(),
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
            RivenAction::Message(rows) => {
                let items = self.message_items(&rows);
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
            RivenAction::Export => {
                let source = source.clone();
                Outcome::export("stock-rivens.json", move |path| {
                    Box::pin(source.export_stock_riven_json(&every_row(), path))
                })
            }
        }
    }

    fn row_id(row: &StockRiven) -> u64 {
        row.id as u64
    }

    fn compare(key: &str, a: &StockRiven, b: &StockRiven) -> Ordering {
        match key {
            "mastery" => a.mastery_rank.cmp(&b.mastery_rank),
            "re_rolls" => a.re_rolls.cmp(&b.re_rolls),
            "bought" => a.bought.cmp(&b.bought),
            "list_price" => a.list_price.cmp(&b.list_price),
            "min_price" => a.properties.min_price.cmp(&b.properties.min_price),
            "status" => a.status.label().cmp(b.status.label()),
            _ => a.title().cmp(&b.title()),
        }
    }

    fn cell(row: &StockRiven, key: &str, emit: &Emit<RivenAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let id = row.id as u64;
        match key {
            "riven" => {
                let mut flags = Vec::new();
                if row.is_hidden {
                    flags.push((IconName::EyeOff, "Hidden from Warframe Market"));
                }
                cells::name(row.title(), &flags, cx)
            }
            "attributes" => attributes_cell(id, &row.attributes, cx),
            "mastery" => cells::amount(Some(row.mastery_rank), "", cx),
            "re_rolls" => cells::amount(Some(row.re_rolls), "", cx),
            "bought" => cells::amount(Some(row.bought), "", cx),
            "list_price" => list_price_cell(row, cx),
            "min_price" => limit_cell(
                id,
                row.properties.min_price,
                |step| RivenAction::StepMinPrice { id, step },
                emit,
                cx,
            ),
            "status" => cells::status(theme.status_color(row.status), row.status.label(), cx),
            _ => {
                let menu_row = row.clone();
                let filter = if row.filter.enabled {
                    "Auction filter: on"
                } else {
                    "Auction filter: off"
                };
                cells::actions()
                    .child(
                        div().pr(px(5.)).child(
                            row_button(("sell", id), "Sell", cx)
                                .on_click(emit.on_click(RivenAction::Sell(id))),
                        ),
                    )
                    .child({
                        let button = IconButton::new(("filter", id), IconName::Funnel, filter)
                            .in_row()
                            .on_click(emit.on_click(RivenAction::Filter(id)));
                        if row.filter.enabled {
                            button
                                .tone(theme.colors.accent)
                                .hover_tone(theme.colors.accent)
                        } else {
                            button
                        }
                    })
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
                    .child(overflow_button(id, move || Self::row_menu(&menu_row), emit))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &StockRiven, key: &str) -> Option<String> {
        match key {
            "riven" => Some(row.title()),
            "attributes" => Some(attributes_text(&row.attributes).join(", ")),
            "mastery" => Some(number(row.mastery_rank)),
            "re_rolls" => Some(number(row.re_rolls)),
            "bought" => Some(number(row.bought)),
            "list_price" => row.list_price.map(|price| {
                if row.properties.is_predicted {
                    format!("{}, predicted", number(price))
                } else {
                    number(price)
                }
            }),
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
                "Edit the auction filter",
                IconName::Funnel,
                RivenAction::Filter(id),
            ),
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                RivenAction::Details(id),
            ),
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
        PanelModel {
            title: row.title(),
            subtitle: format!(
                "{}, {} polarity",
                words(&row.weapon_type),
                words(&row.polarity)
            ),
            status: Some((
                row.status.label().to_owned(),
                theme.status_color(row.status),
            )),
            note: Some(row.status.detail().to_owned()),
            history: row.price_history.iter().map(|check| check.price).collect(),
            lines: Some(("Attributes", attribute_lines(&row.attributes))),
            facts: vec![
                Fact::new("Bought", number(row.bought)),
                Fact::amount("List price", row.list_price, "Not listed"),
                Fact::new(
                    "Price source",
                    if row.properties.is_predicted {
                        "Price model"
                    } else {
                        "Live auctions"
                    },
                ),
                match profit {
                    Some(profit) => {
                        Fact::new("Profit", number(profit)).tone(Tone::of_profit(profit))
                    }
                    None => Fact::new("Profit", "Not listed").tone(Tone::Muted),
                },
                Fact::amount("Minimum price", row.properties.min_price, "Not set"),
                Fact::new("Auction filter", filter_summary(&row.filter)),
                Fact::new("Mastery rank", number(row.mastery_rank)),
                Fact::new("Rank", format!("{} of {MAX_RANK}", row.rank.unwrap_or(0))),
                Fact::new("Re-rolls", number(row.re_rolls)),
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
                    "Edit the auction filter",
                    IconName::Funnel,
                    RivenAction::Filter(id),
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
