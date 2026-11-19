use std::collections::BTreeMap;

use gpui::{App, Div, Hsla, ParentElement, Stateful, Styled, Window, div, prelude::*, px};

use super::{month_index, year};
use crate::{
    chart::{self, Series, Ticks},
    kit,
    theme::{Theme, tabular_figures},
    ui::{MONTHS, chip, grid::Stat},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Positive,
    Negative,
    Accent,
}

impl Ink {
    fn color(self, theme: &Theme) -> Hsla {
        match self {
            Self::Positive => theme.colors.positive,
            Self::Negative => theme.colors.negative,
            Self::Accent => theme.colors.accent,
        }
    }
}

pub struct Table {
    pub title: &'static str,
    pub name: &'static str,
    pub columns: Vec<&'static str>,
    pub empty: &'static str,
    pub rows: Vec<(String, Vec<String>)>,
}

pub struct Monthly {
    pub title: &'static str,
    pub series: Vec<(&'static str, Ink)>,
    years: BTreeMap<String, Vec<[i64; 12]>>,
}

impl Monthly {
    pub fn new(title: &'static str, series: Vec<(&'static str, Ink)>) -> Self {
        Self {
            title,
            series,
            years: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, time: &str, series: usize, amount: i64) {
        let Some(month) = month_index(time) else {
            return;
        };
        let count = self.series.len();
        let months = self
            .years
            .entry(year(time).to_owned())
            .or_insert_with(|| vec![[0; 12]; count]);
        if let Some(values) = months.get_mut(series) {
            values[month] += amount;
        }
    }

    pub fn years(&self) -> Vec<String> {
        self.years.keys().cloned().collect()
    }

    pub fn total(&self, year: &str, series: usize) -> i64 {
        self.years
            .get(year)
            .and_then(|months| months.get(series))
            .map_or(0, |values| values.iter().sum())
    }
}

pub struct Report {
    pub stats: Vec<Stat>,
    pub tables: Vec<Table>,
    pub monthly: Monthly,
    pub note: Option<String>,
}

fn table(table: &Table, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let value_width = px(92.);
    let header = div()
        .flex()
        .items_center()
        .gap(px(12.))
        .h(theme.metrics.header_height)
        .border_b_1()
        .border_color(colors.border)
        .text_size(theme.type_scale.caption)
        .font_weight(theme.type_scale.label_weight)
        .text_color(colors.text_muted)
        .child(div().flex_1().min_w_0().child(table.name))
        .children(table.columns.iter().map(|column| {
            div()
                .w(value_width)
                .flex_shrink_0()
                .text_right()
                .child(*column)
        }));
    let last = table.rows.len().saturating_sub(1);
    let rows = table.rows.iter().enumerate().map(|(index, (name, values))| {
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .h(px(30.))
            .when(index < last, |row| {
                row.border_b_1().border_color(colors.border)
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(colors.text)
                    .child(name.clone()),
            )
            .children(values.iter().map(|value| {
                div()
                    .w(value_width)
                    .flex_shrink_0()
                    .text_right()
                    .font_features(tabular_figures())
                    .text_color(colors.text_muted)
                    .child(value.clone())
            }))
    });
    kit::card(cx)
        .flex_1()
        .px(px(14.))
        .pt(px(12.))
        .pb(px(6.))
        .gap(px(4.))
        .child(kit::heading(table.title, cx))
        .child(header)
        .children(rows)
        .when(table.rows.is_empty(), |card| {
            card.child(
                div()
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .text_color(colors.text_faint)
                    .child(table.empty),
            )
        })
}

pub fn render(
    id: &'static str,
    report: &Report,
    picked_year: Option<&str>,
    on_year: impl Fn(String, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Stateful<Div> {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let years = report.monthly.years();
    let shown = picked_year
        .filter(|picked| years.iter().any(|year| year == picked))
        .map(str::to_owned)
        .or_else(|| years.last().cloned());
    let on_year = std::rc::Rc::new(on_year);

    let stats = div()
        .flex()
        .flex_shrink_0()
        .gap(px(12.))
        .children(report.stats.iter().map(|stat| {
            kit::stat(
                stat.label,
                stat.value.clone(),
                stat.note.clone(),
                stat.tone,
                cx,
            )
        }));

    let tables = div()
        .flex()
        .flex_shrink_0()
        .items_start()
        .gap(px(12.))
        .children(report.tables.iter().map(|spec| table(spec, cx)));

    let year_chips = div()
        .flex()
        .items_center()
        .gap(px(6.))
        .children(years.iter().enumerate().map(|(index, year)| {
            let selected = shown.as_deref() == Some(year.as_str());
            let (on_year, picked) = (on_year.clone(), year.clone());
            chip((id, index), selected, cx)
                .font_features(tabular_figures())
                .child(year.clone())
                .on_click(move |_, window, cx| on_year(picked.clone(), window, cx))
        }));

    let chart_body = match shown.as_deref() {
        Some(year) => {
            let labels: Vec<String> = MONTHS.iter().map(|month| (*month).to_owned()).collect();
            let series: Vec<Series> = report
                .monthly
                .series
                .iter()
                .enumerate()
                .map(|(index, (name, ink))| {
                    let values = report
                        .monthly
                        .years
                        .get(year)
                        .and_then(|months| months.get(index))
                        .map_or_else(|| vec![0; 12], |values| values.to_vec());
                    Series::new(name, ink.color(theme), values)
                })
                .collect();
            chart::bars(id, &labels, Ticks::FromFirst, &series, 200., cx)
        }
        None => chart::empty("Nothing to chart yet.", 200., cx),
    };
    let totals = shown.as_deref().map(|year| {
        div()
            .flex()
            .items_center()
            .gap(px(16.))
            .text_size(theme.type_scale.caption + px(1.))
            .text_color(colors.text_muted)
            .children(
                report
                    .monthly
                    .series
                    .iter()
                    .enumerate()
                    .map(|(index, (name, _))| {
                        div()
                            .flex()
                            .gap(px(5.))
                            .child(format!("{name} in {year}"))
                            .child(
                                div()
                                    .font_features(tabular_figures())
                                    .text_color(colors.text)
                                    .child(crate::ui::number(report.monthly.total(year, index))),
                            )
                    }),
            )
    });

    kit::scroll_page(id, cx)
        .child(stats)
        .child(tables)
        .child(
            kit::card(cx)
                .flex_shrink_0()
                .gap(px(10.))
                .p(px(14.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .gap(px(16.))
                                .child(kit::heading(report.monthly.title, cx))
                                .children(totals),
                        )
                        .child(year_chips),
                )
                .child(chart_body),
        )
        .children(
            report
                .note
                .clone()
                .map(|note| kit::note(note, cx).flex_shrink_0()),
        )
}
