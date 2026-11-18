//! The Orders screen: the user's orders and riven auctions on Warframe Market.

mod auctions;
pub mod model;
mod list;

use gpui::{App, AppContext, Entity, Window};

use crate::{
    model::User,
    preview,
    ui::{
        page::{ListPage, PreviewState},
        screen::TabbedScreen,
    },
};

const REFRESH_NOT_CONNECTED: &str = "Refreshing from Warframe Market isn't connected yet.";

pub fn screen(
    user: &User,
    state: PreviewState,
    window: &mut Window,
    cx: &mut App,
) -> Entity<TabbedScreen> {
    let load = state.load_state();
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("orders-tab");
        let page = cx.new(|cx| {
            ListPage::new(
                list::Orders::new(state.rows(preview::orders()), user.order_limit),
                load.clone(),
                window,
                cx,
            )
        });
        screen.add("orders", "Orders", page, cx);
        let page = cx.new(|cx| {
            ListPage::new(
                auctions::Auctions::new(state.rows(preview::auctions()), user.auctions_limit),
                load.clone(),
                window,
                cx,
            )
        });
        screen.add("auctions", "Auctions", page, cx);
        screen
    })
}
