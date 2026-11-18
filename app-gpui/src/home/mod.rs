//! The dashboard: profit totals, three profit charts, the latest
//! transactions and the totals per item category.

use gpui::{
    App, Context, Div, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    div, prelude::*, px,
};
use gpui_kit_assets::IconName;

use crate::{
    chart::{self, Series},
    data::{Backend, BackendEvent, DataKind, DataSource},
    kit::{self, Loaded},
    model::{
        CategorySummary, DashboardSummary, FinancialReport, PeriodSummary, Transaction,
        TransactionType,
    },
    theme::{Theme, tabular_figures},
    ui::{day_and_time, dot, grid::Tone, number, placeholder},
};

const CHART_HEIGHT: f32 = 150.;
/// The room a chart card gives its plot: the bars, the labels under them
/// and a legend.
const PLOT_HEIGHT: f32 = CHART_HEIGHT + 34.;
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub struct HomeView {
    source: DataSource,
    summary: Loaded<DashboardSummary>,
    _subscription: Subscription,
}

impl HomeView {
    pub fn new(backend: &Entity<Backend>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe(backend, |view, _, event: &BackendEvent, cx| {
            if matches!(
                event,
                BackendEvent::Changed(DataKind::Transactions | DataKind::Settings)
            ) {
                view.load(cx);
            }
        });
        let mut view = Self {
            source: backend.read(cx).source.clone(),
            summary: Loaded::Loading,
            _subscription: subscription,
        };
        view.load(cx);
        view
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let summary = self.source.dashboard_summary();
        cx.spawn(async move |view, cx| {
            let summary = Loaded::from_result(summary.await);
            view.update(cx, |view, cx| {
                view.summary = summary;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

/// Sales, purchases and margin of a report, as the line under a total.
fn trades_line(report: &FinancialReport) -> String {
    format!(
        "{} sales, {} purchases, {}% margin",
        number(report.sale_count),
        number(report.purchases_count),
        kit::decimal(report.profit_margin),
    )
}

fn totals(summary: &DashboardSummary, cx: &App) -> Div {
    let best = &summary.best_seller;
    div()
        .flex()
        .flex_shrink_0()
        .gap(px(12.))
        .child(kit::stat(
            "Total profit",
            number(summary.total.total_profit),
            trades_line(&summary.total),
            Tone::of_profit(summary.total.total_profit),
            cx,
        ))
        .child(kit::stat(
            "Profit today",
            number(summary.today.summary.total_profit),
            trades_line(&summary.today.summary),
            Tone::of_profit(summary.today.summary.total_profit),
            cx,
        ))
        .child(kit::stat(
            "Best seller",
            if best.item_name.is_empty() {
                "None yet".to_owned()
            } else {
                best.item_name.clone()
            },
            format!(
                "{} profit, {}",
                number(best.report.total_profit),
                trades_line(&best.report)
            ),
            Tone::Plain,
            cx,
        ))
}

/// Revenue, expenses, trades and average profit under a chart.
fn report_strip(report: &FinancialReport, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let cell = |label: &'static str, value: String| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(1.))
            .child(
                div()
                    .text_size(theme.type_scale.caption)
                    .text_color(colors.text_faint)
                    .child(label),
            )
            .child(
                div()
                    .truncate()
                    .font_features(tabular_figures())
                    .text_color(colors.text)
                    .child(value),
            )
    };
    div()
        .flex()
        .gap(px(10.))
        .pt(px(10.))
        .border_t_1()
        .border_color(colors.border)
        .child(cell("Revenue", number(report.revenue)))
        .child(cell("Expenses", number(report.expenses)))
        .child(cell("Trades", number(report.total_transactions)))
        .child(cell("Average profit", kit::decimal(report.average_profit)))
}

fn chart_card(
    title: String,
    profit: i64,
    report: &FinancialReport,
    plot: Div,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    kit::card(cx)
        .flex_1()
        .gap(px(10.))
        .p(px(14.))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap(px(8.))
                .child(kit::heading(title, cx))
                .child(
                    div()
                        .flex_shrink_0()
                        .font_features(tabular_figures())
                        .text_color(Tone::of_profit(profit).color(theme))
                        .child(format!("{} platinum", number(profit))),
                ),
        )
        .child(div().h(px(PLOT_HEIGHT)).child(plot))
        .child(report_strip(report, cx))
}

fn period_plot(id: &'static str, period: &PeriodSummary, color: gpui::Hsla, cx: &App) -> Div {
    if period.chart.values.iter().all(|value| *value == 0) {
        return chart::empty("No trades in this period.", PLOT_HEIGHT, cx);
    }
    chart::bars(
        id,
        &period.chart.labels,
        &[Series::new("Profit", color, period.chart.values.clone())],
        CHART_HEIGHT,
        cx,
    )
}

fn charts(summary: &DashboardSummary, cx: &App) -> Div {
    let colors = Theme::global(cx).colors;
    let months: Vec<String> = MONTHS.iter().map(|month| (*month).to_owned()).collect();
    let year = chart::bars(
        "chart-year",
        &months,
        &[
            Series::new(
                "This year",
                colors.accent,
                summary.present_year.chart.values.clone(),
            ),
            Series::new(
                "Last year",
                colors.text_faint,
                summary.last_year.chart.values.clone(),
            ),
        ],
        CHART_HEIGHT - 26.,
        cx,
    );
    let days = summary.recent_days.chart.labels.len().saturating_sub(1).max(1);
    div()
        .flex()
        .flex_shrink_0()
        .gap(px(12.))
        .child(chart_card(
            "Profit by month".to_owned(),
            summary.present_year.summary.total_profit,
            &summary.present_year.summary,
            year,
            cx,
        ))
        .child(chart_card(
            "Profit today by hour".to_owned(),
            summary.today.summary.total_profit,
            &summary.today.summary,
            period_plot("chart-today", &summary.today, colors.positive, cx),
            cx,
        ))
        .child(chart_card(
            format!("Profit over the last {days} days"),
            summary.recent_days.summary.total_profit,
            &summary.recent_days.summary,
            period_plot("chart-recent", &summary.recent_days, colors.info, cx),
            cx,
        ))
}

fn transaction_row(transaction: &Transaction, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let sale = transaction.transaction_type == TransactionType::Sale;
    let (verb, tone) = if sale {
        ("Sold to", colors.positive)
    } else {
        ("Bought from", colors.info)
    };
    let name = match transaction.sub_type.as_ref().and_then(|sub| sub.label()) {
        Some(sub) => format!("{}, {}", transaction.item_name, sub.to_lowercase()),
        None => transaction.item_name.clone(),
    };
    let party = if transaction.user_name.is_empty() {
        if sale { "Sold" } else { "Bought" }.to_owned()
    } else {
        format!("{verb} {}", transaction.user_name)
    };
    div()
        .flex()
        .items_center()
        .gap(px(10.))
        .h(px(40.))
        .px(px(14.))
        .border_b_1()
        .border_color(colors.border)
        .child(dot(tone, px(7.)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .font_weight(theme.type_scale.label_weight)
                        .text_color(colors.text)
                        .child(name),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(theme.type_scale.caption)
                        .text_color(colors.text_muted)
                        .child(format!(
                            "{party}, {}",
                            day_and_time(&transaction.created_at)
                        )),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_color(colors.text_muted)
                .font_features(tabular_figures())
                .child(format!("{} x", number(transaction.quantity))),
        )
        .child(
            div()
                .w(px(64.))
                .flex_shrink_0()
                .flex()
                .justify_end()
                .font_features(tabular_figures())
                .text_color(tone)
                .child(format!(
                    "{}{}",
                    if sale { "+" } else { "-" },
                    number(transaction.price)
                )),
        )
}

fn recent_transactions(summary: &DashboardSummary, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let key = |color, label: String| {
        div()
            .flex()
            .items_center()
            .gap(px(6.))
            .text_size(theme.type_scale.caption + px(1.))
            .text_color(colors.text_muted)
            .child(dot(color, px(7.)))
            .child(label)
    };
    kit::card(cx)
        .flex_1()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(14.))
                .h(px(40.))
                .border_b_1()
                .border_color(colors.border)
                .child(kit::heading("Latest transactions", cx))
                .child(
                    div()
                        .flex()
                        .gap(px(14.))
                        .child(key(
                            colors.positive,
                            format!("{} sales", number(summary.total.sale_count)),
                        ))
                        .child(key(
                            colors.info,
                            format!("{} purchases", number(summary.total.purchases_count)),
                        )),
                ),
        )
        .when(summary.recent_transactions.is_empty(), |card| {
            card.child(div().h(px(160.)).child(placeholder(
                IconName::Inbox,
                "No transactions yet",
                "A sale or a purchase you record shows up here.",
                cx,
            )))
        })
        .children(
            summary
                .recent_transactions
                .iter()
                .map(|transaction| transaction_row(transaction, cx)),
        )
}

fn categories(categories: &[CategorySummary], cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let cell = |text: String, width: f32| {
        div()
            .w(px(width))
            .flex_shrink_0()
            .flex()
            .justify_end()
            .font_features(tabular_figures())
            .child(text)
    };
    let header = div()
        .flex()
        .items_center()
        .gap(px(8.))
        .h(theme.metrics.header_height)
        .px(px(14.))
        .border_b_1()
        .border_color(colors.border)
        .text_size(theme.type_scale.caption)
        .font_weight(theme.type_scale.label_weight)
        .text_color(colors.text_muted)
        .child(div().flex_1().child("Category"))
        .child(cell("Trades".to_owned(), 60.))
        .child(cell("Revenue".to_owned(), 80.))
        .child(cell("Expenses".to_owned(), 80.))
        .child(cell("Profit".to_owned(), 80.))
        .child(cell("Margin".to_owned(), 70.));
    kit::card(cx)
        .flex_1()
        .child(
            div()
                .flex()
                .items_center()
                .px(px(14.))
                .h(px(40.))
                .border_b_1()
                .border_color(colors.border)
                .child(kit::heading("Totals by category", cx)),
        )
        .child(header)
        .children(categories.iter().map(|category| {
            let report = &category.report;
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .h(theme.metrics.row_height)
                .px(px(14.))
                .border_b_1()
                .border_color(colors.border)
                .text_color(colors.text_muted)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_weight(theme.type_scale.label_weight)
                        .text_color(colors.text)
                        .child(category.name.clone()),
                )
                .child(cell(number(report.total_transactions), 60.))
                .child(cell(number(report.revenue), 80.))
                .child(cell(number(report.expenses), 80.))
                .child(
                    cell(number(report.total_profit), 80.)
                        .text_color(Tone::of_profit(report.total_profit).color(theme)),
                )
                .child(cell(format!("{}%", kit::decimal(report.profit_margin)), 70.))
        }))
}

impl Render for HomeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = kit::scroll_page("home", cx);
        match &self.summary {
            Loaded::Loading => page.child(placeholder(
                IconName::LayoutDashboard,
                "Loading the dashboard",
                "Reading your transactions.",
                cx,
            )),
            Loaded::Failed(reason) => page.child(kit::failed(
                "Could not load the dashboard",
                reason,
                cx.listener(|view, _, _, cx| {
                    view.summary = Loaded::Loading;
                    view.load(cx);
                    cx.notify();
                }),
                cx,
            )),
            Loaded::Ready(summary) => page
                .child(totals(summary, cx))
                .child(charts(summary, cx))
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_start()
                        .gap(px(12.))
                        .child(recent_transactions(summary, cx))
                        .child(categories(&summary.categories, cx)),
                ),
        }
    }
}
