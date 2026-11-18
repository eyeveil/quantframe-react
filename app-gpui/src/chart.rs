//! A grouped bar chart drawn with plain elements. gpui-component's
//! `BarChart` draws one series, and the dashboard compares two.

use gpui::{
    App, Bounds, Div, Hsla, ParentElement, Pixels, SharedString, Styled, TextAlign, TextRun,
    Window, canvas, div, point, prelude::*, px,
};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ticks {
    FromFirst,
    FromLast,
}

impl Ticks {
    fn first_index(self, groups: usize, step: usize) -> usize {
        match self {
            Self::FromFirst => 0,
            Self::FromLast => groups.saturating_sub(1) % step,
        }
    }
}

const AXIS_WIDTH: f32 = 46.;
const TICK_HEIGHT: f32 = 14.;
const TICK_GAP: f32 = 6.;

fn label_step(groups: usize, group_width: f32, label_width: f32) -> usize {
    if groups == 0 || group_width <= 0. {
        return 1;
    }
    let needed = ((label_width + TICK_GAP) / group_width).ceil().max(1.) as usize;
    (needed..=groups / 2)
        .find(|step| groups.is_multiple_of(*step))
        .unwrap_or(needed)
}

fn paint_ticks(
    labels: &[SharedString],
    ticks: Ticks,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let lines: Vec<_> = labels
        .iter()
        .map(|label| {
            let run = TextRun {
                len: label.len(),
                font: style.font(),
                color: style.color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            window
                .text_system()
                .shape_line(label.clone(), font_size, &[run], None)
        })
        .collect();
    let widest = lines
        .iter()
        .map(|line| f32::from(line.width))
        .fold(0., f32::max);
    let group_width = f32::from(bounds.size.width) / lines.len().max(1) as f32;
    let step = label_step(lines.len(), group_width, widest);
    let first = ticks.first_index(lines.len(), step);
    for (index, line) in lines.iter().enumerate().skip(first).step_by(step) {
        let centre = px(group_width * (index as f32 + 0.5));
        let origin = point(bounds.origin.x + centre - line.width / 2., bounds.origin.y);
        line.paint(origin, px(TICK_HEIGHT), TextAlign::Left, None, window, cx)
            .ok();
    }
}

pub fn bars(
    id: &'static str,
    labels: &[String],
    ticks: Ticks,
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

    let tick_labels: Vec<SharedString> = labels.iter().cloned().map(SharedString::from).collect();
    let tick_row = div()
        .pl(px(AXIS_WIDTH))
        .text_size(theme.type_scale.caption)
        .text_color(colors.text_faint)
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, (), window, cx| paint_ticks(&tick_labels, ticks, bounds, window, cx),
            )
            .w_full()
            .h(px(TICK_HEIGHT)),
        );

    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(div().flex().pt(px(6.)).child(axis).child(plot))
        .child(tick_row)
        .when(series.len() > 1, |chart| {
            chart.child(legend(
                series.iter().map(|series| (series.name, series.color)),
                cx,
            ))
        })
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_step_keeps_every_label_that_fits() {
        assert_eq!(label_step(8, 45., 34.), 1);
        assert_eq!(label_step(12, 26., 19.), 1);
    }

    #[test]
    fn label_step_prefers_a_step_that_divides_the_groups() {
        assert_eq!(label_step(24, 13., 30.), 3);
        assert_eq!(label_step(24, 8., 30.), 6);
        assert_eq!(label_step(23, 13., 30.), 3);
        assert_eq!(label_step(0, 13., 30.), 1);
    }

    #[test]
    fn ticks_from_last_always_label_the_last_group() {
        assert_eq!(Ticks::FromLast.first_index(8, 2), 1);
        assert_eq!(Ticks::FromLast.first_index(8, 1), 0);
        assert_eq!(Ticks::FromLast.first_index(23, 3), 1);
        assert_eq!(Ticks::FromFirst.first_index(24, 3), 0);
    }
}
