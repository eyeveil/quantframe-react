//! The listings screen: what the live scraper sells and buys, in four tabs.

mod items;
mod rivens;
mod syndicates;
mod wish_list;

use std::{collections::BTreeMap, future::Future};

use gpui::{App, AppContext, Entity, Window};
use gpui_kit_assets::IconName;

use gpui::AnyElement;

use crate::{
    data::{Backend, DataResult},
    model::{FinancialReport, Paginated, StockItemListParams, StockStatus, SubType},
    theme::Theme,
    ui::{
        cells,
        form::FormValues,
        grid::{ChipGroup, ChipSpec, Emit, Load, Query, ToolbarItem},
        page::ListPage,
        screen::TabbedScreen,
    },
};
pub use rivens::{attribute_lines, attributes_cell, attributes_text, words};

pub fn screen(backend: &Entity<Backend>, window: &mut Window, cx: &mut App) -> Entity<TabbedScreen> {
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("listing-tab");
        let page = cx.new(|cx| ListPage::new(items::Items::default(), backend, window, cx));
        screen.add("items", "Items", page, cx);
        let page = cx.new(|cx| ListPage::new(rivens::Rivens::default(), backend, window, cx));
        screen.add("rivens", "Rivens", page, cx);
        let page = cx.new(|cx| ListPage::new(wish_list::WishList::default(), backend, window, cx));
        screen.add("wish-list", "Wish list", page, cx);
        let page =
            cx.new(|cx| ListPage::new(syndicates::Syndicates::default(), backend, window, cx));
        screen.add("syndicates", "Syndicates", page, cx);
        screen
    })
}

/// What one load of a live scraper list fetches.
pub struct StockData<T> {
    pub rows: Vec<T>,
    pub counts: BTreeMap<StockStatus, usize>,
    pub report: FinancialReport,
}

impl<T> Default for StockData<T> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            counts: BTreeMap::new(),
            report: FinancialReport::default(),
        }
    }
}

/// The query for every row of a list. The table filters and sorts them.
fn every_row() -> StockItemListParams {
    StockItemListParams {
        limit: -1,
        ..StockItemListParams::default()
    }
}

/// Joins the three backend calls of a live scraper list into one load. The
/// calls already run on the backend runtime, so the order of the awaits
/// doesn't make them wait on each other.
fn load_stock<T: Send + 'static>(
    rows: impl Future<Output = DataResult<Paginated<T>>> + Send + 'static,
    counts: impl Future<Output = DataResult<BTreeMap<StockStatus, usize>>> + Send + 'static,
    report: impl Future<Output = DataResult<FinancialReport>> + Send + 'static,
) -> Load<StockData<T>> {
    Box::pin(async move {
        Ok(StockData {
            rows: rows.await?.results,
            counts: counts.await?,
            report: report.await?,
        })
    })
}

/// The step of the buttons beside a price limit, in platinum.
const LIMIT_STEP: i64 = 5;

/// A price limit moved by `step`, or `None` when it would drop below zero.
/// An unset limit counts as 0, and a limit of 0 clears it.
fn stepped(limit: Option<i64>, step: i64) -> Option<i64> {
    let next = limit.unwrap_or(0) + step;
    (next >= 0).then_some(next)
}

/// A price limit cell with buttons that lower and raise it by `LIMIT_STEP`.
fn limit_cell<A: Clone + 'static>(
    id: u64,
    value: Option<i64>,
    step: impl Fn(i64) -> A,
    emit: &Emit<A>,
    cx: &App,
) -> AnyElement {
    cells::stepped_price(id, value, LIMIT_STEP, step(-LIMIT_STEP), step(LIMIT_STEP), emit, cx)
}

/// A price limit read from an edit dialog. Emptying the field of one row
/// clears the limit, and an empty field of a bulk edit keeps each row's.
fn limit_field(values: &FormValues, key: &str, single: bool) -> Option<i64> {
    match values.number(key) {
        Some(value) => Some(value),
        None if single => Some(0),
        None => None,
    }
}

fn market_url(slug: &str) -> String {
    format!("https://warframe.market/items/{slug}")
}

/// A typed item name as the slug Warframe Market uses for it.
fn slug(name: &str) -> String {
    name.trim().to_lowercase().replace(' ', "_")
}

fn rank_label(sub_type: &Option<SubType>) -> Option<String> {
    sub_type.as_ref().and_then(SubType::label)
}

fn ids(rows: &[u64]) -> Vec<i64> {
    rows.iter().map(|id| *id as i64).collect()
}

/// One chip per status that has rows, coloured like the status dots.
fn status_chips(counts: &BTreeMap<StockStatus, usize>, cx: &App) -> ChipGroup {
    let theme = Theme::global(cx);
    ChipGroup {
        id: "status",
        chips: counts
            .iter()
            .filter(|(_, count)| **count > 0)
            .map(|(status, count)| ChipSpec {
                key: status.as_str().to_owned(),
                label: status.label().to_owned(),
                count: *count,
                color: Some(theme.status_color(*status)),
                tooltip: status.detail().to_owned(),
            })
            .collect(),
    }
}

/// The status the selected status chip stands for.
fn status_filter(query: &Query) -> Option<StockStatus> {
    query.filter("status").map(StockStatus::parse)
}

/// The toolbar buttons every listings tab has: export, and edit, message
/// and delete for the checked rows.
fn bulk_toolbar<A>(
    checked: &[u64],
    noun: &str,
    export: A,
    edit: Option<fn(Vec<u64>) -> A>,
    message: Option<fn(Vec<u64>) -> A>,
    delete: fn(Vec<u64>) -> A,
) -> Vec<ToolbarItem<A>> {
    let selection = (!checked.is_empty()).then(|| checked.to_vec());
    let mut toolbar = vec![ToolbarItem {
        id: "export",
        icon: IconName::Download,
        tooltip: "Export the list as JSON".to_owned(),
        action: Some(export),
        danger: false,
    }];
    if let Some(edit) = edit {
        toolbar.push(ToolbarItem {
            id: "edit-selected",
            icon: IconName::SquarePen,
            tooltip: format!("Edit the selected {noun}"),
            action: selection.clone().map(edit),
            danger: false,
        });
    }
    if let Some(message) = message {
        toolbar.push(ToolbarItem {
            id: "message-selected",
            icon: IconName::MessageSquare,
            tooltip: format!("Write a trade chat message for the selected {noun}"),
            action: selection.clone().map(message),
            danger: false,
        });
    }
    toolbar.push(ToolbarItem {
        id: "delete-selected",
        icon: IconName::Trash,
        tooltip: format!("Delete the selected {noun}"),
        action: selection.map(delete),
        danger: true,
    });
    toolbar
}
