//! The Orders screen: the user's orders and riven auctions on Warframe Market.

mod auctions;
mod list;

use gpui::{App, AppContext, Entity, Window};

use crate::{
    data::Backend,
    ui::{page::ListPage, screen::TabbedScreen},
};

pub fn screen(backend: &Entity<Backend>, window: &mut Window, cx: &mut App) -> Entity<TabbedScreen> {
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("orders-tab");
        let page = cx.new(|cx| ListPage::new(list::Orders::default(), backend, window, cx));
        screen.add("orders", "Orders", page, cx);
        let page = cx.new(|cx| ListPage::new(auctions::Auctions::default(), backend, window, cx));
        screen.add("auctions", "Auctions", page, cx);
        screen
    })
}

/// A Warframe Market id as a number, for tables that key rows by `u64`.
fn row_key(id: &str) -> u64 {
    id.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}
