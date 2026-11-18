use std::{cmp::Ordering, marker::PhantomData, path::PathBuf};

use gpui::{AnyElement, App, IntoElement, ParentElement};
use gpui_kit_assets::IconName;

use crate::{
    data::{DataKind, DataSource},
    kit,
    model::{ItemPrice, Paginated, PriceLookupParams, RivenPrice, SyndicateItemPrice},
    ui::{
        cells,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ColumnSpec, Emit, Export, Load, MenuEntry, Outcome,
            PrimaryAction, Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number, numeric,
        panel::{Fact, PanelAction, PanelModel},
    },
};

const MAX_RANGE_DAYS: i64 = 90;
const ROW_CHOICES: [i64; 4] = [100, 250, 500, 1000];

pub enum Cell {
    Text(String),
    Number(f64),
}

pub trait PriceKind: 'static {
    type Row: Clone + Send + 'static;

    const NOUN: &'static str;
    const SEARCH_PLACEHOLDER: &'static str;
    const FILE_NAME: &'static str;
    const DATED: bool;
    const COLUMNS: &'static [(&'static str, &'static str, f32)];

    fn fetch(source: &DataSource, params: &PriceLookupParams) -> Load<Paginated<Self::Row>>;
    fn export(source: &DataSource, params: &PriceLookupParams, path: PathBuf) -> Export;
    fn name(row: &Self::Row) -> String;
    fn value(row: &Self::Row, key: &str) -> Cell;
    fn url(row: &Self::Row) -> Option<String>;
}

#[derive(Clone, Debug)]
pub enum PriceAction {
    AskQuery,
    SetQuery(PriceLookupParams),
    Reload,
    Export,
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
}

pub struct Prices<K: PriceKind> {
    params: PriceLookupParams,
    rows: Vec<(u64, K::Row)>,
    total: i64,
    kind: PhantomData<K>,
}

impl<K: PriceKind> Default for Prices<K> {
    fn default() -> Self {
        let mut params =
            PriceLookupParams::new(kit::date_days_ago(30), kit::date_days_ago(1));
        params.limit = ROW_CHOICES[0];
        Self {
            params,
            rows: Vec::new(),
            total: 0,
            kind: PhantomData,
        }
    }
}

impl<K: PriceKind> Prices<K> {
    fn row(&self, key: u64) -> Option<&K::Row> {
        self.rows
            .iter()
            .find(|(id, _)| *id == key)
            .map(|(_, row)| row)
    }

    fn query_form(&self) -> FormSpec {
        let rows = ROW_CHOICES
            .iter()
            .map(|rows| format!("{rows} rows"))
            .collect();
        let selected = ROW_CHOICES
            .iter()
            .position(|rows| *rows == self.params.limit);
        let spec = FormSpec::new(format!("Look up {}", K::NOUN), "Look up")
            .description(if K::DATED {
                format!(
                    "Quantframe's server searches by name and returns statistics for the date range. A range covers up to {MAX_RANGE_DAYS} days and ends before today."
                )
            } else {
                "Quantframe's server searches by name. The search box above the table filters the rows already loaded.".to_owned()
            })
            .field(
                Field::text(
                    "query",
                    "Name contains",
                    self.params.query.as_deref().unwrap_or_default(),
                    "Any name",
                )
                .optional(),
            );
        let spec = if K::DATED {
            spec.field(
                Field::text("from", "From", &self.params.from_date, "YYYY-MM-DD")
                    .hint("Year, month and day.")
                    .half(),
            )
            .field(Field::text("to", "To", &self.params.to_date, "YYYY-MM-DD").half())
        } else {
            spec
        };
        spec.field(Field::choice("rows", "Rows to load", rows, selected))
    }
}

fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |index: usize, width: usize| {
        parts
            .get(index)
            .filter(|part| part.len() == width)
            .and_then(|part| part.parse::<u32>().ok())
    };
    parts.len() == 3
        && number(0, 4).is_some()
        && number(1, 2).is_some_and(|month| (1..=12).contains(&month))
        && number(2, 2).is_some_and(|day| (1..=31).contains(&day))
}

impl<K: PriceKind> Catalog for Prices<K> {
    type Row = (u64, K::Row);
    type Action = PriceAction;
    type Data = (Vec<K::Row>, i64);

    const NOUN: &'static str = K::NOUN;
    const SEARCH_PLACEHOLDER: &'static str = K::SEARCH_PLACEHOLDER;
    const EMPTY_HINT: &'static str =
        "Quantframe's server returned no rows for this search. Change the name or the dates.";
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let page = K::fetch(source, &self.params);
        Box::pin(async move {
            let page = page.await?;
            Ok((page.results, page.total))
        })
    }

    fn apply(&mut self, (rows, total): Self::Data) {
        self.total = total;
        self.rows = rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| (index as u64, row))
            .collect();
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        let mut columns = vec![ColumnSpec::new("name", "Name", 240.).flex().sortable()];
        columns.extend(K::COLUMNS.iter().enumerate().map(|(index, (key, title, width))| {
            let column = ColumnSpec::new(key, title, *width).sortable();
            let column = if is_text_column(key) {
                column
            } else {
                column.end()
            };
            if index > 2 {
                column.droppable((K::COLUMNS.len() - index) as u8)
            } else {
                column
            }
        }));
        columns.push(ColumnSpec::new("actions", "", 48.));
        columns
    }

    fn rows(&self, query: &Query) -> Vec<Self::Row> {
        self.rows
            .iter()
            .filter(|(_, row)| query.matches(&K::name(row)))
            .cloned()
            .collect()
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        Vec::new()
    }

    fn summary(&self) -> Vec<Stat> {
        let search = match self.params.query.as_deref() {
            Some(query) if !query.trim().is_empty() => format!("named like \"{}\"", query.trim()),
            _ => "any name".to_owned(),
        };
        let mut stats = vec![Stat {
            label: "Rows loaded",
            value: number(self.rows.len() as i64),
            note: format!("of {} on the server, {search}", number(self.total)),
            tone: Tone::Plain,
        }];
        if K::DATED {
            stats.push(Stat {
                label: "Date range",
                value: format!("{} to {}", self.params.from_date, self.params.to_date),
                note: format!("up to {MAX_RANGE_DAYS} days"),
                tone: Tone::Plain,
            });
        }
        stats
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<PriceAction>> {
        vec![
            ToolbarItem {
                id: "reload",
                icon: IconName::RefreshCw,
                tooltip: format!("Load the {} again", K::NOUN),
                action: Some(PriceAction::Reload),
                danger: false,
            },
            ToolbarItem {
                id: "export",
                icon: IconName::Download,
                tooltip: format!("Export the {} as JSON", K::NOUN),
                action: Some(PriceAction::Export),
                danger: false,
            },
        ]
    }

    fn primary(&self) -> Option<PrimaryAction<PriceAction>> {
        Some(PrimaryAction {
            label: if K::DATED {
                "Name and dates"
            } else {
                "Name and rows"
            },
            icon: IconName::Funnel,
            action: PriceAction::AskQuery,
        })
    }

    fn perform(&mut self, action: PriceAction, source: &DataSource) -> Outcome<PriceAction> {
        match action {
            PriceAction::AskQuery => {
                let current = self.params.clone();
                Outcome::ask(self.query_form(), move |values| {
                    let mut params = current.clone();
                    let query = values.text("query");
                    params.query = (!query.is_empty()).then_some(query);
                    if K::DATED {
                        params.from_date = values.text("from");
                        params.to_date = values.text("to");
                    }
                    if let Some(rows) = values
                        .choice("rows")
                        .and_then(|rows| rows.split(' ').next()?.parse().ok())
                    {
                        params.limit = rows;
                    }
                    Some(PriceAction::SetQuery(params))
                })
            }
            PriceAction::SetQuery(params) => {
                if K::DATED {
                    if !is_date(&params.from_date) || !is_date(&params.to_date) {
                        return Outcome::Notice(
                            "Write both dates as year, month and day, such as 2026-11-01."
                                .to_owned(),
                        );
                    }
                    if params.from_date > params.to_date {
                        return Outcome::Notice(
                            "The first date has to be on or before the last date.".to_owned(),
                        );
                    }
                }
                self.params = params;
                Outcome::run_quietly(std::future::ready(Ok(())))
            }
            PriceAction::Reload => Outcome::run_quietly(std::future::ready(Ok(()))),
            PriceAction::Export => {
                let source = source.clone();
                let params = self.params.clone();
                Outcome::export(K::FILE_NAME, move |path| K::export(&source, &params, path))
            }
            PriceAction::Details(key) => Outcome::ShowPanel(key),
            PriceAction::CopyName(key) => match self.row(key) {
                Some(row) => Outcome::Copy {
                    what: K::name(row),
                    text: K::name(row),
                },
                None => Outcome::Done,
            },
            PriceAction::OpenMarket(key) => match self.row(key).and_then(K::url) {
                Some(url) => Outcome::OpenUrl(url),
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &Self::Row) -> u64 {
        row.0
    }

    fn compare(key: &str, a: &Self::Row, b: &Self::Row) -> Ordering {
        if key == "name" {
            return K::name(&a.1).cmp(&K::name(&b.1));
        }
        match (K::value(&a.1, key), K::value(&b.1, key)) {
            (Cell::Number(a), Cell::Number(b)) => a.total_cmp(&b),
            (Cell::Text(a), Cell::Text(b)) => a.cmp(&b),
            (Cell::Number(_), Cell::Text(_)) => Ordering::Less,
            (Cell::Text(_), Cell::Number(_)) => Ordering::Greater,
        }
    }

    fn cell(row: &Self::Row, key: &str, emit: &Emit<PriceAction>, cx: &App) -> AnyElement {
        match key {
            "name" => cells::name(K::name(&row.1), &[], cx),
            "actions" => {
                let menu_row = row.clone();
                cells::actions()
                    .child(overflow_button(
                        row.0,
                        move || Self::row_menu(&menu_row),
                        emit,
                    ))
                    .into_any_element()
            }
            _ => match K::value(&row.1, key) {
                Cell::Text(text) => cells::text(text),
                Cell::Number(value) => numeric(kit::decimal(value)).into_any_element(),
            },
        }
    }

    fn cell_text(row: &Self::Row, key: &str) -> Option<String> {
        match key {
            "name" => Some(K::name(&row.1)),
            "actions" => None,
            _ => Some(match K::value(&row.1, key) {
                Cell::Text(text) => text,
                Cell::Number(value) => kit::decimal(value),
            }),
        }
    }

    fn row_menu(row: &Self::Row) -> Vec<MenuEntry<PriceAction>> {
        let id = row.0;
        let mut menu = vec![
            MenuEntry::new("Show details", IconName::PanelRight, PriceAction::Details(id)),
            MenuEntry::new("Copy name", IconName::Copy, PriceAction::CopyName(id)),
        ];
        if K::url(&row.1).is_some() {
            menu.push(MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                PriceAction::OpenMarket(id),
            ));
        }
        menu
    }

    fn panel(row: &Self::Row, _: &App) -> PanelModel<PriceAction> {
        let id = row.0;
        PanelModel {
            title: K::name(&row.1),
            subtitle: format!("One of the {}", K::NOUN),
            status: None,
            note: None,
            history: Vec::new(),
            lines: None,
            facts: K::COLUMNS
                .iter()
                .map(|(key, title, _)| {
                    Fact::new(
                        title,
                        match K::value(&row.1, key) {
                            Cell::Text(text) => text,
                            Cell::Number(value) => kit::decimal(value),
                        },
                    )
                })
                .collect(),
            primary: vec![
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    PriceAction::OpenMarket(id),
                )
                .enabled(K::url(&row.1).is_some()),
            ],
            secondary: vec![PanelAction::new(
                "Copy name",
                IconName::Copy,
                PriceAction::CopyName(id),
            )],
        }
    }
}

fn is_text_column(key: &str) -> bool {
    matches!(key, "syndicate" | "rank")
}

pub struct ItemPrices;

impl PriceKind for ItemPrices {
    type Row = ItemPrice;

    const NOUN: &'static str = "item prices";
    const SEARCH_PLACEHOLDER: &'static str = "Filter the loaded items";
    const FILE_NAME: &'static str = "item-prices.json";
    const DATED: bool = true;
    const COLUMNS: &'static [(&'static str, &'static str, f32)] = &[
        ("rank", "Rank", 110.),
        ("volume", "Volume", 96.),
        ("avg_price", "Average price", 120.),
        ("closed_price", "Closing price", 120.),
        ("supply", "Supply", 96.),
        ("demand", "Demand", 96.),
    ];

    fn fetch(source: &DataSource, params: &PriceLookupParams) -> Load<Paginated<ItemPrice>> {
        Box::pin(source.item_prices_lookup(params))
    }

    fn export(source: &DataSource, params: &PriceLookupParams, path: PathBuf) -> Export {
        Box::pin(source.export_item_price_data(params, path))
    }

    fn name(row: &ItemPrice) -> String {
        row.name.clone()
    }

    fn value(row: &ItemPrice, key: &str) -> Cell {
        match key {
            "rank" => Cell::Text(
                row.sub_type
                    .as_ref()
                    .and_then(|sub| sub.label())
                    .unwrap_or_default(),
            ),
            "volume" => Cell::Number(row.volume),
            "avg_price" => Cell::Number(row.avg_price),
            "closed_price" => Cell::Number(row.closed_price),
            "supply" => Cell::Number(row.supply),
            _ => Cell::Number(row.demand),
        }
    }

    fn url(row: &ItemPrice) -> Option<String> {
        (!row.wfm_url.is_empty()).then(|| format!("https://warframe.market/items/{}", row.wfm_url))
    }
}

pub struct RivenPrices;

impl PriceKind for RivenPrices {
    type Row = RivenPrice;

    const NOUN: &'static str = "riven prices";
    const SEARCH_PLACEHOLDER: &'static str = "Filter the loaded rivens";
    const FILE_NAME: &'static str = "riven-prices.json";
    const DATED: bool = true;
    const COLUMNS: &'static [(&'static str, &'static str, f32)] = &[
        ("volume", "Volume", 96.),
        ("min_price", "Lowest price", 120.),
        ("max_price", "Highest price", 120.),
        ("avg_price", "Average price", 120.),
    ];

    fn fetch(source: &DataSource, params: &PriceLookupParams) -> Load<Paginated<RivenPrice>> {
        Box::pin(source.riven_prices_lookup(params))
    }

    fn export(source: &DataSource, params: &PriceLookupParams, path: PathBuf) -> Export {
        Box::pin(source.export_riven_price_data(params, path))
    }

    fn name(row: &RivenPrice) -> String {
        row.name.clone()
    }

    fn value(row: &RivenPrice, key: &str) -> Cell {
        Cell::Number(match key {
            "volume" => row.volume,
            "min_price" => row.min_price,
            "max_price" => row.max_price,
            _ => row.avg_price,
        })
    }

    fn url(row: &RivenPrice) -> Option<String> {
        (!row.wfm_url.is_empty()).then(|| {
            format!(
                "https://warframe.market/auctions/search?type=riven&weapon_url_name={}",
                row.wfm_url
            )
        })
    }
}

pub struct SyndicatePrices;

impl PriceKind for SyndicatePrices {
    type Row = SyndicateItemPrice;

    const NOUN: &'static str = "syndicate prices";
    const SEARCH_PLACEHOLDER: &'static str = "Filter the loaded syndicate items";
    const FILE_NAME: &'static str = "syndicate-prices.json";
    const DATED: bool = false;
    const COLUMNS: &'static [(&'static str, &'static str, f32)] = &[
        ("syndicate", "Syndicate", 180.),
        ("standing_cost", "Standing cost", 120.),
        ("volume", "Volume", 96.),
        ("min_price", "Lowest price", 120.),
        ("max_price", "Highest price", 120.),
    ];

    fn fetch(
        source: &DataSource,
        params: &PriceLookupParams,
    ) -> Load<Paginated<SyndicateItemPrice>> {
        Box::pin(source.syndicate_item_prices_lookup(params))
    }

    fn export(source: &DataSource, params: &PriceLookupParams, path: PathBuf) -> Export {
        Box::pin(source.export_syndicate_item_price_data(params, path))
    }

    fn name(row: &SyndicateItemPrice) -> String {
        row.name.clone()
    }

    fn value(row: &SyndicateItemPrice, key: &str) -> Cell {
        match key {
            "syndicate" => Cell::Text(row.syndicate.clone()),
            "standing_cost" => Cell::Number(row.standing_cost as f64),
            "volume" => Cell::Number(row.volume),
            "min_price" => Cell::Number(row.min_price),
            _ => Cell::Number(row.max_price),
        }
    }

    fn url(_: &SyndicateItemPrice) -> Option<String> {
        None
    }
}
