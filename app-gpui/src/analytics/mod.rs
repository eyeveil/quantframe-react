mod prices;
mod transactions;
mod users;

use std::collections::BTreeMap;

use gpui::{
    AnyElement, App, AppContext, Context, Div, Entity, IntoElement, ParentElement, Pixels, Styled,
    Subscription, Window, div, px,
};
use gpui_component::chart::LineChart;

use crate::{
    chart::{self, Series, Ticks},
    data::{Backend, BackendEvent, DataKind, DataSource},
    kit::{self, Loaded},
    model::{Transaction, TransactionType},
    theme::Theme,
    ui::{
        MONTHS, Segment,
        grid::Tone,
        number,
        page::{ListPage, PageHandle},
        screen::TabbedScreen,
        segmented,
    },
};
use prices::{ItemPrices, Prices, RivenPrices, SyndicatePrices};
use transactions::{Transactions, all_transactions};

pub fn screen(backend: &Entity<Backend>, window: &mut Window, cx: &mut App) -> Entity<TabbedScreen> {
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("analytics-tab");
        let list = cx.new(|cx| ListPage::new(Transactions::default(), backend, window, cx));
        let ledger = cx.new(|cx| Ledger::new(list, backend, cx));
        screen.add_view("transactions", "Transactions", ledger, cx);
        let page =
            cx.new(|cx| ListPage::new(Prices::<ItemPrices>::default(), backend, window, cx));
        screen.add("items", "Items", page, cx);
        let page =
            cx.new(|cx| ListPage::new(Prices::<RivenPrices>::default(), backend, window, cx));
        screen.add("rivens", "Rivens", page, cx);
        let page =
            cx.new(|cx| ListPage::new(Prices::<SyndicatePrices>::default(), backend, window, cx));
        screen.add("syndicates", "Syndicates", page, cx);
        let users = cx.new(|cx| users::Users::new(backend, cx));
        screen.add_view("users", "Users", users, cx);
        screen
    })
}

const RANGES: [(&str, Option<i64>); 4] = [
    ("30 days", Some(30)),
    ("90 days", Some(90)),
    ("1 year", Some(365)),
    ("All", None),
];
const DAILY_LIMIT: i64 = 120;

pub struct Ledger {
    list: Entity<ListPage<Transactions>>,
    source: DataSource,
    chart: bool,
    range: usize,
    rows: Loaded<Vec<Transaction>>,
    _subscriptions: Vec<Subscription>,
}

impl Ledger {
    fn new(
        list: Entity<ListPage<Transactions>>,
        backend: &Entity<Backend>,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscriptions = vec![
            cx.observe(&list, |_, _, cx| cx.notify()),
            cx.subscribe(backend, |ledger, _, event: &BackendEvent, cx| {
                if matches!(event, BackendEvent::Changed(DataKind::Transactions)) {
                    ledger.load(cx);
                }
            }),
        ];
        let mut ledger = Self {
            list,
            source: backend.read(cx).source.clone(),
            chart: std::env::var("QF_VIEW").as_deref() == Ok("chart"),
            range: 1,
            rows: Loaded::Loading,
            _subscriptions: subscriptions,
        };
        ledger.load(cx);
        ledger
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let rows = self.source.get_transaction_pagination(&all_transactions());
        cx.spawn(async move |ledger, cx| {
            let rows = Loaded::from_result(rows.await.map(|page| page.results));
            ledger
                .update(cx, |ledger, cx| {
                    ledger.rows = rows;
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn render_chart(&self, cx: &mut Context<Self>) -> AnyElement {
        let page = kit::scroll_page("ledger-chart", cx);
        let rows = match &self.rows {
            Loaded::Loading => {
                return page
                    .child(kit::note("Loading the transactions.", cx))
                    .into_any_element();
            }
            Loaded::Failed(reason) => {
                return page
                    .child(kit::failed(
                        "Could not load the transactions",
                        reason,
                        cx.listener(|ledger, _, _, cx| ledger.load(cx)),
                        cx,
                    ))
                    .into_any_element();
            }
            Loaded::Ready(rows) => rows,
        };
        let (label, days) = RANGES[self.range];
        let cutoff = days.map(kit::date_days_ago);
        let monthly = days.is_none_or(|days| days > DAILY_LIMIT);
        let mut buckets: BTreeMap<String, (i64, i64)> = BTreeMap::new();
        for row in rows {
            let Some(day) = row.created_at.get(..10) else {
                continue;
            };
            if cutoff.as_deref().is_some_and(|cutoff| day < cutoff) {
                continue;
            }
            let key = if monthly { &day[..7] } else { day };
            let bucket = buckets.entry(key.to_owned()).or_default();
            match row.transaction_type {
                TransactionType::Sale => bucket.0 += row.price,
                TransactionType::Purchase => bucket.1 += row.price,
            }
        }
        if buckets.is_empty() {
            return page
                .child(chart::empty(
                    "No transactions in this period.",
                    240.,
                    cx,
                ))
                .into_any_element();
        }

        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let labels: Vec<String> = buckets.keys().map(|key| bucket_label(key)).collect();
        let revenue: Vec<i64> = buckets.values().map(|bucket| bucket.0).collect();
        let expenses: Vec<i64> = buckets.values().map(|bucket| bucket.1).collect();
        let (total_revenue, total_expenses) = (
            revenue.iter().sum::<i64>(),
            expenses.iter().sum::<i64>(),
        );
        let mut running = 0;
        let cumulative: Vec<(String, f64)> = labels
            .iter()
            .zip(revenue.iter().zip(&expenses))
            .map(|(label, (revenue, expenses))| {
                running += revenue - expenses;
                (label.clone(), running as f64)
            })
            .collect();
        let profit = total_revenue - total_expenses;
        let unit = if monthly { "month" } else { "day" };
        let period = match days {
            Some(365) => "in the last year".to_owned(),
            Some(_) => format!("in the last {label}"),
            None => "since the first transaction".to_owned(),
        };

        page.child(
            div()
                .flex()
                .flex_shrink_0()
                .gap(px(12.))
                .child(kit::stat(
                    "Revenue",
                    number(total_revenue),
                    period.clone(),
                    Tone::Plain,
                    cx,
                ))
                .child(kit::stat(
                    "Expenses",
                    number(total_expenses),
                    period.clone(),
                    Tone::Plain,
                    cx,
                ))
                .child(kit::stat(
                    "Profit",
                    number(profit),
                    period,
                    Tone::of_profit(profit),
                    cx,
                )),
        )
        .child(
            kit::card(cx)
                .flex_shrink_0()
                .gap(px(10.))
                .p(px(14.))
                .child(kit::heading(format!("Revenue and expenses per {unit}"), cx))
                .child(chart::bars(
                    "ledger-bars",
                    &labels,
                    Ticks::FromLast,
                    &[
                        Series::new("Revenue", colors.positive, revenue),
                        Series::new("Expenses", colors.negative, expenses),
                    ],
                    200.,
                    cx,
                )),
        )
        .child(
            kit::card(cx)
                .flex_shrink_0()
                .gap(px(10.))
                .p(px(14.))
                .child(kit::heading("Cumulative profit", cx))
                .child(
                    div().h(px(200.)).child(
                        LineChart::new(cumulative)
                            .x(|point: &(String, f64)| point.0.clone())
                            .y(|point: &(String, f64)| point.1)
                            .stroke(colors.accent)
                            .linear()
                            .y_axis(true)
                            .y_tick_format(|value| number(value.round() as i64))
                            .x_tick_count(10)
                            .appear(false)
                            .dot(),
                    ),
                ),
        )
        .into_any_element()
    }
}

fn bucket_label(key: &str) -> String {
    let month = key
        .get(5..7)
        .and_then(|month| month.parse::<usize>().ok())
        .and_then(|month| MONTHS.get(month.wrapping_sub(1)))
        .copied()
        .unwrap_or_default();
    match key.get(8..10) {
        Some(day) => format!("{} {month}", day.trim_start_matches('0')),
        None => format!("{month} {}", key.get(..4).unwrap_or_default()),
    }
}

fn choices(
    id: &'static str,
    labels: impl IntoIterator<Item = &'static str>,
    active: usize,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Div {
    segmented(
        id,
        labels
            .into_iter()
            .map(|label| Segment { label, count: None })
            .collect(),
        active,
        on_select,
        cx,
    )
}

impl PageHandle for Entity<Ledger> {
    fn toolbar(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let (chart, range, list) = {
            let ledger = self.read(cx);
            (ledger.chart, ledger.range, ledger.list.clone())
        };
        let ledger = self.downgrade();
        let view = choices(
            "ledger-view",
            ["List", "Chart"],
            usize::from(chart),
            move |index, _, cx| {
                ledger
                    .update(cx, |ledger, cx| {
                        ledger.chart = index == 1;
                        cx.notify();
                    })
                    .ok();
            },
            cx,
        );
        let ledger = self.downgrade();
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(view)
            .child(if chart {
                choices(
                    "ledger-range",
                    RANGES.map(|(label, _)| label),
                    range,
                    move |index, _, cx| {
                        ledger
                            .update(cx, |ledger, cx| {
                                ledger.range = index;
                                cx.notify();
                            })
                            .ok();
                    },
                    cx,
                )
                .into_any_element()
            } else {
                list.toolbar(window, cx)
            })
            .into_any_element()
    }

    fn body(&self, width: Pixels, window: &mut Window, cx: &mut App) -> AnyElement {
        let (chart, list) = {
            let ledger = self.read(cx);
            (ledger.chart, ledger.list.clone())
        };
        if chart {
            self.update(cx, |ledger, cx| ledger.render_chart(cx))
        } else {
            list.body(width, window, cx)
        }
    }

    fn count(&self, cx: &App) -> Option<usize> {
        self.read(cx).list.count(cx)
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        self.read(cx).list.clone().focus(window, cx);
    }

    fn focus_search(&self, window: &mut Window, cx: &mut App) {
        self.read(cx).list.clone().focus_search(window, cx);
    }

    fn on_load(&self, row: Option<usize>, dialog: Option<String>, cx: &mut App) {
        self.read(cx).list.clone().on_load(row, dialog, cx);
    }
}
