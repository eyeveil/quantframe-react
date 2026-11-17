//! Small pieces shared by the shell and the screens.

use gpui::{
    App, Div, ElementId, Hsla, IntoElement, ParentElement, Pixels, SharedString, Stateful, Styled,
    div, prelude::*, px,
};
use gpui_component::{Icon, Sizable, Size, button::Button, tooltip::Tooltip};
use gpui_kit_assets::IconName;

use crate::{
    model::StockStatus,
    theme::{StatusStyle, Theme},
};

pub fn icon(name: IconName, size: Pixels, color: Hsla) -> Icon {
    Icon::new(name).size(size).text_color(color)
}

/// Formats a platinum amount or a count with thousands separators.
pub fn number(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        grouped.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

pub fn percent(ratio: f64) -> String {
    format!("{:.1}%", ratio * 100.0)
}

/// Text for column headers and group labels, in capitals when the theme asks.
pub fn label_text(theme: &Theme, text: &str) -> SharedString {
    if theme.type_scale.capital_labels {
        text.to_uppercase().into()
    } else {
        text.to_owned().into()
    }
}

/// A labelled button at the theme's control height.
pub fn text_button(id: impl Into<ElementId>, label: &str, cx: &App) -> Button {
    let theme = Theme::global(cx);
    Button::new(id)
        .label(label.to_owned())
        .with_size(control_size(theme))
        .h(theme.metrics.control_height)
}

/// A square icon button with a tooltip, for row and toolbar actions.
pub struct IconButton {
    id: ElementId,
    name: IconName,
    tooltip: SharedString,
    tone: Option<Hsla>,
    hover_tone: Option<Hsla>,
    disabled: bool,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>, name: IconName, tooltip: &'static str) -> Self {
        Self {
            id: id.into(),
            name,
            tooltip: tooltip.into(),
            tone: None,
            hover_tone: None,
            disabled: false,
        }
    }

    /// The icon colour at rest. The default is the muted text colour.
    pub fn tone(mut self, tone: Hsla) -> Self {
        self.tone = Some(tone);
        self
    }

    /// The icon colour under the pointer. The default is the text colour.
    pub fn hover_tone(mut self, tone: Hsla) -> Self {
        self.hover_tone = Some(tone);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn build(self, cx: &App) -> Stateful<Div> {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let side = theme.metrics.action_button;
        let tooltip = self.tooltip;
        let button = div()
            .id(self.id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(side)
            .rounded(theme.metrics.radius)
            .child(Icon::new(self.name).size(theme.metrics.icon))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx));
        if self.disabled {
            return button.text_color(colors.text_faint.opacity(0.55));
        }
        let hover_tone = self.hover_tone.unwrap_or(colors.text);
        button
            .cursor_pointer()
            .text_color(self.tone.unwrap_or(colors.text_muted))
            .hover(|button| button.bg(colors.control_hover).text_color(hover_tone))
            .active(|button| button.bg(colors.control_active))
    }
}

/// The gpui-component size preset closest to the theme's control height.
fn control_size(theme: &Theme) -> Size {
    if theme.metrics.control_height >= px(32.) {
        Size::Medium
    } else {
        Size::Small
    }
}

/// A clickable filter chip. The selected chip takes the accent colours.
pub fn chip(id: impl Into<ElementId>, selected: bool, cx: &App) -> Stateful<Div> {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(6.))
        .h(theme.metrics.control_height - px(4.))
        .px(px(9.))
        .rounded(theme.metrics.control_height)
        .border_1()
        .text_size(theme.type_scale.caption + px(1.))
        .cursor_pointer()
        .map(|chip| {
            if selected {
                chip.border_color(colors.accent)
                    .bg(colors.row_selected)
                    .text_color(colors.text)
            } else {
                chip.border_color(colors.border)
                    .text_color(colors.text_muted)
                    .hover(|chip| chip.bg(colors.control_hover).text_color(colors.text))
            }
        })
}

pub fn dot(color: Hsla, size: Pixels) -> Div {
    div().flex_shrink_0().size(size).rounded(size).bg(color)
}

pub fn status_indicator(status: StockStatus, cx: &App) -> impl IntoElement + use<> {
    let theme = Theme::global(cx);
    let color = theme.status_color(status);
    let row = div().flex().items_center().gap(px(7.)).min_w_0();
    match theme.layout.status {
        StatusStyle::Dot => row
            .child(dot(color, px(7.)))
            .child(div().truncate().child(status.label())),
        StatusStyle::OutlinedTag => row.child(
            div()
                .px(px(6.))
                .h(px(17.))
                .flex()
                .items_center()
                .rounded(theme.metrics.radius)
                .border_1()
                .border_color(color.opacity(0.55))
                .text_color(color)
                .text_size(theme.type_scale.caption)
                .font_weight(theme.type_scale.label_weight)
                .child(status.label()),
        ),
        StatusStyle::ColoredText => row.child(
            div()
                .truncate()
                .text_color(color)
                .font_weight(theme.type_scale.label_weight)
                .child(status.label()),
        ),
    }
}

/// A vertical hairline between groups in a toolbar.
pub fn divider(cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .flex_shrink_0()
        .w(px(1.))
        .h(theme.metrics.control_height - px(8.))
        .bg(theme.colors.border)
}
