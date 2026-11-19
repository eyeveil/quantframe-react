use std::collections::BTreeMap;

use gpui::{App, Div, ParentElement, Stateful, Styled, div, prelude::*, px};

use super::payments::{by_currency, money};
use crate::{
    kit,
    model::{GdprAccount, GdprTradeKind},
    theme::{Theme, tabular_figures},
    ui::{counted, date_time_with_year, date_with_year, grid::Tone, number},
};

fn or_unknown(value: &str) -> String {
    if value.trim().is_empty() {
        "Not in the export".to_owned()
    } else {
        value.to_owned()
    }
}

fn with_signup(current: &str, signup: &str) -> String {
    if signup.is_empty() || signup == current {
        or_unknown(current)
    } else if current.is_empty() {
        format!("{signup} at sign-up")
    } else {
        format!("{current}, {signup} at sign-up")
    }
}

fn profile(account: &GdprAccount, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let date = |value: &Option<String>| {
        value
            .as_deref()
            .map_or_else(|| or_unknown(""), date_with_year)
    };
    let cell = |label: &'static str, value: String| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(theme.type_scale.caption + px(1.))
                    .text_color(colors.text_muted)
                    .child(label),
            )
            .child(
                div()
                    .truncate()
                    .font_features(tabular_figures())
                    .text_color(colors.text)
                    .child(value),
            )
    };
    let row = |cells: [(&'static str, String); 4]| {
        div()
            .flex()
            .gap(px(24.))
            .children(cells.map(|(label, value)| cell(label, value)))
    };
    kit::card(cx)
        .flex_shrink_0()
        .gap(px(12.))
        .px(px(14.))
        .py(px(12.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(theme.type_scale.title + px(1.))
                        .font_weight(theme.type_scale.title_weight)
                        .text_color(colors.text)
                        .child(or_unknown(&account.display_name)),
                )
                .child(
                    div()
                        .text_color(colors.text_muted)
                        .child(or_unknown(&account.email)),
                ),
        )
        .child(kit::rule(cx))
        .child(row([
            ("Member since", date(&account.created_at)),
            ("Last sign-in", date(&account.last_login_at)),
            (
                "Country",
                with_signup(&account.country_code, &account.signup_country_code),
            ),
            (
                "Language",
                with_signup(&account.language, &account.signup_language),
            ),
        ]))
        .child(row([
            ("Signed up through", or_unknown(&account.signup_page)),
            (
                "Account",
                if account.activated {
                    "Activated".to_owned()
                } else {
                    "Not activated".to_owned()
                },
            ),
            (
                "Marketing emails",
                if account.subscribed_to_emails {
                    "Subscribed".to_owned()
                } else {
                    "Not subscribed".to_owned()
                },
            ),
            ("Account ID", or_unknown(&account.oid)),
        ]))
}

fn stats(account: &GdprAccount, cx: &App) -> Div {
    let count = |kind| {
        account
            .trades
            .iter()
            .filter(|trade| trade.kind == kind)
            .count() as i64
    };
    let platinum = |kind| -> i64 {
        account
            .trades
            .iter()
            .filter(|trade| trade.kind == kind)
            .map(|trade| trade.platinum)
            .sum()
    };
    let received = platinum(GdprTradeKind::Sale);
    let given = platinum(GdprTradeKind::Purchase);
    let spent: i64 = account
        .purchases
        .iter()
        .map(|purchase| purchase.platinum)
        .sum();
    let paid = by_currency(&account.payments);
    div()
        .flex()
        .flex_shrink_0()
        .gap(px(12.))
        .child(kit::stat(
            "Trades",
            number(account.trades.len() as i64),
            format!(
                "{}, {}, {}",
                counted(count(GdprTradeKind::Sale), "sale", "sales"),
                counted(count(GdprTradeKind::Purchase), "purchase", "purchases"),
                counted(count(GdprTradeKind::Swap), "swap", "swaps")
            ),
            Tone::Plain,
            cx,
        ))
        .child(kit::stat(
            "Net platinum from trades",
            number(received - given),
            format!("{} received, {} given", number(received), number(given)),
            Tone::of_profit(received - given),
            cx,
        ))
        .child(kit::stat(
            "Platinum spent in the market",
            number(spent),
            counted(account.purchases.len() as i64, "purchase", "purchases"),
            Tone::Plain,
            cx,
        ))
        .child(kit::stat(
            "Paid in real money",
            paid.first().map_or_else(
                || "None".to_owned(),
                |total| format!("{} {}", money(total.total), total.currency),
            ),
            match paid.len() {
                0 | 1 => counted(account.payments.len() as i64, "payment", "payments"),
                more => format!(
                    "{}, {} currencies",
                    counted(account.payments.len() as i64, "payment", "payments"),
                    more
                ),
            },
            Tone::Plain,
            cx,
        ))
}

fn sign_ins(account: &GdprAccount, cx: &App) -> Div {
    const SHOWN: usize = 8;
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let mut logins: Vec<_> = account.logins.iter().collect();
    logins.sort_by(|a, b| b.time.cmp(&a.time));
    let header = |text: &'static str| {
        div()
            .text_size(theme.type_scale.caption)
            .font_weight(theme.type_scale.label_weight)
            .text_color(colors.text_muted)
            .child(text)
    };
    let shown = logins.len().min(SHOWN);
    kit::card(cx)
        .flex_1()
        .px(px(14.))
        .pt(px(12.))
        .pb(px(8.))
        .gap(px(4.))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .child(kit::heading("Latest sign-ins", cx))
                .child(kit::note(
                    if logins.len() > SHOWN {
                        format!("{shown} of {}", number(logins.len() as i64))
                    } else {
                        counted(logins.len() as i64, "sign-in", "sign-ins")
                    },
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .h(theme.metrics.header_height)
                .border_b_1()
                .border_color(colors.border)
                .child(header("Date").w(px(170.)).flex_shrink_0())
                .child(header("Address").flex_1())
                .child(header("Client").w(px(90.)).flex_shrink_0()),
        )
        .children(logins.iter().take(SHOWN).enumerate().map(|(index, login)| {
            div()
                .flex()
                .items_center()
                .gap(px(12.))
                .h(px(30.))
                .when(index + 1 < shown, |row| {
                    row.border_b_1().border_color(colors.border)
                })
                .child(
                    div()
                        .w(px(170.))
                        .flex_shrink_0()
                        .font_features(tabular_figures())
                        .text_color(colors.text)
                        .child(date_time_with_year(&login.time)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_features(tabular_figures())
                        .text_color(colors.text_muted)
                        .child(or_unknown(login.ip.as_deref().unwrap_or_default())),
                )
                .child(
                    div()
                        .w(px(90.))
                        .flex_shrink_0()
                        .truncate()
                        .text_color(colors.text_muted)
                        .child(or_unknown(login.client.as_deref().unwrap_or_default())),
                )
        }))
        .when(logins.is_empty(), |card| {
            card.child(
                div()
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .text_color(colors.text_faint)
                    .child("The export lists no sign-ins."),
            )
        })
}

fn addresses(account: &GdprAccount, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let mut seen: BTreeMap<&str, (i64, &str)> = account
        .ips
        .iter()
        .map(|ip| (ip.as_str(), (0, "")))
        .collect();
    for login in &account.logins {
        if let Some(ip) = login.ip.as_deref() {
            let entry = seen.entry(ip).or_default();
            entry.0 += 1;
            if login.time.as_str() > entry.1 {
                entry.1 = login.time.as_str();
            }
        }
    }
    let mut rows: Vec<_> = seen.into_iter().collect();
    rows.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
    let last = rows.len().saturating_sub(1);
    kit::card(cx)
        .w(px(420.))
        .flex_shrink_0()
        .px(px(14.))
        .pt(px(12.))
        .pb(px(8.))
        .gap(px(4.))
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .child(kit::heading("IP addresses", cx))
                .child(kit::note(
                    counted(rows.len() as i64, "address", "addresses"),
                    cx,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .h(theme.metrics.header_height)
                .border_b_1()
                .border_color(colors.border)
                .text_size(theme.type_scale.caption)
                .font_weight(theme.type_scale.label_weight)
                .text_color(colors.text_muted)
                .child("Address")
                .child("Sign-ins"),
        )
        .children(rows.into_iter().enumerate().map(|(index, (ip, (count, latest)))| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .h(px(30.))
                .when(index < last, |row| {
                    row.border_b_1().border_color(colors.border)
                })
                .child(
                    div()
                        .font_features(tabular_figures())
                        .text_color(colors.text)
                        .child(ip.to_owned()),
                )
                .child(
                    div()
                        .truncate()
                        .font_features(tabular_figures())
                        .text_color(colors.text_muted)
                        .child(if count == 0 {
                            "No sign-in listed".to_owned()
                        } else {
                            format!(
                                "{}, last on {}",
                                counted(count, "sign-in", "sign-ins"),
                                date_with_year(latest)
                            )
                        }),
                )
        }))
}

pub fn render(account: &GdprAccount, cx: &App) -> Stateful<Div> {
    kit::scroll_page("gdpr-overview", cx)
        .child(profile(account, cx))
        .child(stats(account, cx))
        .child(
            div()
                .flex()
                .flex_shrink_0()
                .items_start()
                .gap(px(12.))
                .child(sign_ins(account, cx))
                .child(addresses(account, cx)),
        )
}
