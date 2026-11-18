//! Pieces the dashboard, chat, settings, about and sign-in screens share.
//! The list screens take theirs from `ui/`.

use gpui::{
    App, ClickEvent, Div, ElementId, Hsla, ParentElement, SharedString, Stateful, Styled, Window,
    div, prelude::*, px,
};
use gpui_component::Icon;
use gpui_kit_assets::IconName;

use crate::{
    data::DataError,
    theme::{Theme, tabular_figures},
    ui::{focus_ring, grid::Tone, icon, placeholder, text_button},
};

/// A backend error as one sentence for a notification or an error state.
pub fn describe(error: &DataError) -> String {
    if error.cause.is_empty() {
        error.message.clone()
    } else {
        format!("{} ({})", error.message, error.cause)
    }
}

/// What a view knows about data it fetched from the backend.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Loaded<T> {
    #[default]
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Loaded<T> {
    pub fn from_result(result: Result<T, DataError>) -> Self {
        match result {
            Ok(value) => Self::Ready(value),
            Err(error) => Self::Failed(describe(&error)),
        }
    }

    pub fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Loading | Self::Failed(_) => None,
        }
    }
}

/// A number with up to two decimals and no trailing zeros, such as `12.5`.
pub fn decimal(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    let whole = crate::ui::number(whole.parse().unwrap_or(0));
    let sign = if value < 0. && !whole.starts_with('-') {
        "-"
    } else {
        ""
    };
    if fraction.is_empty() {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{fraction}")
    }
}

/// A bordered panel on the chrome colour.
pub fn card(cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .rounded(theme.metrics.radius_large)
        .border_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.chrome)
}

/// The title line of a card or a section.
pub fn heading(text: impl Into<SharedString>, cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .text_size(theme.type_scale.body)
        .font_weight(theme.type_scale.title_weight)
        .text_color(theme.colors.text)
        .child(text.into())
}

/// A small label above a group of rows.
pub fn overline(text: impl Into<SharedString>, cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .text_size(theme.type_scale.caption)
        .font_weight(theme.type_scale.label_weight)
        .text_color(theme.colors.text_faint)
        .child(text.into())
}

/// Secondary text under a title or beside a control.
pub fn note(text: impl Into<SharedString>, cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .text_size(theme.type_scale.caption + px(1.))
        .line_height(px(17.))
        .text_color(theme.colors.text_muted)
        .child(text.into())
}

/// A screen body that scrolls when its content is taller than the window.
pub fn scroll_page(id: impl Into<ElementId>, cx: &App) -> Stateful<Div> {
    let theme = Theme::global(cx);
    div()
        .id(id)
        .size_full()
        .flex()
        .flex_col()
        .gap(px(14.))
        .p(theme.metrics.gutter)
        .overflow_y_scroll()
        .bg(theme.colors.surface)
        .text_size(theme.type_scale.body)
        .text_color(theme.colors.text)
}

/// A figure with its label and a line of detail, as the list screens' totals.
pub fn stat(
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    tone: Tone,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    card(cx)
        .flex_1()
        .gap(px(3.))
        .px(px(14.))
        .py(px(10.))
        .child(
            div()
                .text_size(theme.type_scale.caption + px(1.))
                .text_color(colors.text_muted)
                .child(label.into()),
        )
        .child(
            div()
                .text_size(theme.type_scale.figure)
                .font_weight(theme.type_scale.title_weight)
                .font_features(tabular_figures())
                .text_color(tone.color(theme))
                .child(value.into()),
        )
        .child(
            div()
                .truncate()
                .text_size(theme.type_scale.caption + px(1.))
                .text_color(colors.text_faint)
                .child(detail.into()),
        )
}

/// A label and its value on one line.
pub fn fact(
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    tone: Tone,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.))
        .h(px(24.))
        .child(
            div()
                .flex_shrink_0()
                .text_color(theme.colors.text_muted)
                .child(label.into()),
        )
        .child(
            div()
                .truncate()
                .font_features(tabular_figures())
                .text_color(tone.color(theme))
                .child(value.into()),
        )
}

/// The state of a view whose data failed to load, with a button to try again.
pub fn failed(
    title: &str,
    reason: &str,
    on_retry: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Div {
    placeholder(IconName::TriangleAlert, title, reason, cx).child(
        div().pt(px(8.)).child(
            text_button("retry", "Try again", cx)
                .icon(Icon::new(IconName::RotateCcw).size(px(13.)))
                .on_click(on_retry),
        ),
    )
}

/// A clickable line with an icon, a title and a line of detail. A list of
/// them is a menu of places to go, such as the links on the About screen.
pub fn link_row(
    id: impl Into<ElementId>,
    name: IconName,
    title: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    cx: &App,
) -> Stateful<Div> {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let ring = focus_ring(theme);
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(12.))
        .h(px(44.))
        .px(px(14.))
        .cursor_pointer()
        .tab_index(0)
        .focus_visible(move |style| style.shadow(ring))
        .hover(|row| row.bg(colors.control_hover))
        .active(|row| row.bg(colors.control_active))
        .child(icon(name, theme.metrics.icon + px(1.), colors.text_muted))
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
                        .child(title.into()),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(theme.type_scale.caption + px(1.))
                        .text_color(colors.text_muted)
                        .child(detail.into()),
                ),
        )
        .child(icon(
            IconName::ExternalLink,
            theme.metrics.icon - px(2.),
            colors.text_faint,
        ))
}

/// A hairline between the rows of a card.
pub fn rule(cx: &App) -> Div {
    div()
        .flex_shrink_0()
        .h(px(1.))
        .w_full()
        .bg(Theme::global(cx).colors.border)
}

/// A line of text behind an icon in one colour, such as an error under a form.
pub fn notice(name: IconName, tone: Hsla, text: impl Into<SharedString>, cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .flex()
        .items_start()
        .gap(px(8.))
        .child(div().pt(px(2.)).child(icon(name, theme.metrics.icon, tone)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .line_height(px(19.))
                .text_color(tone)
                .child(text.into()),
        )
}

/// The calendar date `days` days before today in UTC, as `YYYY-MM-DD`.
pub fn date_days_ago(days: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    civil_date(now.div_euclid(86_400) - days)
}

/// A Unix time in milliseconds as `YYYY-MM-DD HH:MM:SS` in UTC.
pub fn utc_date_time(millis: i64) -> String {
    let seconds = millis.div_euclid(1000);
    let time = seconds.rem_euclid(86_400);
    format!(
        "{} {:02}:{:02}:{:02}",
        civil_date(seconds.div_euclid(86_400)),
        time / 3600,
        time / 60 % 60,
        time % 60
    )
}

/// The date of a day counted from 1970-01-01, by Howard Hinnant's
/// `civil_from_days`.
fn civil_date(day: i64) -> String {
    let shifted = day + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day_of_month = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day_of_month:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date_matches_known_days() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(19_782), "2024-02-29");
        assert_eq!(civil_date(20_775), "2026-11-18");
        assert_eq!(utc_date_time(1_794_965_045_250), "2026-11-18 01:24:05");
    }

    #[test]
    fn decimal_groups_and_trims() {
        assert_eq!(decimal(1234.5), "1,234.5");
        assert_eq!(decimal(12.0), "12");
        assert_eq!(decimal(-0.25), "-0.25");
        assert_eq!(decimal(-1234.567), "-1,234.57");
    }
}
