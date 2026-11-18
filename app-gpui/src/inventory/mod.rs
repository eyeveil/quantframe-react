//! The Inventory screen: the rivens and syndicate standing Quantframe reads
//! from the game, through AlecaFrame, the public profile or a file.

use std::cmp::Ordering;

use gpui::{AnyElement, App, AppContext, Entity, IntoElement, ParentElement, Window};
use gpui_component::{Disableable, button::ButtonVariants};
use gpui_kit_assets::IconName;

use crate::{
    data::{Backend, DataKind, DataSource},
    model::{InventoryRiven, InventorySyndicate, ListParams},
    theme::Theme,
    ui::{
        cells,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ChipSpec, ColumnSpec, Emit, Load, MenuEntry, Outcome,
            PrimaryAction, Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number,
        page::ListPage,
        panel::{Fact, PanelAction, PanelModel},
        row_button,
        screen::TabbedScreen,
    },
};

const SOURCE_HINT: &str =
    "Pick where Quantframe reads your inventory under Settings, Inventory source, then refresh.";

pub fn screen(backend: &Entity<Backend>, window: &mut Window, cx: &mut App) -> Entity<TabbedScreen> {
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("inventory-tab");
        let page = cx.new(|cx| ListPage::new(Rivens::default(), backend, window, cx));
        screen.add("rivens", "Rivens", page, cx);
        let page = cx.new(|cx| ListPage::new(Syndicates::default(), backend, window, cx));
        screen.add("syndicates", "Syndicates", page, cx);
        screen
    })
}

/// A row id from a uuid or a name, for tables that key rows by `u64`.
fn row_key(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn refresh_item<A>(action: A) -> ToolbarItem<A> {
    ToolbarItem {
        id: "refresh",
        icon: IconName::RefreshCw,
        tooltip: "Read the inventory again from its source".to_owned(),
        action: Some(action),
        danger: false,
    }
}

#[derive(Clone, Debug)]
pub enum RivenAction {
    Refresh,
    Add(u64),
    Added { key: u64, bought: i64 },
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
}

#[derive(Default)]
pub struct Rivens {
    rows: Vec<InventoryRiven>,
}

fn riven_title(row: &InventoryRiven) -> String {
    if row.mod_name.is_empty() {
        row.name.clone()
    } else {
        format!("{} {}", row.name, row.mod_name)
    }
}

/// The backend's riven states as the trader reads them.
fn state_label(state: &str) -> &'static str {
    match state {
        "veiled" => "Veiled",
        "unveiled" => "Unveiled",
        "pre-veiled" => "Not yet unveiled",
        _ => "Unknown",
    }
}

/// The React app adds only the rivens the backend calls `veiled` to stock.
fn can_add(row: &InventoryRiven) -> bool {
    row.riven_type == "veiled" && !row.is_in_stock
}

impl Rivens {
    fn row(&self, key: u64) -> Option<&InventoryRiven> {
        self.rows.iter().find(|row| row_key(&row.uuid) == key)
    }
}

impl Catalog for Rivens {
    type Row = InventoryRiven;
    type Action = RivenAction;
    type Data = Vec<InventoryRiven>;

    const NOUN: &'static str = "inventory rivens";
    const SEARCH_PLACEHOLDER: &'static str = "Search rivens";
    const EMPTY_HINT: &'static str = SOURCE_HINT;
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[DataKind::StockRivens, DataKind::Settings];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        Box::pin(source.wf_inventory_list_rivens())
    }

    fn apply(&mut self, data: Self::Data) {
        self.rows = data;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("riven", "Riven", 260.).flex().sortable(),
            ColumnSpec::new("state", "State", 150.).sortable(),
            ColumnSpec::new("mastery", "MR", 64.).end().sortable(),
            ColumnSpec::new("rank", "Rank", 64.).end().sortable(),
            ColumnSpec::new("re_rolls", "Rolls", 70.).end().sortable(),
            ColumnSpec::new("polarity", "Polarity", 110.)
                .sortable()
                .droppable(1),
            ColumnSpec::new("stock", "Stock", 120.).sortable(),
            ColumnSpec::new("actions", "", 150.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<InventoryRiven> {
        let state = query.filter("state");
        let stock = query.filter("stock");
        self.rows
            .iter()
            .filter(|row| state.is_none_or(|key| row.riven_type == key))
            .filter(|row| stock.is_none_or(|key| row.is_in_stock == (key == "in")))
            .filter(|row| query.matches(&riven_title(row)))
            .cloned()
            .collect()
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        let count = |matches: &dyn Fn(&InventoryRiven) -> bool| {
            self.rows.iter().filter(|row| matches(row)).count()
        };
        let chip = |key: &str, label: &str, count: usize| ChipSpec {
            key: key.to_owned(),
            label: label.to_owned(),
            count,
            color: None,
            tooltip: String::new(),
        };
        vec![
            ChipGroup {
                id: "state",
                chips: ["veiled", "unveiled"]
                    .into_iter()
                    .map(|state| {
                        chip(
                            state,
                            state_label(state),
                            count(&|row| row.riven_type == state),
                        )
                    })
                    .collect(),
            },
            ChipGroup {
                id: "stock",
                chips: vec![
                    chip("in", "In stock", count(&|row| row.is_in_stock)),
                    chip("out", "Not in stock", count(&|row| !row.is_in_stock)),
                ],
            },
        ]
    }

    fn summary(&self) -> Vec<Stat> {
        let in_stock = self.rows.iter().filter(|row| row.is_in_stock).count();
        vec![
            Stat {
                label: "Rivens in the game",
                value: number(self.rows.len() as i64),
                note: "from the inventory source".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Already in stock",
                value: number(in_stock as i64),
                note: format!(
                    "{} not in stock",
                    number((self.rows.len() - in_stock) as i64)
                ),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<RivenAction>> {
        vec![refresh_item(RivenAction::Refresh)]
    }

    fn primary(&self) -> Option<PrimaryAction<RivenAction>> {
        None
    }

    fn perform(&mut self, action: RivenAction, source: &DataSource) -> Outcome<RivenAction> {
        match action {
            RivenAction::Refresh => {
                Outcome::run(source.wf_inventory_update(), "Read the inventory again.")
            }
            RivenAction::Add(key) => match self.row(key) {
                Some(row) => Outcome::ask(
                    FormSpec::new(format!("Add {} to stock", riven_title(row)), "Add to stock")
                        .description("The live scraper prices the riven once it is in stock.")
                        .field(
                            Field::platinum("bought", "Bought", Some(0))
                                .hint("What you paid for the riven."),
                        ),
                    move |values| {
                        Some(RivenAction::Added {
                            key,
                            bought: values.number("bought")?,
                        })
                    },
                ),
                None => Outcome::Done,
            },
            RivenAction::Added { key, bought } => match self.row(key) {
                Some(row) => Outcome::run(
                    source.wf_inventory_stock_riven(row.uuid.clone(), bought),
                    format!("Added {} to stock.", riven_title(row)),
                ),
                None => Outcome::Done,
            },
            RivenAction::Details(key) => Outcome::ShowPanel(key),
            RivenAction::CopyName(key) => match self.row(key) {
                Some(row) => Outcome::Copy {
                    what: riven_title(row),
                    text: riven_title(row),
                },
                None => Outcome::Done,
            },
            RivenAction::OpenMarket(key) => match self.row(key) {
                Some(row) if !row.wfm_url.is_empty() => Outcome::OpenUrl(format!(
                    "https://warframe.market/auctions/search?type=riven&weapon_url_name={}",
                    row.wfm_url
                )),
                _ => Outcome::Done,
            },
        }
    }

    fn row_id(row: &InventoryRiven) -> u64 {
        row_key(&row.uuid)
    }

    fn compare(key: &str, a: &InventoryRiven, b: &InventoryRiven) -> Ordering {
        match key {
            "state" => a.riven_type.cmp(&b.riven_type),
            "mastery" => a.mastery_rank.cmp(&b.mastery_rank),
            "rank" => a.rank.cmp(&b.rank),
            "re_rolls" => a.re_rolls.cmp(&b.re_rolls),
            "polarity" => a.polarity.cmp(&b.polarity),
            "stock" => a.is_in_stock.cmp(&b.is_in_stock),
            _ => riven_title(a).cmp(&riven_title(b)),
        }
    }

    fn cell(row: &InventoryRiven, key: &str, emit: &Emit<RivenAction>, cx: &App) -> AnyElement {
        let colors = Theme::global(cx).colors;
        let id = row_key(&row.uuid);
        match key {
            "riven" => cells::name(riven_title(row), &[], cx),
            "state" => cells::text(state_label(&row.riven_type)),
            "mastery" => cells::amount(Some(row.mastery_rank), "", cx),
            "rank" => cells::amount(Some(row.rank), "", cx),
            "re_rolls" => cells::amount(Some(row.re_rolls), "", cx),
            "polarity" => cells::text(crate::listings::words(&row.polarity)),
            "stock" => {
                if row.is_in_stock {
                    cells::status(colors.positive, "In stock", cx)
                } else {
                    cells::status(colors.text_faint, "Not in stock", cx)
                }
            }
            _ => {
                let menu_row = row.clone();
                let add = row_button(("add", id), "Add to stock", cx)
                    .primary()
                    .disabled(!can_add(row));
                cells::actions()
                    .child(if can_add(row) {
                        add.on_click(emit.on_click(RivenAction::Add(id)))
                    } else {
                        add
                    })
                    .child(overflow_button(
                        id,
                        move || Self::row_menu(&menu_row),
                        emit,
                    ))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &InventoryRiven, key: &str) -> Option<String> {
        match key {
            "riven" => Some(riven_title(row)),
            "state" => Some(state_label(&row.riven_type).to_owned()),
            "mastery" => Some(number(row.mastery_rank)),
            "rank" => Some(number(row.rank)),
            "re_rolls" => Some(number(row.re_rolls)),
            "polarity" => Some(row.polarity.clone()),
            "stock" => Some(if row.is_in_stock { "In stock" } else { "Not in stock" }.to_owned()),
            _ => None,
        }
    }

    fn row_menu(row: &InventoryRiven) -> Vec<MenuEntry<RivenAction>> {
        let id = row_key(&row.uuid);
        let mut menu = Vec::new();
        if can_add(row) {
            menu.push(MenuEntry::new(
                "Add to stock",
                IconName::PackagePlus,
                RivenAction::Add(id),
            ));
        }
        menu.extend([
            MenuEntry::new("Show details", IconName::PanelRight, RivenAction::Details(id)),
            MenuEntry::new("Copy name", IconName::Copy, RivenAction::CopyName(id)),
            MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                RivenAction::OpenMarket(id),
            ),
        ]);
        menu
    }

    fn panel(row: &InventoryRiven, cx: &App) -> PanelModel<RivenAction> {
        let colors = Theme::global(cx).colors;
        let id = row_key(&row.uuid);
        PanelModel {
            title: riven_title(row),
            subtitle: state_label(&row.riven_type).to_owned(),
            status: Some(if row.is_in_stock {
                ("In stock".to_owned(), colors.positive)
            } else {
                ("Not in stock".to_owned(), colors.text_faint)
            }),
            note: None,
            history: Vec::new(),
            lines: None,
            facts: vec![
                Fact::new("Mastery rank", number(row.mastery_rank)),
                Fact::new("Rank", number(row.rank)),
                Fact::new("Re-rolls", number(row.re_rolls)),
                Fact::new("Polarity", crate::listings::words(&row.polarity)),
            ],
            primary: vec![
                PanelAction::new("Add to stock", IconName::PackagePlus, RivenAction::Add(id))
                    .enabled(can_add(row)),
            ],
            secondary: vec![
                PanelAction::new("Copy name", IconName::Copy, RivenAction::CopyName(id)),
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    RivenAction::OpenMarket(id),
                )
                .enabled(!row.wfm_url.is_empty()),
            ],
        }
    }
}

#[derive(Clone, Debug)]
pub enum SyndicateAction {
    Refresh,
    Details(u64),
    CopyName(u64),
}

#[derive(Default)]
pub struct Syndicates {
    rows: Vec<InventorySyndicate>,
}

impl Catalog for Syndicates {
    type Row = InventorySyndicate;
    type Action = SyndicateAction;
    type Data = Vec<InventorySyndicate>;

    const NOUN: &'static str = "syndicate standings";
    const SEARCH_PLACEHOLDER: &'static str = "Search syndicates";
    const EMPTY_HINT: &'static str = SOURCE_HINT;
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[DataKind::Settings];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let rows = source.wf_inventory_get_syndicates(&ListParams {
            limit: -1,
            ..ListParams::default()
        });
        Box::pin(async move { Ok(rows.await?.results) })
    }

    fn apply(&mut self, data: Self::Data) {
        self.rows = data;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("name", "Syndicate", 280.).flex().sortable(),
            ColumnSpec::new("standing", "Standing in this title", 190.)
                .end()
                .sortable(),
            ColumnSpec::new("total", "Total standing", 160.).end().sortable(),
            ColumnSpec::new("actions", "", 48.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<InventorySyndicate> {
        self.rows
            .iter()
            .filter(|row| query.matches(&row.name))
            .cloned()
            .collect()
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        Vec::new()
    }

    fn summary(&self) -> Vec<Stat> {
        vec![
            Stat {
                label: "Syndicates",
                value: number(self.rows.len() as i64),
                note: "with standing".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Total standing",
                value: number(self.rows.iter().map(|row| row.total_standing).sum()),
                note: "across every syndicate".to_owned(),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<SyndicateAction>> {
        vec![refresh_item(SyndicateAction::Refresh)]
    }

    fn primary(&self) -> Option<PrimaryAction<SyndicateAction>> {
        None
    }

    fn perform(
        &mut self,
        action: SyndicateAction,
        source: &DataSource,
    ) -> Outcome<SyndicateAction> {
        let name = |key: u64| {
            self.rows
                .iter()
                .find(|row| row_key(&row.unique_name) == key)
                .map(|row| row.name.clone())
        };
        match action {
            SyndicateAction::Refresh => {
                Outcome::run(source.wf_inventory_update(), "Read the inventory again.")
            }
            SyndicateAction::Details(key) => Outcome::ShowPanel(key),
            SyndicateAction::CopyName(key) => match name(key) {
                Some(name) => Outcome::Copy {
                    what: name.clone(),
                    text: name,
                },
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &InventorySyndicate) -> u64 {
        row_key(&row.unique_name)
    }

    fn compare(key: &str, a: &InventorySyndicate, b: &InventorySyndicate) -> Ordering {
        match key {
            "standing" => a.standing.cmp(&b.standing),
            "total" => a.total_standing.cmp(&b.total_standing),
            _ => a.name.cmp(&b.name),
        }
    }

    fn cell(
        row: &InventorySyndicate,
        key: &str,
        emit: &Emit<SyndicateAction>,
        cx: &App,
    ) -> AnyElement {
        match key {
            "name" => cells::name(row.name.clone(), &[], cx),
            "standing" => cells::amount(Some(row.standing), "", cx),
            "total" => cells::amount(Some(row.total_standing), "", cx),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(overflow_button(
                        row_key(&row.unique_name),
                        move || Self::row_menu(&menu_row),
                        emit,
                    ))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &InventorySyndicate, key: &str) -> Option<String> {
        match key {
            "name" => Some(row.name.clone()),
            "standing" => Some(number(row.standing)),
            "total" => Some(number(row.total_standing)),
            _ => None,
        }
    }

    fn row_menu(row: &InventorySyndicate) -> Vec<MenuEntry<SyndicateAction>> {
        let id = row_key(&row.unique_name);
        vec![
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                SyndicateAction::Details(id),
            ),
            MenuEntry::new("Copy name", IconName::Copy, SyndicateAction::CopyName(id)),
        ]
    }

    fn panel(row: &InventorySyndicate, _: &App) -> PanelModel<SyndicateAction> {
        PanelModel {
            title: row.name.clone(),
            subtitle: "Syndicate standing".to_owned(),
            status: None,
            note: None,
            history: Vec::new(),
            lines: None,
            facts: vec![
                Fact::new("Standing in this title", number(row.standing)),
                Fact::new("Total standing", number(row.total_standing)),
            ],
            primary: Vec::new(),
            secondary: vec![PanelAction::new(
                "Copy name",
                IconName::Copy,
                SyndicateAction::CopyName(row_key(&row.unique_name)),
            )],
        }
    }
}
