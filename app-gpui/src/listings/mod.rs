//! The listings screen: the live scraper's stock, with the Item tab built.

mod inspector;
mod table;

use std::{collections::BTreeMap, rc::Rc};

use gpui::{
    App, Context, Div, Entity, IntoElement, ParentElement, Pixels, Render, Stateful, Styled,
    Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    Sizable, Size,
    button::ButtonVariants,
    input::{Input, InputEvent, InputState},
    table::{DataTable, TableState},
};
use gpui_kit_assets::IconName;

use crate::{
    data::DataSource,
    model::{FinancialReport, StockItemListParams, StockStatus},
    theme::{Navigation, Theme, Variant},
    ui::{IconButton, chip, divider, dot, icon, label_text, number, percent, text_button},
};
use table::StockTable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListingTab {
    Items,
    Rivens,
    WishList,
    Syndicates,
}

impl ListingTab {
    const ALL: [Self; 4] = [Self::Items, Self::Rivens, Self::WishList, Self::Syndicates];

    const fn label(self) -> &'static str {
        match self {
            Self::Items => "Items",
            Self::Rivens => "Rivens",
            Self::WishList => "Wish list",
            Self::Syndicates => "Syndicates",
        }
    }
}

pub struct ListingsView {
    source: Rc<dyn DataSource>,
    tab: ListingTab,
    status_filter: Option<StockStatus>,
    search: Entity<InputState>,
    table: Entity<TableState<StockTable>>,
    status_counts: BTreeMap<StockStatus, usize>,
    report: FinancialReport,
    total_items: usize,
    total_units: i64,
    _subscriptions: Vec<Subscription>,
}

impl ListingsView {
    pub fn new(source: Rc<dyn DataSource>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search items"));
        let page = source.get_stock_item_pagination(&StockItemListParams::default());
        let total_items = page.total as usize;
        let total_units = page.results.iter().map(|item| item.owned).sum();
        let width = table_width(window, cx);
        let has_inspector = Theme::global(cx).layout.inspector_width.is_some();
        let table = cx.new(|cx| {
            let mut state = TableState::new(StockTable::new(page.results, width, cx), window, cx)
                .col_movable(false)
                .col_selectable(false);
            if has_inspector {
                state.set_selected_row(0, cx);
            }
            state
        });
        let subscriptions = vec![
            cx.subscribe(&search, |view, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    view.reload(cx);
                }
            }),
            cx.observe(&table, |_, _, cx| cx.notify()),
        ];
        Self {
            status_counts: source.get_stock_item_status_counts(),
            report: source.get_stock_item_financial_report(),
            source,
            tab: ListingTab::Items,
            status_filter: None,
            search,
            table,
            total_items,
            total_units,
            _subscriptions: subscriptions,
        }
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().to_string();
        let params = StockItemListParams {
            query: (!query.trim().is_empty()).then_some(query),
            status: self.status_filter,
            ..StockItemListParams::default()
        };
        let rows = self.source.get_stock_item_pagination(&params).results;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_rows(rows);
            cx.notify();
        });
        cx.notify();
    }

    fn set_status_filter(&mut self, status: Option<StockStatus>, cx: &mut Context<Self>) {
        self.status_filter = status;
        self.reload(cx);
    }

    fn sync_table_width(&mut self, window: &Window, cx: &mut Context<Self>) {
        let width = table_width(window, cx);
        if self.table.read(cx).delegate().available_width() != width {
            self.table.update(cx, |table, cx| {
                table.delegate_mut().set_available_width(width, cx);
                table.refresh(cx);
            });
        }
    }

    fn tabs(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let segmented = theme.variant == Variant::A;
        let tabs = ListingTab::ALL.into_iter().map(|tab| {
            let active = tab == self.tab;
            let label = div().child(tab.label());
            let count = (tab == ListingTab::Items).then(|| {
                div()
                    .text_size(theme.type_scale.caption)
                    .text_color(if active {
                        colors.text_muted
                    } else {
                        colors.text_faint
                    })
                    .child(number(self.total_items as i64))
            });
            let base = div()
                .id(tab.label())
                .flex()
                .items_center()
                .gap(px(6.))
                .cursor_pointer()
                .font_weight(theme.type_scale.label_weight)
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.tab = tab;
                    cx.notify();
                }))
                .child(label)
                .children(count);
            if segmented {
                base.h(theme.metrics.control_height - px(4.))
                    .px(px(12.))
                    .rounded(theme.metrics.radius - px(1.))
                    .text_size(theme.type_scale.body)
                    .map(|tab| {
                        if active {
                            tab.bg(colors.border_strong).text_color(colors.text)
                        } else {
                            tab.text_color(colors.text_muted)
                                .hover(|tab| tab.text_color(colors.text))
                        }
                    })
            } else {
                base.h_full()
                    .text_size(theme.type_scale.emphasis)
                    .border_b_2()
                    .map(|tab| {
                        if active {
                            tab.border_color(colors.accent).text_color(colors.text)
                        } else {
                            tab.border_color(colors.accent.opacity(0.))
                                .text_color(colors.text_muted)
                                .hover(|tab| tab.text_color(colors.text))
                        }
                    })
            }
        });
        if segmented {
            div()
                .flex()
                .items_center()
                .gap(px(2.))
                .p(px(2.))
                .rounded(theme.metrics.radius + px(1.))
                .bg(colors.raised)
                .children(tabs)
        } else {
            div()
                .flex()
                .items_center()
                .h_full()
                .gap(theme.metrics.gutter)
                .children(tabs)
        }
    }

    /// The search box. gpui-component fixes its input's height per size
    /// preset, so the frame is drawn here at the theme's control height.
    fn search_field(&self, width: Pixels, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        div()
            .w(width)
            .h(theme.metrics.control_height)
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(2.))
            .pl(px(9.))
            .pr(px(4.))
            .rounded(theme.metrics.radius)
            .border_1()
            .border_color(colors.border_strong)
            .bg(colors.raised)
            .child(icon(
                IconName::Search,
                theme.metrics.icon,
                colors.text_muted,
            ))
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(&self.search)
                        .small()
                        .appearance(false)
                        .cleanable(true),
                ),
            )
    }

    fn add_item_button(&self, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::global(cx);
        text_button("add-item", "Add item", cx)
            .primary()
            .icon(gpui_component::Icon::new(IconName::Plus).size(theme.metrics.icon))
    }

    fn status_chips(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let all = chip("status-all", self.status_filter.is_none(), cx)
            .child("All")
            .child(chip_count(self.total_items, &theme))
            .on_click(cx.listener(|view, _, _, cx| view.set_status_filter(None, cx)));
        let chips = self
            .status_counts
            .iter()
            .map(|(status, count)| {
                let status = *status;
                let selected = self.status_filter == Some(status);
                chip(status.as_str(), selected, cx)
                    .child(dot(theme.status_color(status), px(6.)))
                    .child(status.label())
                    .child(chip_count(*count, &theme))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let next = (view.status_filter != Some(status)).then_some(status);
                        view.set_status_filter(next, cx);
                    }))
            })
            .collect::<Vec<_>>();
        div()
            .id("status-chips")
            .flex()
            .items_center()
            .gap(px(6.))
            .min_w_0()
            .overflow_x_scroll()
            .child(all)
            .children(chips)
    }

    fn bulk_actions(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let selected = self.table.read(cx).delegate().selected_count();
        let none = selected == 0;
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(2.))
            .when(!none, |row| {
                row.child(
                    div()
                        .pr(px(8.))
                        .text_size(theme.type_scale.caption + px(1.))
                        .text_color(colors.text_muted)
                        .child(format!("{selected} selected")),
                )
            })
            .child(
                IconButton::new("export", IconName::Download, "Export the list as JSON").build(cx),
            )
            .child(
                IconButton::new(
                    "edit-selected",
                    IconName::SquarePen,
                    "Edit the selected items",
                )
                .disabled(none)
                .build(cx),
            )
            .child(
                IconButton::new(
                    "wts-selected",
                    IconName::MessageSquare,
                    "Write a trade chat message for the selected items",
                )
                .disabled(none)
                .build(cx),
            )
            .child(
                IconButton::new(
                    "delete-selected",
                    IconName::Trash,
                    "Delete the selected items",
                )
                .hover_tone(colors.negative)
                .disabled(none)
                .build(cx),
            )
    }

    fn totals(&self) -> [(&'static str, String, String); 4] {
        let report = &self.report;
        [
            (
                "Bought",
                number(report.expenses),
                format!("{} units in stock", number(self.total_units)),
            ),
            (
                "Listed",
                number(report.revenue),
                "at current list prices".to_owned(),
            ),
            (
                "Profit",
                number(report.total_profit),
                format!("{} return on cost", percent(report.roi)),
            ),
            (
                "Items",
                number(self.total_items as i64),
                format!(
                    "{} live on Warframe Market",
                    self.status_counts
                        .get(&StockStatus::Live)
                        .copied()
                        .unwrap_or(0)
                ),
            ),
        ]
    }

    fn total_color(&self, label: &str, theme: &Theme) -> gpui::Hsla {
        match label {
            "Profit" if self.report.total_profit < 0 => theme.colors.negative,
            "Profit" => theme.colors.positive,
            _ => theme.colors.text,
        }
    }

    fn summary_panels(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        div()
            .flex()
            .gap(px(12.))
            .children(self.totals().map(|(label, value, note)| {
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .px(px(14.))
                    .py(px(10.))
                    .rounded(theme.metrics.radius_large)
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.chrome)
                    .child(
                        div()
                            .text_size(theme.type_scale.caption + px(1.))
                            .text_color(colors.text_muted)
                            .child(label),
                    )
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(theme.type_scale.figure)
                                    .font_weight(theme.type_scale.title_weight)
                                    .text_color(self.total_color(label, theme))
                                    .child(value),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme.type_scale.caption)
                                    .text_color(colors.text_faint)
                                    .child(note),
                            ),
                    )
            }))
    }

    fn summary_figures(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let totals = self.totals();
        let last = totals.len() - 1;
        div()
            .flex()
            .items_start()
            .children(
                totals
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value, note))| {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .pr(px(40.))
                            .when(index > 0, |figure| {
                                figure.pl(px(40.)).border_l_1().border_color(colors.border)
                            })
                            .when(index == last, |figure| figure.pr(px(0.)))
                            .child(
                                div()
                                    .text_size(theme.type_scale.caption)
                                    .text_color(colors.text_muted)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .text_size(theme.type_scale.figure)
                                    .font_weight(theme.type_scale.title_weight)
                                    .text_color(self.total_color(label, theme))
                                    .child(value),
                            )
                            .child(
                                div()
                                    .text_size(theme.type_scale.caption)
                                    .text_color(colors.text_faint)
                                    .child(note),
                            )
                    }),
            )
    }

    fn summary_footer(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let report = &self.report;
        let figure = |label: &'static str, value: String, color: gpui::Hsla| {
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(theme.type_scale.caption)
                        .font_weight(theme.type_scale.label_weight)
                        .text_color(colors.text_faint)
                        .child(label_text(theme, label)),
                )
                .child(
                    div()
                        .font_family(theme.type_scale.numeric_family.clone())
                        .font_weight(theme.type_scale.label_weight)
                        .text_color(color)
                        .child(value),
                )
        };
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .h(theme.metrics.row_height + px(3.))
            .px(theme.metrics.gutter)
            .border_t_1()
            .border_color(colors.border_strong)
            .bg(colors.chrome)
            .text_size(theme.type_scale.body)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(20.))
                    .child(figure(
                        "Items",
                        number(self.total_items as i64),
                        colors.text,
                    ))
                    .child(figure("Units", number(self.total_units), colors.text)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(20.))
                    .child(figure("Bought", number(report.expenses), colors.text))
                    .child(figure("Listed", number(report.revenue), colors.text))
                    .child(figure(
                        "Profit",
                        format!("{} ({})", number(report.total_profit), percent(report.roi)),
                        self.total_color("Profit", theme),
                    )),
            )
    }

    fn table_element(&self, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::global(cx);
        div().flex_1().min_h_0().w_full().child(
            DataTable::new(&self.table)
                .with_size(Size::Size(theme.metrics.row_height))
                .header_row_height(theme.metrics.header_height)
                .stripe(theme.layout.striped)
                .bordered(false)
                .accessibility_label("Stock items"),
        )
    }
}

fn chip_count(count: usize, theme: &Theme) -> Div {
    div()
        .font_family(theme.type_scale.numeric_family.clone())
        .text_color(theme.colors.text_faint)
        .child(number(count as i64))
}

/// The width left for the table once the navigation and the item panel have
/// taken theirs.
fn table_width(window: &Window, cx: &App) -> Pixels {
    let layout = Theme::global(cx).layout;
    let navigation = match layout.navigation {
        Navigation::Sidebar { width } | Navigation::Rail { width } => width,
        Navigation::TopBar => px(0.),
    };
    window.viewport_size().width - navigation - layout.inspector_width.unwrap_or(px(0.))
}

impl Render for ListingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_table_width(window, cx);
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let gutter = theme.metrics.gutter;
        let page = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.surface)
            .text_size(theme.type_scale.body)
            .text_color(colors.text);

        match theme.variant {
            Variant::A => page
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .px(gutter)
                        .pt(px(14.))
                        .child(self.tabs(cx))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(self.bulk_actions(cx))
                                .child(divider(cx))
                                .child(self.search_field(px(260.), cx))
                                .child(self.add_item_button(cx)),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .px(gutter)
                        .pt(px(14.))
                        .child(self.summary_panels(cx)),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .px(gutter)
                        .py(px(12.))
                        .child(self.status_chips(cx)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .border_t_1()
                        .border_color(colors.border)
                        .child(self.table_element(cx)),
                ),
            Variant::B => {
                let selected = self
                    .table
                    .read(cx)
                    .selected_row()
                    .and_then(|row| self.table.read(cx).delegate().row(row).cloned());
                page.child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .gap(px(12.))
                        .h(px(38.))
                        .px(gutter)
                        .child(self.tabs(cx))
                        .child(div().flex_1())
                        .child(self.bulk_actions(cx))
                        .child(divider(cx))
                        .child(self.search_field(px(240.), cx))
                        .child(self.add_item_button(cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .h(px(34.))
                        .px(gutter)
                        .border_t_1()
                        .border_b_1()
                        .border_color(colors.border)
                        .child(self.status_chips(cx)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(self.table_element(cx))
                                .child(self.summary_footer(cx)),
                        )
                        .child(inspector::panel(selected.as_ref(), cx)),
                )
            }
            Variant::C => page
                .child(
                    div()
                        .flex_shrink_0()
                        .px(gutter)
                        .pt(px(4.))
                        .pb(px(18.))
                        .child(self.summary_figures(cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_between()
                        .h(px(48.))
                        .px(gutter)
                        .border_b_1()
                        .border_color(colors.border)
                        .child(self.tabs(cx))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(self.search_field(px(300.), cx))
                                .child(self.add_item_button(cx)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_between()
                        .gap(px(16.))
                        .px(gutter)
                        .py(px(12.))
                        .child(self.status_chips(cx))
                        .child(self.bulk_actions(cx)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .border_t_1()
                        .border_color(colors.border)
                        .child(self.table_element(cx)),
                ),
        }
    }
}
