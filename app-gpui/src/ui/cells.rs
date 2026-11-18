//! Table cells the list screens share.

use gpui::{AnyElement, App, Div, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px};
use gpui_kit_assets::IconName;

use super::{
    IconButton, dot_label,
    grid::{Emit, Tone, centered},
    icon, number, numeric,
};
use crate::theme::{Theme, tabular_figures};

/// The name of a row in the text colour, with small icons for its flags.
/// Each flag is an icon and the tooltip that explains it.
pub fn name(text: impl Into<SharedString>, flags: &[(IconName, &'static str)], cx: &App) -> AnyElement {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    div()
        .size_full()
        .flex()
        .items_center()
        .gap(px(8.))
        .min_w_0()
        .child(
            div()
                .truncate()
                .text_color(colors.text)
                .font_weight(theme.type_scale.label_weight)
                .child(text.into()),
        )
        .children(flags.iter().enumerate().map(|(index, (name, tooltip))| {
            let tooltip = *tooltip;
            div()
                .id(("flag", index))
                .flex_shrink_0()
                .child(icon(*name, theme.metrics.icon - px(2.), colors.text_faint))
                .tooltip(move |window, cx| {
                    gpui_component::tooltip::Tooltip::new(tooltip).build(window, cx)
                })
        }))
        .into_any_element()
}

/// Plain text in the muted row colour.
pub fn text(text: impl Into<SharedString>) -> AnyElement {
    centered(div().truncate().child(text.into()))
}

/// A right-aligned amount, or `empty` in the faint colour when there is none.
pub fn amount(value: Option<i64>, empty: &'static str, cx: &App) -> AnyElement {
    let colors = Theme::global(cx).colors;
    match value {
        Some(value) => numeric(number(value)).text_color(colors.text),
        None => numeric(empty).text_color(colors.text_faint),
    }
    .into_any_element()
}

/// A right-aligned amount in a tone, for a profit or a loss.
pub fn toned_amount(value: Option<i64>, empty: &'static str, tone: Tone, cx: &App) -> AnyElement {
    let theme = Theme::global(cx);
    match value {
        Some(value) => numeric(number(value)).text_color(tone.color(theme)),
        None => numeric(empty).text_color(theme.colors.text_faint),
    }
    .into_any_element()
}

/// A coloured dot and a label.
pub fn status(color: gpui::Hsla, label: impl Into<SharedString>, cx: &App) -> AnyElement {
    centered(dot_label(color, label).text_color(Theme::global(cx).colors.text))
}

/// A price limit with buttons that lower and raise it by `step` platinum.
pub fn stepped_price<A: Clone + 'static>(
    id: u64,
    value: Option<i64>,
    step: i64,
    lower: A,
    raise: A,
    emit: &Emit<A>,
    cx: &App,
) -> AnyElement {
    let colors = Theme::global(cx).colors;
    let value = match value {
        Some(price) => div()
            .font_features(tabular_figures())
            .text_color(colors.text)
            .child(number(price)),
        None => div().text_color(colors.text_faint).child("Not set"),
    };
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(4.))
        .child(value.flex_1().min_w_0().truncate())
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    IconButton::new(("price-down", id), IconName::Minus, format!("Lower by {step}"))
                        .in_row()
                        .tone(colors.text_faint)
                        .on_click(emit.on_click(lower)),
                )
                .child(
                    IconButton::new(("price-up", id), IconName::Plus, format!("Raise by {step}"))
                        .in_row()
                        .tone(colors.text_faint)
                        .on_click(emit.on_click(raise)),
                ),
        )
        .into_any_element()
}

/// The buttons at the end of a row, pushed to the right edge.
pub fn actions() -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_end()
        .gap(px(1.))
}

/// Small rounded labels, such as the tags of a trade message.
pub fn tags(tags: &[String], cx: &App) -> AnyElement {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    div()
        .size_full()
        .flex()
        .items_center()
        .gap(px(5.))
        .overflow_hidden()
        .children(tags.iter().map(|tag| {
            div()
                .flex_shrink_0()
                .px(px(7.))
                .h(px(19.))
                .flex()
                .items_center()
                .rounded(px(10.))
                .bg(colors.control_hover)
                .text_size(theme.type_scale.caption)
                .text_color(colors.text_muted)
                .child(tag.clone())
        }))
        .into_any_element()
}
