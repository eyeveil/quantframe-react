use gpui::{
    AnyElement, App, Context, Div, Entity, IntoElement, ParentElement, Pixels, Styled, Window,
    div, px,
};
use gpui_component::chart::LineChart;
use gpui_kit_assets::IconName;

use super::choices;
use crate::{
    data::{Backend, DataSource},
    kit::{self, Loaded},
    model::UserActivity,
    theme::Theme,
    ui::{IconButton, day_and_time, grid::Tone, number, page::PageHandle},
};

const RANGES: [(&str, i64); 3] = [("7 days", 7), ("30 days", 30), ("90 days", 90)];

pub struct Users {
    source: DataSource,
    range: usize,
    activity: Loaded<UserActivity>,
    generation: u64,
}

impl Users {
    pub fn new(backend: &Entity<Backend>, cx: &mut Context<Self>) -> Self {
        let mut users = Self {
            source: backend.read(cx).source.clone(),
            range: 1,
            activity: Loaded::Loading,
            generation: 0,
        };
        users.load(cx);
        users
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        let generation = self.generation;
        self.activity = Loaded::Loading;
        let days = RANGES[self.range].1;
        let activity = self.source.get_user_activity(
            format!("{}T00:00:00Z", kit::date_days_ago(days)),
            format!("{}T23:59:59Z", kit::date_days_ago(0)),
        );
        cx.spawn(async move |users, cx| {
            let activity = Loaded::from_result(activity.await);
            users
                .update(cx, |users, cx| {
                    if users.generation == generation {
                        users.activity = activity;
                        cx.notify();
                    }
                })
                .ok();
        })
        .detach();
        cx.notify();
    }

    fn series(
        title: &'static str,
        activity: &UserActivity,
        values: &[f64],
        color: gpui::Hsla,
        cx: &App,
    ) -> Div {
        let points: Vec<(String, f64)> = activity
            .labels
            .iter()
            .map(|label| day_and_time(label))
            .zip(values.iter().copied())
            .collect();
        kit::card(cx)
            .flex_shrink_0()
            .gap(px(10.))
            .p(px(14.))
            .child(kit::heading(title, cx))
            .child(
                div().h(px(200.)).child(
                    LineChart::new(points)
                        .x(|point: &(String, f64)| point.0.clone())
                        .y(|point: &(String, f64)| point.1)
                        .stroke(color)
                        .linear()
                        .y_axis(true)
                        .y_tick_format(|value| number(value.round() as i64))
                        .x_tick_count(10)
                        .appear(false)
                        .dot(),
                ),
            )
    }

    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = Theme::global(cx).colors;
        let page = kit::scroll_page("users", cx);
        let activity = match &self.activity {
            Loaded::Loading => {
                return page
                    .child(kit::note("Loading the user counts from Quantframe's server.", cx))
                    .into_any_element();
            }
            Loaded::Failed(reason) => {
                return page
                    .child(kit::failed(
                        "Could not load the user counts",
                        reason,
                        cx.listener(|users, _, _, cx| users.load(cx)),
                        cx,
                    ))
                    .into_any_element();
            }
            Loaded::Ready(activity) => activity,
        };
        let last = |values: &[f64]| values.last().copied().unwrap_or(0.) as i64;
        let peak = |values: &[f64]| values.iter().copied().fold(0., f64::max) as i64;
        let label = RANGES[self.range].0;
        page.child(
            div()
                .flex()
                .flex_shrink_0()
                .gap(px(12.))
                .child(kit::stat(
                    "Registered users",
                    number(last(&activity.registered_users)),
                    format!(
                        "highest {} in the last {label}",
                        number(peak(&activity.registered_users))
                    ),
                    Tone::Plain,
                    cx,
                ))
                .child(kit::stat(
                    "Active users",
                    number(last(&activity.total_users)),
                    format!(
                        "highest {} in the last {label}",
                        number(peak(&activity.total_users))
                    ),
                    Tone::Plain,
                    cx,
                )),
        )
        .child(Self::series(
            "Registered users per day",
            activity,
            &activity.registered_users,
            colors.accent,
            cx,
        ))
        .child(Self::series(
            "Active users per day",
            activity,
            &activity.total_users,
            colors.positive,
            cx,
        ))
        .into_any_element()
    }
}

impl PageHandle for Entity<Users> {
    fn toolbar(&self, _: &mut Window, cx: &mut App) -> AnyElement {
        let range = self.read(cx).range;
        let users = self.downgrade();
        let reload = self.downgrade();
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                IconButton::new("users-reload", IconName::RefreshCw, "Load the user counts again")
                    .on_click(move |_, _, cx| {
                        reload.update(cx, |users, cx| users.load(cx)).ok();
                    }),
            )
            .child(choices(
                "users-range",
                RANGES.map(|(label, _)| label),
                range,
                move |index, _, cx| {
                    users
                        .update(cx, |users, cx| {
                            users.range = index;
                            users.load(cx);
                        })
                        .ok();
                },
                cx,
            ))
            .into_any_element()
    }

    fn body(&self, _: Pixels, _: &mut Window, cx: &mut App) -> AnyElement {
        self.update(cx, |users, cx| users.render_body(cx))
    }

    fn count(&self, _: &App) -> Option<usize> {
        None
    }

    fn focus(&self, _: &mut Window, _: &mut App) {}

    fn focus_search(&self, _: &mut Window, _: &mut App) {}

    fn on_load(&self, _: Option<usize>, _: Option<String>, _: &mut App) {}
}
