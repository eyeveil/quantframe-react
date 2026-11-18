//! A grouped bar chart drawn with plain elements. gpui-component's
//! `BarChart` draws one series, and the dashboard compares two.

use gpui::{App, Div, Hsla, ParentElement, SharedString, Styled, div, prelude::*, px};
use gpui_component::tooltip::Tooltip;

use crate::{
    theme::{Theme, tabular_figures},
    ui::{dot, number},
};

pub struct Series {
    pub name: &'static str,
    pub color: Hsla,
    pub values: Vec<i64>,
}

impl Series {
    pub fn new(name: &'static str, color: Hsla, values: Vec<i64>) -> Self {
        Self {
            name,
            color,
            values,
        }
    }
}

const AXIS_WIDTH: f32 = 46.;
/// The most labels the horizontal axis shows before it skips some.
const MAX_LABELS: usize = 12;

/// The step that leaves at most [`MAX_LABELS`] of `count` labels.
fn label_step(count: usize) -> usize {
    count.div_ceil(MAX_LABELS).max(1)
}

/// One group of bars per label, one bar per series that has a value there.
/// Values below zero hang under the zero line. Each group shows its values
/// in a tooltip.
pub fn bars(
    id: &'static str,
    labels: &[String],
    series: &[Series],
    height: f32,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let values = || series.iter().flat_map(|series| series.values.iter().copied());
    let high = values().max().unwrap_or(0).max(0);
    let low = values().min().unwrap_or(0).min(0);
    let span = (high - low).max(1) as f32;
    let above = height * high as f32 / span;
    let below = height - above;
    let groups = labels.len();
    let step = label_step(groups);

    let axis_label = |value: i64| {
        div()
            .h(px(12.))
            .text_size(theme.type_scale.caption)
            .font_features(tabular_figures())
            .text_color(colors.text_faint)
            .child(number(value))
    };
    let axis = div()
        .w(px(AXIS_WIDTH))
        .h(px(height))
        .flex_shrink_0()
        .relative()
        .child(div().absolute().top(px(-6.)).right(px(8.)).child(axis_label(high)))
        .when(above > 18. && below > 0., |axis| {
            axis.child(
                div()
                    .absolute()
                    .top(px(above - 6.))
                    .right(px(8.))
                    .child(axis_label(0)),
            )
        })
        .child(
            div()
                .absolute()
                .top(px(height - 6.))
                .right(px(8.))
                .child(axis_label(low)),
        );

    let group = |index: usize| {
        let bar = |color: Hsla, length: f32, up: bool| {
            let bar = div().flex_1().max_w(px(16.)).h(px(length)).bg(color);
            if up {
                bar.rounded_t(px(2.))
            } else {
                bar.rounded_b(px(2.))
            }
        };
        let length = |value: i64, room: f32, reach: i64| {
            if reach == 0 {
                0.
            } else {
                (room * value as f32 / reach as f32).max(1.5)
            }
        };
        let value = |series: &Series| series.values.get(index).copied().unwrap_or(0);
        let tooltip: SharedString = series
            .iter()
            .map(|series| format!("{} {}", series.name, number(value(series))))
            .collect::<Vec<_>>()
            .join(", ")
            .into();
        let title = labels.get(index).cloned().unwrap_or_default();
        div()
            .id((id, index))
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .rounded(px(3.))
            .hover(|group| group.bg(colors.control_hover))
            .tooltip(move |window, cx| {
                Tooltip::new(format!("{title}: {tooltip}")).build(window, cx)
            })
            .child(
                div()
                    .h(px(above))
                    .flex()
                    .items_end()
                    .justify_center()
                    .gap(px(2.))
                    .px(px(5.))
                    .children(series.iter().filter(|series| value(series) > 0).map(
                        |series| bar(series.color, length(value(series), above, high), true),
                    )),
            )
            .child(
                div()
                    .h(px(below))
                    .flex()
                    .items_start()
                    .justify_center()
                    .gap(px(2.))
                    .px(px(5.))
                    .children(series.iter().filter(|series| value(series) < 0).map(
                        |series| {
                            bar(
                                series.color.opacity(0.7),
                                length(value(series), below, low),
                                false,
                            )
                        },
                    )),
            )
    };

    let plot = div()
        .flex_1()
        .min_w_0()
        .h(px(height))
        .relative()
        .child(
            div()
                .absolute()
                .top(px(0.))
                .left(px(0.))
                .right(px(0.))
                .h(px(1.))
                .bg(colors.border),
        )
        .child(
            div()
                .absolute()
                .top(px(above))
                .left(px(0.))
                .right(px(0.))
                .h(px(1.))
                .bg(colors.border_strong),
        )
        .child(div().size_full().flex().children((0..groups).map(group)));

    let ticks = div()
        .flex()
        .pl(px(AXIS_WIDTH))
        .children(labels.iter().enumerate().map(|(index, label)| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .justify_center()
                .text_size(theme.type_scale.caption)
                .text_color(colors.text_faint)
                .when(index % step == 0, |tick| {
                    tick.child(div().whitespace_nowrap().child(label.clone()))
                })
        }));

    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(div().flex().pt(px(6.)).child(axis).child(plot))
        .child(ticks)
        .when(series.len() > 1, |chart| {
            chart.child(legend(
                series.iter().map(|series| (series.name, series.color)),
                cx,
            ))
        })
}

/// A dot and a name per series.
pub fn legend(entries: impl IntoIterator<Item = (&'static str, Hsla)>, cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .flex()
        .items_center()
        .gap(px(14.))
        .pl(px(AXIS_WIDTH))
        .text_size(theme.type_scale.caption + px(1.))
        .text_color(theme.colors.text_muted)
        .children(entries.into_iter().map(|(name, color)| {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(dot(color, px(7.)))
                .child(name)
        }))
}

/// The line shown in place of a chart that has no values.
pub fn empty(text: &'static str, height: f32, cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .h(px(height))
        .flex()
        .items_center()
        .justify_center()
        .text_size(theme.type_scale.caption + px(1.))
        .text_color(theme.colors.text_faint)
        .child(text)
}
