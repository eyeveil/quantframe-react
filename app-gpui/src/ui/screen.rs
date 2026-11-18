//! A screen made of tabs, each a list page. It draws the tabs and the active
//! page's toolbar on one line, and the page below.

use gpui::{
    Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, Styled, Subscription,
    Window, div, px,
};

use super::{
    Segment,
    grid::Catalog,
    page::{FocusSearch, ListPage, PAGE_CONTEXT, PageHandle, ToggleItemPanel, toggle_item_panel},
    segmented,
};
use crate::theme::Theme;

struct Tab {
    key: &'static str,
    label: &'static str,
    page: Box<dyn PageHandle>,
}

pub struct TabbedScreen {
    id: &'static str,
    tabs: Vec<Tab>,
    active: usize,
    _subscriptions: Vec<Subscription>,
}

impl TabbedScreen {
    pub fn new(id: &'static str) -> Self {
        Self {
            id,
            tabs: Vec::new(),
            active: 0,
            _subscriptions: Vec::new(),
        }
    }

    pub fn add<C: Catalog>(
        &mut self,
        key: &'static str,
        label: &'static str,
        page: Entity<ListPage<C>>,
        cx: &mut Context<Self>,
    ) {
        self._subscriptions
            .push(cx.observe(&page, |_, _, cx| cx.notify()));
        self.tabs.push(Tab {
            key,
            label,
            page: Box::new(page),
        });
    }

    /// Adds a tab that is not a list page, such as a chart.
    pub fn add_view<V: 'static>(
        &mut self,
        key: &'static str,
        label: &'static str,
        view: Entity<V>,
        cx: &mut Context<Self>,
    ) where
        Entity<V>: PageHandle,
    {
        self._subscriptions
            .push(cx.observe(&view, |_, _, cx| cx.notify()));
        self.tabs.push(Tab {
            key,
            label,
            page: Box::new(view),
        });
    }

    /// The active tab's page.
    pub fn page(&self) -> &dyn PageHandle {
        self.tabs[self.active].page.as_ref()
    }

    /// Opens the tab with this key. An unknown key keeps the current tab.
    pub fn open_tab(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.tabs.iter().position(|tab| tab.key == key) {
            self.select(index, window, cx);
        }
    }

    fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active = index;
        self.page().focus(window, cx);
        cx.notify();
    }
}

impl Render for TabbedScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let width = window.viewport_size().width - theme.metrics.sidebar_width;
        let segments = self
            .tabs
            .iter()
            .map(|tab| Segment {
                label: tab.label,
                count: tab.page.count(cx),
            })
            .collect();
        let screen = cx.entity().downgrade();
        let tabs = segmented(
            self.id,
            segments,
            self.active,
            move |index, window, cx| {
                screen
                    .update(cx, |screen, cx| screen.select(index, window, cx))
                    .ok();
            },
            cx,
        );
        let toolbar = self.page().toolbar(window, cx);
        let body = self.page().body(width, window, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .key_context(PAGE_CONTEXT)
            .on_action(cx.listener(|screen, _: &FocusSearch, window, cx| {
                screen.page().focus_search(window, cx);
            }))
            .on_action(|_: &ToggleItemPanel, _, cx| toggle_item_panel(cx))
            .bg(theme.colors.surface)
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text)
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .px(theme.metrics.gutter)
                    .pt(px(14.))
                    .child(tabs)
                    .child(toolbar),
            )
            .child(div().flex_1().min_h_0().child(body))
    }
}
