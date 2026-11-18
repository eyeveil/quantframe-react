use gpui::{
    App, ClickEvent, Div, Hsla, ParentElement, Stateful, Styled, Window, div, prelude::*, px,
};
use gpui_component::{Disableable, Icon, button::ButtonVariants};
use gpui_kit_assets::IconName;

use super::{
    IconButton, counted, dot_label, focus_ring,
    grid::{Emit, Tone},
    icon, number, text_button,
};
use crate::theme::{Theme, tabular_figures};

pub struct PanelModel<A> {
    pub title: String,
    pub subtitle: String,
    pub status: Option<(String, Hsla)>,
    pub note: Option<String>,
    pub history: Vec<i64>,
    pub lines: Option<(&'static str, Vec<(String, Tone)>)>,
    pub facts: Vec<Fact>,
    pub primary: Vec<PanelAction<A>>,
    pub secondary: Vec<PanelAction<A>>,
}

pub struct Fact {
    pub label: &'static str,
    pub value: String,
    pub tone: Tone,
}

impl Fact {
    pub fn new(label: &'static str, value: impl Into<String>) -> Self {
        Self {
            label,
            value: value.into(),
            tone: Tone::Plain,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn amount(label: &'static str, value: Option<i64>, empty: &str) -> Self {
        match value {
            Some(value) => Self::new(label, number(value)),
            None => Self::new(label, empty).tone(Tone::Muted),
        }
    }
}

pub struct PanelAction<A> {
    pub label: String,
    pub icon: IconName,
    pub action: Option<A>,
    pub danger: bool,
}

impl<A> PanelAction<A> {
    pub fn new(label: impl Into<String>, icon: IconName, action: A) -> Self {
        Self {
            label: label.into(),
            icon,
            action: Some(action),
            danger: false,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        if !enabled {
            self.action = None;
        }
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
}

pub fn render<A: Clone + 'static>(
    model: Option<PanelModel<A>>,
    noun: &str,
    emit: &Emit<A>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Stateful<Div> {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let panel = div()
        .id("item-panel")
        .w(theme.metrics.panel_width)
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .overflow_y_scroll()
        .border_l_1()
        .border_color(colors.border)
        .bg(colors.chrome)
        .text_size(theme.type_scale.body);
    let close = IconButton::new("panel-close", IconName::X, "Close the item panel")
        .on_click(on_close);
    let Some(model) = model else {
        return panel
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pl(px(14.))
                    .pr(px(8.))
                    .pt(px(8.))
                    .child(
                        div()
                            .font_weight(theme.type_scale.label_weight)
                            .text_color(colors.text)
                            .child("Details"),
                    )
                    .child(close),
            )
            .child(
                div()
                    .px(px(14.))
                    .pt(px(6.))
                    .text_color(colors.text_muted)
                    .child(format!(
                        "Select one of the {noun} to see its details and actions."
                    )),
            );
    };

    let header = section(theme)
        .gap(px(5.))
        .pt(px(8.))
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .pt(px(3.))
                        .min_w_0()
                        .text_size(theme.type_scale.title + px(1.))
                        .font_weight(theme.type_scale.title_weight)
                        .text_color(colors.text)
                        .child(model.title),
                )
                .child(div().mr(px(-6.)).child(close)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .truncate()
                        .text_size(theme.type_scale.caption + px(1.))
                        .text_color(colors.text_muted)
                        .child(model.subtitle),
                )
                .children(model.status.map(|(label, color)| {
                    dot_label(color, label)
                        .flex_shrink_0()
                        .text_color(colors.text)
                })),
        )
        .children(model.note.map(|note| {
            div()
                .text_size(theme.type_scale.caption + px(1.))
                .text_color(colors.text_faint)
                .child(note)
        }));

    let facts = model.facts.into_iter().map(|fact| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .h(px(24.))
            .child(div().text_color(colors.text_muted).child(fact.label))
            .child(
                div()
                    .truncate()
                    .font_features(tabular_figures())
                    .text_color(fact.tone.color(theme))
                    .child(fact.value),
            )
    });

    let primary = model
        .primary
        .into_iter()
        .enumerate()
        .map(|(index, action)| {
            let button = text_button(("panel-primary", index), &action.label, cx)
                .w_full()
                .icon(Icon::new(action.icon).size(theme.metrics.icon))
                .disabled(action.action.is_none());
            let button = if index == 0 {
                button.primary()
            } else {
                button
            };
            match action.action {
                Some(action) => button.on_click(emit.on_click(action)),
                None => button,
            }
        });

    let secondary = model
        .secondary
        .into_iter()
        .enumerate()
        .map(|(index, action)| {
            let tone = if action.danger {
                colors.negative
            } else {
                colors.text
            };
            let ring = focus_ring(theme);
            let row = div()
                .id(("panel-action", index))
                .flex()
                .items_center()
                .gap(px(9.))
                .h(px(28.))
                .px(px(8.))
                .rounded(theme.metrics.radius)
                .child(icon(action.icon, theme.metrics.icon, tone.opacity(0.8)))
                .child(action.label);
            match action.action {
                Some(action) => row
                    .cursor_pointer()
                    .text_color(tone)
                    .tab_index(0)
                    .focus_visible(move |style| style.shadow(ring))
                    .hover(|row| row.bg(colors.control_hover))
                    .active(|row| row.bg(colors.control_active))
                    .on_click(emit.on_click(action)),
                None => row.opacity(0.4).text_color(colors.text_muted),
            }
        });

    panel
        .child(header)
        .when(!model.history.is_empty(), |panel| {
            panel.child(
                section(theme)
                    .gap(px(8.))
                    .child(heading(
                        theme,
                        format!(
                            "Price history, {}",
                            counted(model.history.len() as i64, "check", "checks")
                        ),
                    ))
                    .child(price_bars(&model.history, theme)),
            )
        })
        .children(model.lines.map(|(title, lines)| {
            section(theme)
                .gap(px(4.))
                .child(heading(theme, title.to_owned()).pb(px(2.)))
                .children(lines.into_iter().map(|(text, tone)| {
                    div()
                        .font_features(tabular_figures())
                        .text_color(tone.color(theme))
                        .child(text)
                }))
        }))
        .child(section(theme).py(px(8.)).children(facts))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .px(px(14.))
                .pt(px(12.))
                .children(primary),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .px(px(6.))
                .py(px(8.))
                .children(secondary),
        )
}

fn section(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .px(px(14.))
        .py(px(12.))
        .border_b_1()
        .border_color(theme.colors.border)
}

fn heading(theme: &Theme, text: String) -> Div {
    div()
        .text_size(theme.type_scale.caption)
        .font_weight(theme.type_scale.label_weight)
        .text_color(theme.colors.text_faint)
        .child(text)
}

fn price_bars(prices: &[i64], theme: &Theme) -> Div {
    const HEIGHT: f32 = 52.;
    let colors = theme.colors;
    let low = prices.iter().copied().min().unwrap_or(0);
    let high = prices.iter().copied().max().unwrap_or(0);
    let span = (high - low).max(1) as f32;
    let last = prices.len().saturating_sub(1);
    let caption = |label: &'static str, value: i64| {
        div()
            .flex()
            .gap(px(5.))
            .child(div().text_color(colors.text_faint).child(label))
            .child(
                div()
                    .font_features(tabular_figures())
                    .text_color(colors.text_muted)
                    .child(number(value)),
            )
    };
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(div().flex().items_end().gap(px(3.)).h(px(HEIGHT)).children(
            prices.iter().enumerate().map(|(index, price)| {
                let height = 10. + (HEIGHT - 10.) * (*price - low) as f32 / span;
                div()
                    .flex_1()
                    .h(px(height))
                    .rounded_t(px(1.5))
                    .bg(if index == last {
                        colors.accent
                    } else {
                        colors.accent.opacity(0.38)
                    })
            }),
        ))
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(theme.type_scale.caption)
                .child(caption("Low", low))
                .child(caption("High", high))
                .child(caption("Last", prices.last().copied().unwrap_or(0))),
        )
}
