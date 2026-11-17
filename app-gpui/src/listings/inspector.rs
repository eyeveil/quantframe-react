use gpui::{App, Div, Hsla, ParentElement, Styled, div, px};
use gpui_component::{Icon, button::ButtonVariants};
use gpui_kit_assets::IconName;

use crate::{
    model::StockItem,
    theme::Theme,
    ui::{label_text, number, status_indicator, text_button},
};

/// The panel beside the table that shows the selected stock item.
pub fn panel(item: Option<&StockItem>, cx: &App) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let Some(width) = theme.layout.inspector_width else {
        return div();
    };
    let panel = div()
        .w(width)
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .border_l_1()
        .border_color(colors.border)
        .bg(colors.chrome)
        .text_size(theme.type_scale.body);
    let Some(item) = item else {
        return panel.child(
            div()
                .p(px(16.))
                .text_color(colors.text_muted)
                .child("Select an item to see its prices and actions."),
        );
    };

    let detail = match item.rank_label() {
        Some(rank) => format!("{rank}, {}", item.wfm_url),
        None => item.wfm_url.clone(),
    };
    let profit = item.profit_per_unit();
    let profit_color = match profit {
        Some(profit) if profit < 0 => colors.negative,
        Some(_) => colors.positive,
        None => colors.text_faint,
    };
    let optional = |value: Option<i64>, empty: &str| match value {
        Some(value) => (number(value), colors.text),
        None => (empty.to_owned(), colors.text_faint),
    };
    let (list_price, list_color) = optional(item.list_price, "Not listed");
    let (min_price, min_color) = optional(item.properties.min_price, "Not set");
    let (stock_value, stock_color) = optional(
        item.list_price.map(|price| price * item.owned),
        "Not listed",
    );

    panel
        .child(
            section(theme)
                .gap(px(6.))
                .child(
                    div()
                        .text_size(theme.type_scale.figure)
                        .font_weight(theme.type_scale.title_weight)
                        .text_color(colors.text)
                        .child(item.item_name.clone()),
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
                                .font_family(theme.type_scale.numeric_family.clone())
                                .text_size(theme.type_scale.caption)
                                .text_color(colors.text_muted)
                                .child(detail),
                        )
                        .child(status_indicator(item.status, cx)),
                )
                .child(
                    div()
                        .text_size(theme.type_scale.caption)
                        .text_color(colors.text_faint)
                        .child(item.status.detail()),
                ),
        )
        .child(
            section(theme)
                .gap(px(8.))
                .child(heading(
                    theme,
                    &format!("Price history, {} checks", item.price_history.len()),
                ))
                .child(price_bars(item, theme)),
        )
        .child(
            section(theme)
                .gap(px(0.))
                .child(fact(theme, "Bought", number(item.bought), colors.text))
                .child(fact(theme, "List price", list_price, list_color))
                .child(fact(theme, "Min price", min_price, min_color))
                .child(fact(
                    theme,
                    "Profit per unit",
                    profit.map_or("Not listed".to_owned(), number),
                    profit_color,
                ))
                .child(fact(theme, "Owned", number(item.owned), colors.text))
                .child(fact(theme, "Stock value", stock_value, stock_color))
                .child(fact(
                    theme,
                    "Last checked",
                    item.updated_at.get(11..16).unwrap_or("").to_owned(),
                    colors.text_muted,
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .p(px(12.))
                .child(
                    text_button("panel-sell", "Sell", cx)
                        .primary()
                        .w_full()
                        .icon(Icon::new(IconName::HandCoins).size(theme.metrics.icon)),
                )
                .child(
                    text_button("panel-sell-list", "Sell one at the list price", cx)
                        .w_full()
                        .icon(Icon::new(IconName::Gavel).size(theme.metrics.icon)),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(text_button("panel-edit", "Edit", cx).flex_1())
                        .child(
                            text_button(
                                "panel-hide",
                                if item.is_hidden { "Show" } else { "Hide" },
                                cx,
                            )
                            .flex_1(),
                        )
                        .child(text_button("panel-delete", "Delete", cx).danger().flex_1()),
                ),
        )
}

fn section(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .p(px(12.))
        .border_b_1()
        .border_color(theme.colors.border)
}

fn heading(theme: &Theme, text: &str) -> Div {
    div()
        .text_size(theme.type_scale.caption)
        .font_weight(theme.type_scale.label_weight)
        .text_color(theme.colors.text_faint)
        .child(label_text(theme, text))
}

fn fact(theme: &Theme, label: &'static str, value: String, color: Hsla) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(24.))
        .child(div().text_color(theme.colors.text_muted).child(label))
        .child(
            div()
                .font_family(theme.type_scale.numeric_family.clone())
                .text_color(color)
                .child(value),
        )
}

/// One bar per price check, scaled between the lowest and highest price.
fn price_bars(item: &StockItem, theme: &Theme) -> Div {
    const HEIGHT: f32 = 52.;
    let colors = theme.colors;
    let prices: Vec<i64> = item.price_history.iter().map(|entry| entry.price).collect();
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
                    .font_family(theme.type_scale.numeric_family.clone())
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
