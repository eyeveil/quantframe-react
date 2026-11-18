//! The listings screen: what the live scraper sells and buys, in four tabs.

mod items;
pub mod model;
mod rivens;
mod syndicates;
mod wish_list;

use std::rc::Rc;

use gpui::{App, AppContext, Entity, Window};
use gpui_kit_assets::IconName;

use crate::{
    data::DataSource,
    model::{FinancialReport, StockItemListParams, StockStatus},
    preview,
    theme::Theme,
    ui::{
        grid::{ChipGroup, ChipSpec, Query, ToolbarItem},
        page::{ListPage, PreviewState},
        screen::TabbedScreen,
    },
};
use model::{Record, Store};
pub use rivens::attributes_cell;

/// The step of the buttons beside a price limit, in platinum.
const MIN_PRICE_STEP: i64 = 5;

const EXPORT_NOT_CONNECTED: &str = "Export isn't connected to the backend yet.";

pub fn screen(
    source: &Rc<dyn DataSource>,
    state: PreviewState,
    window: &mut Window,
    cx: &mut App,
) -> Entity<TabbedScreen> {
    let stock = source
        .get_stock_item_pagination(&StockItemListParams {
            limit: 1000,
            ..StockItemListParams::default()
        })
        .results;
    let report = match state {
        PreviewState::Empty => FinancialReport::default(),
        _ => source.get_stock_item_financial_report(),
    };
    let load = state.load_state();
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("listing-tab");
        let page = cx.new(|cx| {
            ListPage::new(
                items::Items::new(state.rows(stock), report),
                load.clone(),
                window,
                cx,
            )
        });
        screen.add("items", "Items", page, cx);
        let page = cx.new(|cx| {
            ListPage::new(
                rivens::Rivens::new(state.rows(preview::stock_rivens())),
                load.clone(),
                window,
                cx,
            )
        });
        screen.add("rivens", "Rivens", page, cx);
        let page = cx.new(|cx| {
            ListPage::new(
                wish_list::WishList::new(state.rows(preview::wish_list())),
                load.clone(),
                window,
                cx,
            )
        });
        screen.add("wish-list", "Wish list", page, cx);
        let page = cx.new(|cx| {
            ListPage::new(
                syndicates::Syndicates::new(state.rows(preview::syndicate_items())),
                load.clone(),
                window,
                cx,
            )
        });
        screen.add("syndicates", "Syndicates", page, cx);
        screen
    })
}

fn market_url(section: &str, slug: &str) -> String {
    format!("https://warframe.market/{section}/{slug}")
}

/// One chip per status that has rows, coloured like the status dots.
fn status_chips<T: Record>(store: &Store<T>, cx: &App) -> ChipGroup {
    let theme = Theme::global(cx);
    ChipGroup {
        id: "status",
        chips: store
            .status_counts()
            .into_iter()
            .map(|(status, count)| ChipSpec {
                key: status.as_str().to_owned(),
                label: status.label().to_owned(),
                count,
                color: Some(theme.status_color(status)),
                tooltip: status.detail().to_owned(),
            })
            .collect(),
    }
}

/// The status the selected status chip stands for.
fn status_filter<T: Record>(store: &Store<T>, query: &Query) -> Option<StockStatus> {
    let key = query.filter("status")?;
    store
        .status_counts()
        .into_keys()
        .find(|status| status.as_str() == key)
}

/// The toolbar buttons every listings tab has: export, and edit, message
/// and delete for the checked rows.
fn bulk_toolbar<A>(
    checked: &[u64],
    noun: &str,
    export: A,
    edit: fn(Vec<u64>) -> A,
    message: Option<fn(Vec<u64>) -> A>,
    delete: fn(Vec<u64>) -> A,
) -> Vec<ToolbarItem<A>> {
    let selection = (!checked.is_empty()).then(|| checked.to_vec());
    let mut toolbar = vec![
        ToolbarItem {
            id: "export",
            icon: IconName::Download,
            tooltip: "Export the list as JSON".to_owned(),
            action: Some(export),
            danger: false,
        },
        ToolbarItem {
            id: "edit-selected",
            icon: IconName::SquarePen,
            tooltip: format!("Edit the selected {noun}"),
            action: selection.clone().map(edit),
            danger: false,
        },
    ];
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
