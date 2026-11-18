pub mod cells;
pub mod choices;
pub mod form;
pub mod grid;
pub mod page;
pub mod panel;
pub mod prompts;
pub mod screen;

use gpui::{
    App, BoxShadow, ClickEvent, Div, ElementId, Entity, Focusable, Hsla, InteractiveElement, Interactivity,
    IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, point, prelude::*, px,
};
use gpui_component::{
    Icon, Selectable, Sizable, Size,
    button::Button,
    input::{Input, InputState},
    tooltip::Tooltip,
};
use gpui_kit_assets::IconName;

use crate::theme::{Theme, tabular_figures};

pub fn icon(name: IconName, size: Pixels, color: Hsla) -> Icon {
    Icon::new(name).size(size).text_color(color)
}

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

pub fn day_and_time(timestamp: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = timestamp
        .get(5..7)
        .and_then(|month| month.parse::<usize>().ok())
        .and_then(|month| MONTHS.get(month.wrapping_sub(1)));
    match (month, timestamp.get(8..10), timestamp.get(11..16)) {
        (Some(month), Some(day), Some(time)) => {
            format!("{} {month}, {time}", day.trim_start_matches('0'))
        }
        _ => timestamp.to_owned(),
    }
}

/// The outline of a control that has keyboard focus. It is drawn inside the
/// control's bounds, because a shadow outside them would show through a
/// control with no background as a filled block.
pub fn focus_ring(theme: &Theme) -> Vec<BoxShadow> {
    ring(theme, true)
}

fn ring(theme: &Theme, inset: bool) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: theme.colors.accent.opacity(if inset { 1.0 } else { 0.45 }),
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: theme.metrics.focus_ring,
        inset,
    }]
}

pub fn text_button(id: impl Into<ElementId>, label: &str, cx: &App) -> Button {
    let theme = Theme::global(cx);
    Button::new(id)
        .label(label.to_owned())
        .with_size(Size::Small)
        .h(theme.metrics.control_height)
}

pub fn row_button(id: impl Into<ElementId>, label: &str, cx: &App) -> Button {
    let theme = Theme::global(cx);
    Button::new(id)
        .label(label.to_owned())
        .with_size(Size::Small)
        .h(theme.metrics.action_button)
        .tab_stop(false)
}

#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    base: Stateful<Div>,
    name: IconName,
    tooltip: SharedString,
    tone: Option<Hsla>,
    hover_tone: Option<Hsla>,
    disabled: bool,
    selected: bool,
    tab_stop: bool,
}

impl IconButton {
    pub fn new(
        id: impl Into<ElementId>,
        name: IconName,
        tooltip: impl Into<SharedString>,
    ) -> Self {
        let id = id.into();
        Self {
            base: div().id(id.clone()),
            id,
            name,
            tooltip: tooltip.into(),
            tone: None,
            hover_tone: None,
            disabled: false,
            selected: false,
            tab_stop: true,
        }
    }

    pub fn tone(mut self, tone: Hsla) -> Self {
        self.tone = Some(tone);
        self
    }

    pub fn hover_tone(mut self, tone: Hsla) -> Self {
        self.hover_tone = Some(tone);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_click(handler);
        self
    }

    pub fn in_row(mut self) -> Self {
        self.tab_stop = false;
        self
    }
}

impl Styled for IconButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for IconButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for IconButton {}

impl Selectable for IconButton {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for IconButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let ring = focus_ring(theme);
        let tooltip = self.tooltip;
        let base = if self.disabled {
            div().id(self.id)
        } else {
            self.base
        };
        let button = base
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(theme.metrics.action_button)
            .rounded(theme.metrics.radius)
            .child(Icon::new(self.name).size(theme.metrics.icon))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx));
        if self.disabled {
            return button.text_color(colors.text_faint.opacity(0.5));
        }
        let hover_tone = self.hover_tone.unwrap_or(colors.text);
        button
            .cursor_pointer()
            .text_color(self.tone.unwrap_or(colors.text_muted))
            .when(self.selected, |button| {
                button.bg(colors.control_active).text_color(hover_tone)
            })
            .when(self.tab_stop, |button| {
                button
                    .tab_index(0)
                    .focus_visible(move |style| style.shadow(ring))
            })
            .hover(|button| button.bg(colors.control_hover).text_color(hover_tone))
            .active(|button| button.bg(colors.control_active))
    }
}

pub fn chip(id: impl Into<ElementId>, selected: bool, cx: &App) -> Stateful<Div> {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let ring = focus_ring(theme);
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
        .tab_index(0)
        .focus_visible(move |style| style.shadow(ring))
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

pub fn dot_label(color: Hsla, label: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(7.))
        .min_w_0()
        .child(dot(color, px(7.)))
        .child(div().truncate().child(label.into()))
}

pub fn divider(cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .flex_shrink_0()
        .w(px(1.))
        .h(theme.metrics.control_height - px(8.))
        .bg(theme.colors.border)
}

pub fn numeric(text: impl Into<SharedString>) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_end()
        .font_features(tabular_figures())
        .child(text.into())
}

pub struct Segment {
    pub label: &'static str,
    pub count: Option<usize>,
}

pub fn segmented(
    id: &'static str,
    segments: Vec<Segment>,
    active: usize,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let on_select = std::rc::Rc::new(on_select);
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(2.))
        .p(px(2.))
        .rounded(theme.metrics.radius + px(1.))
        .bg(colors.raised)
        .children(segments.into_iter().enumerate().map(|(index, segment)| {
            let selected = index == active;
            let ring = focus_ring(theme);
            let on_select = on_select.clone();
            div()
                .id((id, index))
                .flex()
                .items_center()
                .gap(px(6.))
                .h(theme.metrics.control_height - px(4.))
                .px(px(12.))
                .rounded(theme.metrics.radius - px(1.))
                .text_size(theme.type_scale.body)
                .font_weight(theme.type_scale.label_weight)
                .cursor_pointer()
                .tab_index(0)
                .focus_visible(move |style| style.shadow(ring))
                .on_click(move |_, window, cx| on_select(index, window, cx))
                .map(|segment| {
                    if selected {
                        segment.bg(colors.segment_active).text_color(colors.text)
                    } else {
                        segment
                            .text_color(colors.text_muted)
                            .hover(|segment| segment.text_color(colors.text))
                    }
                })
                .child(segment.label)
                .children(segment.count.map(|count| {
                    div()
                        .text_size(theme.type_scale.caption)
                        .font_features(tabular_figures())
                        .text_color(if selected {
                            colors.text_muted
                        } else {
                            colors.text_faint
                        })
                        .child(number(count as i64))
                }))
        }))
}

/// The search box. gpui-component fixes its input's height per size preset,
/// so the frame is drawn here at the theme's control height.
pub fn search_field(
    input: &Entity<InputState>,
    width: Pixels,
    window: &Window,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let focused = input.focus_handle(cx).is_focused(window);
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
        .border_color(if focused {
            colors.accent
        } else {
            colors.border_strong
        })
        .bg(colors.raised)
        .when(focused, |field| field.shadow(ring(theme, false)))
        .child(icon(
            IconName::Search,
            theme.metrics.icon,
            colors.text_muted,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(Input::new(input).small().appearance(false).cleanable(true)),
        )
}

pub fn placeholder(name: IconName, title: &str, detail: &str, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .child(icon(name, px(26.), colors.text_faint))
        .child(
            div()
                .pt(px(4.))
                .text_size(theme.type_scale.body)
                .font_weight(theme.type_scale.label_weight)
                .text_color(colors.text)
                .child(title.to_owned()),
        )
        .child(
            div()
                .max_w(px(560.))
                .text_center()
                .text_size(theme.type_scale.caption + px(1.))
                .text_color(colors.text_muted)
                .child(detail.to_owned()),
        )
}
