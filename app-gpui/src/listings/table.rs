use std::collections::HashSet;

use gpui::{
    AnyElement, App, Context, Div, Edges, IntoElement, ParentElement, Pixels, Stateful, Styled,
    Window, div, prelude::*, px,
};
use gpui_component::{
    Sizable,
    checkbox::Checkbox,
    table::{Column, ColumnSort, TableDelegate, TableState},
};
use gpui_kit_assets::IconName;

use crate::{
    model::StockItem,
    theme::{ActionStyle, RowLayout, Theme, Variant},
    ui::{IconButton, icon, label_text, number, status_indicator, text_button},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColumnKey {
    Select,
    Item,
    Rank,
    Bought,
    ListPrice,
    MinPrice,
    Owned,
    Status,
    Actions,
}

impl ColumnKey {
    const fn title(self) -> &'static str {
        match self {
            Self::Select => "",
            Self::Item => "Item",
            Self::Rank => "Rank",
            Self::Bought => "Bought",
            Self::ListPrice => "List price",
            Self::MinPrice => "Min price",
            Self::Owned => "Owned",
            Self::Status => "Status",
            Self::Actions => "Actions",
        }
    }

    /// The `sort_by` value the backend's pagination query expects.
    const fn sort_key(self) -> Option<&'static str> {
        match self {
            Self::Item => Some("item_name"),
            Self::Bought => Some("bought"),
            Self::ListPrice => Some("list_price"),
            Self::MinPrice => Some("minimum_price"),
            Self::Owned => Some("owned"),
            Self::Status => Some("status"),
            Self::Select | Self::Rank | Self::Actions => None,
        }
    }

    const fn is_numeric(self) -> bool {
        matches!(self, Self::Bought | Self::ListPrice | Self::Owned)
    }
}

/// The step of the buttons beside a minimum price, in platinum.
const MIN_PRICE_STEP: i64 = 5;

pub struct StockTable {
    rows: Vec<StockItem>,
    selected: HashSet<i64>,
    columns: Vec<(ColumnKey, Pixels)>,
    available_width: Pixels,
}

impl StockTable {
    pub fn new(rows: Vec<StockItem>, available_width: Pixels, cx: &App) -> Self {
        let mut table = Self {
            rows,
            selected: HashSet::new(),
            columns: Vec::new(),
            available_width,
        };
        table.layout_columns(cx);
        table
    }

    pub fn set_rows(&mut self, rows: Vec<StockItem>) {
        self.selected
            .retain(|id| rows.iter().any(|row| row.id == *id));
        self.rows = rows;
    }

    pub fn row(&self, row_ix: usize) -> Option<&StockItem> {
        self.rows.get(row_ix)
    }

    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    pub fn available_width(&self) -> Pixels {
        self.available_width
    }

    pub fn set_available_width(&mut self, width: Pixels, cx: &App) {
        self.available_width = width;
        self.layout_columns(cx);
    }

    /// Fixed widths for every column but the item name, which takes the rest
    /// of the table's width.
    fn layout_columns(&mut self, cx: &App) {
        use ColumnKey::*;
        let fixed: &[(ColumnKey, f32)] = match Theme::global(cx).variant {
            Variant::A => &[
                (Select, 46.),
                (Rank, 104.),
                (Bought, 88.),
                (ListPrice, 104.),
                (MinPrice, 136.),
                (Owned, 78.),
                (Status, 156.),
                (Actions, 206.),
            ],
            Variant::B => &[
                (Select, 34.),
                (Rank, 92.),
                (Bought, 74.),
                (ListPrice, 88.),
                (MinPrice, 116.),
                (Owned, 64.),
                (Status, 132.),
                (Actions, 166.),
            ],
            Variant::C => &[
                (Select, 60.),
                (Bought, 116.),
                (ListPrice, 132.),
                (MinPrice, 176.),
                (Owned, 96.),
                (Status, 196.),
                (Actions, 286.),
            ],
        };
        let fixed_total: f32 = fixed.iter().map(|(_, width)| width).sum();
        let item_width = (self.available_width - px(fixed_total)).max(px(220.));
        self.columns = fixed
            .iter()
            .map(|(key, width)| (*key, px(*width)))
            .collect();
        self.columns.insert(1, (Item, item_width));
    }

    fn key(&self, col_ix: usize) -> ColumnKey {
        self.columns[col_ix].0
    }

    fn all_selected(&self) -> bool {
        !self.rows.is_empty() && self.selected.len() == self.rows.len()
    }

    fn item_cell(&self, item: &StockItem, theme: &Theme) -> AnyElement {
        let name = div()
            .truncate()
            .text_color(theme.colors.text)
            .font_weight(theme.type_scale.label_weight)
            .child(item.item_name.clone());
        match theme.layout.row {
            RowLayout::SingleLine => div()
                .size_full()
                .flex()
                .items_center()
                .gap(px(8.))
                .min_w_0()
                .child(name)
                .when(item.is_hidden, |cell| {
                    cell.child(icon(
                        IconName::EyeOff,
                        theme.metrics.icon - px(2.),
                        theme.colors.text_faint,
                    ))
                })
                .into_any_element(),
            RowLayout::TwoLine => {
                let detail = match item.rank_label() {
                    Some(rank) => format!("{rank}, {}", item.wfm_url),
                    None => item.wfm_url.clone(),
                };
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(2.))
                    .min_w_0()
                    .child(name.text_size(theme.type_scale.emphasis))
                    .child(
                        div()
                            .truncate()
                            .text_size(theme.type_scale.caption)
                            .text_color(theme.colors.text_faint)
                            .child(detail),
                    )
                    .into_any_element()
            }
        }
    }

    fn numeric_cell(&self, value: Option<i64>, empty: &'static str, theme: &Theme) -> AnyElement {
        let cell = div().size_full().flex().items_center().justify_end();
        match value {
            Some(value) => cell
                .font_family(theme.type_scale.numeric_family.clone())
                .text_color(theme.colors.text)
                .child(number(value)),
            None => cell.text_color(theme.colors.text_faint).child(empty),
        }
        .into_any_element()
    }

    fn min_price_cell(&self, item: &StockItem, cx: &mut Context<TableState<Self>>) -> AnyElement {
        let theme = Theme::global(cx);
        let id = item.id;
        let faint = theme.colors.text_faint;
        let value = match item.properties.min_price {
            Some(price) => div()
                .font_family(theme.type_scale.numeric_family.clone())
                .text_color(theme.colors.text)
                .child(number(price)),
            None => div().text_color(faint).child("Not set"),
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
                        IconButton::new(("min-down", id as u64), IconName::Minus, "Lower by 5")
                            .tone(faint)
                            .build(cx)
                            .on_click(cx.listener(move |table, _, _, cx| {
                                cx.stop_propagation();
                                table.delegate_mut().step_min_price(id, -MIN_PRICE_STEP);
                                cx.notify();
                            })),
                    )
                    .child(
                        IconButton::new(("min-up", id as u64), IconName::Plus, "Raise by 5")
                            .tone(faint)
                            .build(cx)
                            .on_click(cx.listener(move |table, _, _, cx| {
                                cx.stop_propagation();
                                table.delegate_mut().step_min_price(id, MIN_PRICE_STEP);
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element()
    }

    fn actions_cell(&self, item: &StockItem, cx: &mut Context<TableState<Self>>) -> AnyElement {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let id = item.id as u64;
        let item_id = item.id;
        let (hide_icon, hide_tooltip) = if item.is_hidden {
            (IconName::EyeOff, "Show on Warframe Market")
        } else {
            (IconName::Eye, "Hide from Warframe Market")
        };
        let sell: AnyElement = match theme.layout.actions {
            ActionStyle::Icons => IconButton::new(("sell", id), IconName::HandCoins, "Sell")
                .tone(colors.accent)
                .hover_tone(colors.accent)
                .build(cx)
                .into_any_element(),
            ActionStyle::LabelledSell => div()
                .pr(px(6.))
                .child(text_button(("sell", id), "Sell", cx))
                .into_any_element(),
        };
        let edit = if item.has_override() {
            IconButton::new(
                ("edit", id),
                IconName::SquarePen,
                "Edit, this item overrides the live scraper settings",
            )
            .tone(colors.warning)
            .hover_tone(colors.warning)
        } else {
            IconButton::new(("edit", id), IconName::SquarePen, "Edit")
        };
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_end()
            .when(item.properties.cooldown.is_some(), |row| {
                row.child(
                    IconButton::new(
                        ("cooldown", id),
                        IconName::Clock,
                        "On cooldown after a price drop",
                    )
                    .tone(colors.warning)
                    .hover_tone(colors.warning)
                    .build(cx),
                )
            })
            .child(sell)
            .child(
                IconButton::new(
                    ("sell-list", id),
                    IconName::Gavel,
                    "Sell one at the list price",
                )
                .disabled(item.list_price.is_none())
                .build(cx),
            )
            .child(IconButton::new(("info", id), IconName::Info, "Details").build(cx))
            .child(edit.build(cx))
            .child(
                IconButton::new(("hide", id), hide_icon, hide_tooltip)
                    .build(cx)
                    .on_click(cx.listener(move |table, _, _, cx| {
                        cx.stop_propagation();
                        table.delegate_mut().toggle_hidden(item_id);
                        cx.notify();
                    })),
            )
            .child(
                IconButton::new(("delete", id), IconName::Trash, "Delete")
                    .hover_tone(colors.negative)
                    .build(cx),
            )
            .into_any_element()
    }

    fn step_min_price(&mut self, id: i64, step: i64) {
        if let Some(item) = self.rows.iter_mut().find(|item| item.id == id) {
            let next = item.properties.min_price.unwrap_or(0) + step;
            item.properties.min_price = (next > 0).then_some(next);
        }
    }

    fn toggle_hidden(&mut self, id: i64) {
        if let Some(item) = self.rows.iter_mut().find(|item| item.id == id) {
            item.is_hidden = !item.is_hidden;
        }
    }
}

/// Fills the cell and centres its content on the row's height.
fn centered(content: impl IntoElement) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .min_w_0()
        .child(content)
        .into_any_element()
}

impl TableDelegate for StockTable {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, cx: &App) -> Column {
        let theme = Theme::global(cx);
        let (key, width) = self.columns[col_ix];
        let padding = theme.metrics.cell_padding;
        let column = Column::new(format!("{key:?}"), key.title())
            .width(width)
            .movable(false)
            .paddings(Edges {
                top: px(0.),
                bottom: px(0.),
                left: padding,
                right: padding,
            });
        let column = if key.sort_key().is_some() {
            column.sortable()
        } else {
            column
        };
        match key {
            ColumnKey::Select => column.resizable(false).selectable(false),
            ColumnKey::Actions => column.resizable(false).selectable(false),
            _ => column,
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        let key = self.key(col_ix);
        self.rows.sort_by(|a, b| {
            let ordering = match key {
                ColumnKey::Bought => a.bought.cmp(&b.bought),
                ColumnKey::ListPrice => a.list_price.cmp(&b.list_price),
                ColumnKey::MinPrice => a.properties.min_price.cmp(&b.properties.min_price),
                ColumnKey::Owned => a.owned.cmp(&b.owned),
                ColumnKey::Status => a.status.label().cmp(b.status.label()),
                _ => a.item_name.cmp(&b.item_name),
            };
            match sort {
                ColumnSort::Descending => ordering.reverse(),
                ColumnSort::Ascending => ordering,
                ColumnSort::Default => a.id.cmp(&b.id),
            }
        });
        cx.notify();
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let theme = Theme::global(cx);
        let key = self.key(col_ix);
        if key == ColumnKey::Select {
            let table = cx.entity();
            return centered(
                Checkbox::new("select-all")
                    .small()
                    .checked(self.all_selected())
                    .on_click(move |checked, _, cx| {
                        let checked = *checked;
                        table.update(cx, |table, cx| {
                            let delegate = table.delegate_mut();
                            delegate.selected = if checked {
                                delegate.rows.iter().map(|row| row.id).collect()
                            } else {
                                HashSet::new()
                            };
                            cx.notify();
                        });
                    }),
            );
        }
        div()
            .size_full()
            .flex()
            .items_center()
            .when(key.is_numeric() || key == ColumnKey::Actions, |cell| {
                cell.justify_end()
            })
            .text_size(theme.type_scale.caption)
            .font_weight(theme.type_scale.label_weight)
            .text_color(theme.colors.text_muted)
            .child(label_text(theme, key.title()))
            .into_any_element()
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        let theme = Theme::global(cx);
        let hidden = self.rows.get(row_ix).is_some_and(|item| item.is_hidden);
        div()
            .id(("row", row_ix))
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text_muted)
            .when(hidden, |row| row.opacity(0.55))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(item) = self.rows.get(row_ix).cloned() else {
            return div().into_any_element();
        };
        let theme = Theme::global(cx).clone();
        match self.key(col_ix) {
            ColumnKey::Select => {
                let table = cx.entity();
                let id = item.id;
                centered(
                    Checkbox::new(("select", id as u64))
                        .small()
                        .checked(self.selected.contains(&id))
                        .on_click(move |checked, _, cx| {
                            let checked = *checked;
                            table.update(cx, |table, cx| {
                                let selected = &mut table.delegate_mut().selected;
                                if checked {
                                    selected.insert(id);
                                } else {
                                    selected.remove(&id);
                                }
                                cx.notify();
                            });
                        }),
                )
            }
            ColumnKey::Item => self.item_cell(&item, &theme),
            ColumnKey::Rank => match item.rank_label() {
                Some(rank) => centered(div().truncate().child(rank)),
                None => div().into_any_element(),
            },
            ColumnKey::Bought => self.numeric_cell(Some(item.bought), "", &theme),
            ColumnKey::ListPrice => self.numeric_cell(item.list_price, "Not listed", &theme),
            ColumnKey::Owned => self.numeric_cell(Some(item.owned), "", &theme),
            ColumnKey::MinPrice => self.min_price_cell(&item, cx),
            ColumnKey::Status => centered(
                div()
                    .min_w_0()
                    .text_color(theme.colors.text)
                    .child(status_indicator(item.status, cx)),
            ),
            ColumnKey::Actions => self.actions_cell(&item, cx),
        }
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _: &App) -> Option<gpui::SharedString> {
        let item = self.rows.get(row_ix)?;
        let text = match self.key(col_ix) {
            ColumnKey::Item => item.item_name.clone(),
            ColumnKey::Rank => item.rank_label()?,
            ColumnKey::Bought => number(item.bought),
            ColumnKey::ListPrice => item.list_price.map(number)?,
            ColumnKey::MinPrice => item.properties.min_price.map(number)?,
            ColumnKey::Owned => number(item.owned),
            ColumnKey::Status => item.status.label().to_owned(),
            ColumnKey::Select | ColumnKey::Actions => return None,
        };
        Some(text.into())
    }

    fn render_last_empty_col(
        &mut self,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div()
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let theme = Theme::global(cx);
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text_muted)
            .child("No stock items match the filter")
    }
}
