mod sample;

use gpui::{
    AnyElement, App, Context, Div, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    Disableable, Icon, Sizable, WindowExt,
    button::ButtonVariants,
    input::{Input, InputEvent, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    notification::Notification,
};
use gpui_kit_assets::IconName;

use crate::{
    data::{Backend, BackendEvent, DataKind, DataResult, DataSource, Trade},
    kit,
    model::{
        DetectedItem, HandleItem, OrderType, StockItem, StockItemListParams, SubType, WishListItem,
    },
    theme::{Theme, tabular_figures},
    ui::{IconButton, counted, day_and_time, icon, number, text_button},
};

const SHEET_WIDTH: f32 = 700.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Sale,
    Purchase,
}

impl Side {
    fn of(trade: &Trade) -> Self {
        if trade.trade_type == "purchase" {
            Self::Purchase
        } else {
            Self::Sale
        }
    }

    const fn order_type(self) -> OrderType {
        match self {
            Self::Sale => OrderType::Sell,
            Self::Purchase => OrderType::Buy,
        }
    }

    const fn noun(self) -> &'static str {
        match self {
            Self::Sale => "sale",
            Self::Purchase => "purchase",
        }
    }

    const fn confirm(self) -> &'static str {
        match self {
            Self::Sale => "Record sale",
            Self::Purchase => "Record purchase",
        }
    }

    fn title(self, player: &str) -> String {
        match (self, player.is_empty()) {
            (Self::Sale, true) => "Sale".to_owned(),
            (Self::Purchase, true) => "Purchase".to_owned(),
            (Self::Sale, false) => format!("Sale to {player}"),
            (Self::Purchase, false) => format!("Purchase from {player}"),
        }
    }

    fn with(self, player: &str) -> String {
        let player = if player.is_empty() {
            "an unnamed player"
        } else {
            player
        };
        match self {
            Self::Sale => format!("sale to {player}"),
            Self::Purchase => format!("purchase from {player}"),
        }
    }
}

struct Line {
    item: DetectedItem,
    price: Entity<InputState>,
    rank: Option<Entity<InputState>>,
    amber: Option<Entity<InputState>>,
    cyan: Option<Entity<InputState>>,
    variant: Option<String>,
}

fn whole(input: &Entity<InputState>, cx: &App) -> Option<i64> {
    input
        .read(cx)
        .value()
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|value| *value >= 0)
}

impl Line {
    fn unit_price(&self, cx: &App) -> Option<i64> {
        whole(&self.price, cx)
    }

    fn total(&self, cx: &App) -> Option<i64> {
        self.unit_price(cx)
            .map(|price| price * self.item.quantity)
    }

    fn sub_type(&self, cx: &App) -> Option<SubType> {
        if !self.item.needs_sub_type {
            return self.item.sub_type.clone();
        }
        let mut sub_type = self.item.sub_type.clone().unwrap_or_default();
        if let Some(rank) = &self.rank {
            sub_type.rank = whole(rank, cx);
        }
        if let Some(amber) = &self.amber {
            sub_type.amber_stars = whole(amber, cx);
        }
        if let Some(cyan) = &self.cyan {
            sub_type.cyan_stars = whole(cyan, cx);
        }
        if self.variant.is_some() {
            sub_type.variant = self.variant.clone();
        }
        Some(sub_type)
    }

    fn problem(&self, cx: &App) -> Option<String> {
        let name = &self.item.name;
        if self.unit_price(cx).is_none() {
            return Some(format!("Enter a whole price of 0 or more for {name}."));
        }
        let within = |input: &Option<Entity<InputState>>, max: Option<i64>| {
            input.as_ref().is_none_or(|input| {
                whole(input, cx).is_some_and(|value| max.is_none_or(|max| value <= max))
            })
        };
        if !within(&self.rank, self.item.max_rank) {
            return Some(match self.item.max_rank {
                Some(max) => format!("Enter a rank from 0 to {max} for {name}."),
                None => format!("Enter a rank for {name}."),
            });
        }
        if !within(&self.amber, self.item.max_amber_stars)
            || !within(&self.cyan, self.item.max_cyan_stars)
        {
            return Some(format!(
                "Enter how many stars {name} has, up to {} amber and {} cyan.",
                self.item.max_amber_stars.unwrap_or(0),
                self.item.max_cyan_stars.unwrap_or(0)
            ));
        }
        None
    }
}

struct Pending {
    id: u64,
    trade: Trade,
    side: Side,
    lines: Vec<Line>,
}

impl Pending {
    fn items_total(&self, cx: &App) -> i64 {
        self.lines.iter().filter_map(|line| line.total(cx)).sum()
    }

    fn handle_items(&self, cx: &App) -> Result<Vec<HandleItem>, String> {
        if let Some(problem) = self.lines.iter().find_map(|line| line.problem(cx)) {
            return Err(problem);
        }
        Ok(self
            .lines
            .iter()
            .map(|line| HandleItem {
                wfm_url: line.item.wfm_url.clone(),
                sub_type: line.sub_type(cx),
                quantity: line.item.quantity,
                price: line.total(cx).unwrap_or(0),
                user_name: self.trade.player_name.clone(),
                order_type: self.side.order_type(),
                operations: vec![
                    format!("SetDate:{}", self.trade.time),
                    if line.item.on_wish_list {
                        "WishList".to_owned()
                    } else {
                        "NotWishList".to_owned()
                    },
                ],
            })
            .collect())
    }
}

pub struct TradeDesk {
    backend: Entity<Backend>,
    source: DataSource,
    queue: Vec<Pending>,
    current: usize,
    next_id: u64,
    editing: bool,
    edit_on_open: bool,
    busy: bool,
    error: Option<String>,
    stock: Vec<StockItem>,
    wishes: Vec<WishListItem>,
    sheet_open: bool,
    _subscriptions: Vec<Subscription>,
}

impl TradeDesk {
    pub fn new(backend: &Entity<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.subscribe_in(
            backend,
            window,
            |desk, _, event: &BackendEvent, window, cx| match event {
                BackendEvent::TradeDetected(trade) => desk.receive(trade.clone(), window, cx),
                BackendEvent::Changed(DataKind::StockItems | DataKind::WishList)
                    if !desk.queue.is_empty() =>
                {
                    desk.load_stock(cx);
                }
                _ => {}
            },
        );
        Self {
            source: backend.read(cx).source.clone(),
            backend: backend.clone(),
            queue: Vec::new(),
            current: 0,
            next_id: 0,
            editing: false,
            edit_on_open: false,
            busy: false,
            error: None,
            stock: Vec::new(),
            wishes: Vec::new(),
            sheet_open: false,
            _subscriptions: vec![subscription],
        }
    }

    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    pub fn preview(&mut self, kinds: &str, edit: bool, cx: &mut Context<Self>) {
        let trades: Vec<BackendEvent> = kinds
            .split(',')
            .filter_map(|kind| sample::payload(kind.trim()))
            .map(|payload| BackendEvent::parse("add_trade", &payload))
            .collect();
        if trades.is_empty() {
            eprintln!("QF_TRADE takes sale, purchase or both with a comma, not {kinds:?}.");
            return;
        }
        self.edit_on_open = edit;
        self.backend.update(cx, |_, cx| {
            for trade in trades {
                cx.emit(trade);
            }
        });
    }

    fn number_input(
        value: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(value.to_string(), window, cx);
            state
        });
        cx.subscribe(&input, |desk, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                desk.error = None;
                cx.notify();
            }
        })
        .detach();
        input
    }

    fn receive(&mut self, trade: Trade, window: &mut Window, cx: &mut Context<Self>) {
        let lines = trade
            .items
            .iter()
            .map(|item| {
                let asks = |limit: Option<i64>| item.needs_sub_type && limit.is_some();
                let sub_type = item.sub_type.clone().unwrap_or_default();
                Line {
                    price: Self::number_input(item.price, window, cx),
                    rank: asks(item.max_rank)
                        .then(|| Self::number_input(sub_type.rank.unwrap_or(0), window, cx)),
                    amber: asks(item.max_amber_stars).then(|| {
                        Self::number_input(sub_type.amber_stars.unwrap_or(0), window, cx)
                    }),
                    cyan: asks(item.max_cyan_stars)
                        .then(|| Self::number_input(sub_type.cyan_stars.unwrap_or(0), window, cx)),
                    variant: if item.needs_sub_type && !item.variants.is_empty() {
                        sub_type
                            .variant
                            .clone()
                            .or_else(|| item.variants.first().cloned())
                    } else {
                        None
                    },
                    item: item.clone(),
                }
            })
            .collect();
        self.next_id += 1;
        self.queue.push(Pending {
            id: self.next_id,
            side: Side::of(&trade),
            trade,
            lines,
        });
        self.load_stock(cx);
        if !self.sheet_open && !window.has_active_dialog(cx) {
            self.open(window, cx);
        }
        cx.notify();
    }

    fn load_stock(&mut self, cx: &mut Context<Self>) {
        let all = StockItemListParams {
            limit: -1,
            ..StockItemListParams::default()
        };
        let stock = self.source.get_stock_item_pagination(&all);
        let wishes = self.source.get_wish_list_pagination(&all);
        cx.spawn(async move |desk, cx| {
            let stock = stock.await;
            let wishes = wishes.await;
            desk.update(cx, |desk, cx| {
                if let Ok(stock) = stock {
                    desk.stock = stock.results;
                }
                if let Ok(wishes) = wishes {
                    desk.wishes = wishes.results;
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.queue.is_empty() || self.sheet_open {
            return;
        }
        self.sheet_open = true;
        self.current = self.current.min(self.queue.len() - 1);
        self.editing = std::mem::take(&mut self.edit_on_open);
        self.error = None;
        let desk = cx.entity();
        window.open_dialog(cx, move |dialog, _, cx| {
            let title = desk.read(cx).title();
            let closed = desk.downgrade();
            let confirm = desk.downgrade();
            dialog
                .w(px(SHEET_WIDTH))
                .title(title)
                .child(desk.clone())
                .on_ok(move |_, window, cx| {
                    confirm
                        .update(cx, |desk, cx| desk.record(window, cx))
                        .ok();
                    false
                })
                .on_close(move |_, _, cx| {
                    closed
                        .update(cx, |desk, cx| {
                            desk.sheet_open = false;
                            desk.editing = false;
                            cx.notify();
                        })
                        .ok();
                })
        });
        if self.editing {
            self.focus_first_price(window, cx);
        }
        cx.notify();
    }

    fn title(&self) -> String {
        self.queue.get(self.current).map_or_else(String::new, |pending| {
            pending.side.title(&pending.trade.player_name)
        })
    }

    fn focus_first_price(&self, window: &mut Window, cx: &mut App) {
        let first = self
            .queue
            .get(self.current)
            .and_then(|pending| pending.lines.first());
        if let Some(line) = first {
            line.price.focus_handle(cx).focus(window, cx);
        }
    }

    fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let last = self.queue.len().saturating_sub(1);
        self.current = if forward {
            (self.current + 1).min(last)
        } else {
            self.current.saturating_sub(1)
        };
        self.editing = false;
        self.error = None;
        cx.notify();
    }

    fn toggle_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing = !self.editing;
        if self.editing {
            self.focus_first_price(window, cx);
        }
        cx.notify();
    }

    fn drop_current(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.current < self.queue.len() {
            self.queue.remove(self.current);
        }
        self.current = self.current.min(self.queue.len().saturating_sub(1));
        self.editing = false;
        self.error = None;
        if self.queue.is_empty() && self.sheet_open {
            self.sheet_open = false;
            window.close_dialog(cx);
        }
        cx.notify();
    }

    fn ignore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pending) = self.queue.get(self.current) else {
            return;
        };
        let notice = format!(
            "Ignored the {}. Nothing was recorded.",
            pending.side.with(&pending.trade.player_name)
        );
        self.drop_current(window, cx);
        window.push_notification(notice, cx);
    }

    fn record(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(pending) = self.queue.get(self.current) else {
            return;
        };
        let items = match pending.handle_items(cx) {
            Ok(items) if items.is_empty() => {
                self.error = Some("This trade has no items Quantframe can record.".to_owned());
                cx.notify();
                return;
            }
            Ok(items) => items,
            Err(problem) => {
                self.error = Some(problem);
                self.editing = true;
                cx.notify();
                return;
            }
        };
        let id = pending.id;
        let done = format!(
            "Recorded {} from the {}.",
            counted(items.len() as i64, "item", "items"),
            pending.side.with(&pending.trade.player_name)
        );
        self.busy = true;
        self.error = None;
        let recorded = self.source.handles_handle_items(items);
        cx.spawn_in(window, async move |desk, cx| {
            let result = recorded.await;
            desk.update_in(cx, |desk, window, cx| {
                desk.recorded(id, done, result, window, cx)
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn recorded(
        &mut self,
        id: u64,
        done: String,
        result: DataResult<i64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.busy = false;
        self.backend.update(cx, |_, cx| {
            for kind in [
                DataKind::StockItems,
                DataKind::WishList,
                DataKind::Transactions,
                DataKind::WfmOrders,
            ] {
                cx.emit(BackendEvent::Changed(kind));
            }
        });
        match result {
            Ok(_) => {
                if let Some(index) = self.queue.iter().position(|pending| pending.id == id) {
                    self.current = index;
                    self.drop_current(window, cx);
                }
                window.push_notification(done, cx);
            }
            Err(error) => {
                let reason = kit::describe(&error);
                self.error = Some(format!(
                    "{reason}. The backend stops at the first item that fails, so the items above it are already recorded."
                ));
                window.push_notification(
                    Notification::error(reason).title("Could not record the trade"),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn stock_change(&self, pending: &Pending, line: &Line, cx: &App) -> (String, bool) {
        let sub_type = line.sub_type(cx);
        let quantity = line.item.quantity;
        let owned = self
            .stock
            .iter()
            .find(|item| item.wfm_url == line.item.wfm_url && item.sub_type == sub_type)
            .map(|item| item.owned);
        match (pending.side, owned) {
            (Side::Sale, Some(owned)) => (
                format!("Stock {} to {}", number(owned), number((owned - quantity).max(0))),
                false,
            ),
            (Side::Sale, None) => ("Not in stock".to_owned(), true),
            (Side::Purchase, _) if line.item.on_wish_list => {
                let wanted = self
                    .wishes
                    .iter()
                    .find(|wish| wish.wfm_url == line.item.wfm_url && wish.sub_type == sub_type)
                    .map(|wish| wish.quantity);
                match wanted {
                    Some(wanted) => (
                        format!(
                            "Wish list {} to {}",
                            number(wanted),
                            number((wanted - quantity).max(0))
                        ),
                        false,
                    ),
                    None => ("Wish list".to_owned(), false),
                }
            }
            (Side::Purchase, owned) => {
                let owned = owned.unwrap_or(0);
                (
                    format!("Stock {} to {}", number(owned), number(owned + quantity)),
                    false,
                )
            }
        }
    }

    fn side_card(&self, title: &'static str, rows: Vec<(String, String)>, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        kit::card(cx)
            .flex_1()
            .gap(px(6.))
            .px(px(12.))
            .py(px(10.))
            .bg(colors.window)
            .child(kit::overline(title, cx))
            .children(rows.into_iter().map(|(name, amount)| {
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .font_weight(theme.type_scale.label_weight)
                            .text_color(colors.text)
                            .child(name),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .font_features(tabular_figures())
                            .text_color(colors.text_muted)
                            .child(amount),
                    )
            }))
    }

    fn exchange(&self, pending: &Pending, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let trade = &pending.trade;
        let items: Vec<(String, String)> = pending
            .lines
            .iter()
            .map(|line| {
                (
                    item_label(&line.item.name, line.sub_type(cx).as_ref()),
                    format!("x {}", number(line.item.quantity)),
                )
            })
            .collect();
        let platinum = vec![("Platinum".to_owned(), number(trade.platinum))];
        let credits = (trade.credits > 0).then(|| ("Credits".to_owned(), number(trade.credits)));
        let (mut gave, received) = match pending.side {
            Side::Sale => (items, platinum),
            Side::Purchase => (platinum, items),
        };
        gave.extend(credits);
        div()
            .flex()
            .items_stretch()
            .gap(px(10.))
            .child(self.side_card("You gave", gave, cx))
            .child(div().flex().items_center().child(icon(
                IconName::ArrowLeftRight,
                theme.metrics.icon + px(1.),
                theme.colors.text_faint,
            )))
            .child(self.side_card("You received", received, cx))
    }

    fn sub_type_editor(&self, index: usize, line: &Line, cx: &Context<Self>) -> Option<Div> {
        if !line.item.needs_sub_type {
            return None;
        }
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let labelled = |label: String, input: &Entity<InputState>| {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(div().text_color(colors.text_muted).child(label))
                .child(div().w(px(56.)).child(Input::new(input).small()))
        };
        let mut row = div()
            .flex()
            .items_center()
            .gap(px(14.))
            .pb(px(8.))
            .text_size(theme.type_scale.caption + px(1.));
        if let Some(rank) = &line.rank {
            row = row.child(labelled(
                format!("Rank, 0 to {}", line.item.max_rank.unwrap_or(0)),
                rank,
            ));
        }
        if let Some(amber) = &line.amber {
            row = row.child(labelled(
                format!("Amber stars, 0 to {}", line.item.max_amber_stars.unwrap_or(0)),
                amber,
            ));
        }
        if let Some(cyan) = &line.cyan {
            row = row.child(labelled(
                format!("Cyan stars, 0 to {}", line.item.max_cyan_stars.unwrap_or(0)),
                cyan,
            ));
        }
        if !line.item.variants.is_empty() {
            let current = line.variant.clone().unwrap_or_default();
            let variants = line.item.variants.clone();
            let desk = cx.entity().downgrade();
            row = row.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().text_color(colors.text_muted).child("Variant"))
                    .child(
                        text_button(("variant", index), &variant_label(&current), cx)
                            .dropdown_caret(true)
                            .dropdown_menu(move |mut menu, _, _| {
                                for variant in &variants {
                                    let (desk, picked) = (desk.clone(), variant.clone());
                                    menu = menu.item(
                                        PopupMenuItem::new(variant_label(variant))
                                            .checked(*variant == current)
                                            .on_click(move |_, _, cx| {
                                                desk.update(cx, |desk, cx| {
                                                    desk.set_variant(index, picked.clone(), cx);
                                                })
                                                .ok();
                                            }),
                                    );
                                }
                                menu
                            }),
                    ),
            );
        }
        Some(row)
    }

    fn set_variant(&mut self, index: usize, variant: String, cx: &mut Context<Self>) {
        let line = self
            .queue
            .get_mut(self.current)
            .and_then(|pending| pending.lines.get_mut(index));
        if let Some(line) = line {
            line.variant = Some(variant);
            cx.notify();
        }
    }

    fn record_table(&self, pending: &Pending, cx: &Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let cell = |width: f32| div().w(px(width)).flex_shrink_0();
        let head = |text: &'static str, width: f32| {
            cell(width)
                .text_size(theme.type_scale.caption)
                .font_weight(theme.type_scale.label_weight)
                .text_color(colors.text_muted)
                .child(text)
        };
        let header = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .h(theme.metrics.header_height)
            .px(px(12.))
            .border_b_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(theme.type_scale.caption)
                    .font_weight(theme.type_scale.label_weight)
                    .text_color(colors.text_muted)
                    .child("Item"),
            )
            .child(head("Quantity", 62.).text_right())
            .child(head("Unit price", 92.).text_right())
            .child(head("Total", 64.).text_right())
            .child(head("Stock change", 128.));
        let last = pending.lines.len().saturating_sub(1);
        let rows: Vec<Div> = pending.lines.iter().enumerate().map(|(index, line)| {
            let (change, missing) = self.stock_change(pending, line, cx);
            let price = if self.editing {
                cell(92.)
                    .child(Input::new(&line.price).small())
                    .into_any_element()
            } else {
                cell(92.)
                    .text_right()
                    .font_features(tabular_figures())
                    .text_color(colors.text)
                    .child(line.unit_price(cx).map_or_else(String::new, number))
                    .into_any_element()
            };
            let editor = if self.editing {
                self.sub_type_editor(index, line, cx)
            } else {
                None
            };
            div()
                .flex()
                .flex_col()
                .px(px(12.))
                .when(index < last, |row| {
                    row.border_b_1().border_color(colors.border)
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .h(theme.metrics.row_height)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .font_weight(theme.type_scale.label_weight)
                                .text_color(colors.text)
                                .child(item_label(&line.item.name, line.sub_type(cx).as_ref())),
                        )
                        .child(
                            cell(62.)
                                .text_right()
                                .font_features(tabular_figures())
                                .text_color(colors.text)
                                .child(number(line.item.quantity)),
                        )
                        .child(price)
                        .child(
                            cell(64.)
                                .text_right()
                                .font_features(tabular_figures())
                                .font_weight(theme.type_scale.label_weight)
                                .text_color(colors.text)
                                .child(line.total(cx).map_or_else(String::new, number)),
                        )
                        .child(
                            cell(128.)
                                .truncate()
                                .font_features(tabular_figures())
                                .text_color(if missing {
                                    colors.text_faint
                                } else {
                                    colors.text_muted
                                })
                                .child(change),
                        ),
                )
                .children(editor)
        })
        .collect();
        kit::card(cx)
            .bg(colors.window)
            .child(header)
            .children(rows)
    }

    fn summary(&self, pending: &Pending, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let total = pending.items_total(cx);
        let traded = pending.trade.platinum;
        let priced = pending
            .lines
            .iter()
            .filter(|line| line.total(cx).is_some_and(|total| total > 0))
            .count() as i64;
        let unpriced = pending.lines.len() as i64 - priced;
        let noun = pending.side.noun();
        let records = format!(
            "{} dated {}.",
            counted(
                priced,
                &format!("{noun} transaction"),
                &format!("{noun} transactions")
            ),
            day_and_time(&pending.trade.time)
        );
        let balance = match total - traded {
            0 => kit::notice(
                IconName::Info,
                colors.text_muted,
                format!("The item prices add up to the {} platinum traded.", number(traded)),
                cx,
            ),
            over if over > 0 => kit::notice(
                IconName::TriangleAlert,
                colors.warning,
                format!(
                    "The item prices add up to {}, which is {} more than the {} platinum traded.",
                    number(total),
                    number(over),
                    number(traded)
                ),
                cx,
            ),
            under => kit::notice(
                IconName::TriangleAlert,
                colors.warning,
                format!(
                    "The item prices add up to {}, which is {} less than the {} platinum traded.",
                    number(total),
                    number(-under),
                    number(traded)
                ),
                cx,
            ),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(balance)
            .when(unpriced > 0, |summary| {
                summary.child(kit::notice(
                    IconName::Info,
                    colors.text_muted,
                    format!(
                        "{} at 0 platinum {} no transaction.",
                        counted(unpriced, "item", "items"),
                        if unpriced == 1 { "gets" } else { "get" }
                    ),
                    cx,
                ))
            })
            .child(
                div()
                    .pl(px(23.))
                    .line_height(px(19.))
                    .text_color(colors.text_muted)
                    .child(format!(
                        "Quantframe records {records} It also updates your matching warframe.market orders."
                    )),
            )
    }

    fn footer(&self, pending: &Pending, cx: &Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let size = theme.metrics.icon - px(2.);
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .pt(px(4.))
            .child(
                text_button("trade-ignore", "Ignore trade", cx)
                    .ghost()
                    .icon(Icon::new(IconName::X).size(size))
                    .disabled(self.busy)
                    .on_click(cx.listener(|desk, _, window, cx| desk.ignore(window, cx))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        text_button(
                            "trade-edit",
                            if self.editing { "Done editing" } else { "Edit" },
                            cx,
                        )
                        .icon(Icon::new(IconName::SquarePen).size(size))
                        .disabled(self.busy || pending.lines.is_empty())
                        .on_click(cx.listener(|desk, _, window, cx| desk.toggle_edit(window, cx))),
                    )
                    .child(
                        text_button("trade-record", pending.side.confirm(), cx)
                            .primary()
                            .loading(self.busy)
                            .disabled(self.busy || pending.lines.is_empty())
                            .on_click(cx.listener(|desk, _, window, cx| desk.record(window, cx))),
                    ),
            )
    }

    fn stepper(&self, cx: &Context<Self>) -> Option<Div> {
        let count = self.queue.len();
        if count < 2 {
            return None;
        }
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let at_first = self.current == 0;
        let at_last = self.current + 1 >= count;
        let previous = IconButton::new("trade-previous", IconName::ChevronLeft, "Previous trade");
        let next = IconButton::new("trade-next", IconName::ChevronRight, "Next trade");
        Some(
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(4.))
                .child(previous.on_click_or_disabled(
                    (!at_first).then(|| cx.listener(|desk, _, _, cx| desk.step(false, cx))),
                ))
                .child(
                    div()
                        .font_features(tabular_figures())
                        .text_color(colors.text_muted)
                        .child(format!("Trade {} of {count}", self.current + 1)),
                )
                .child(next.on_click_or_disabled(
                    (!at_last).then(|| cx.listener(|desk, _, _, cx| desk.step(true, cx))),
                )),
        )
    }
}

fn variant_label(variant: &str) -> String {
    let mut chars = variant.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars)
            .collect::<String>()
            .replace('_', " "),
        None => "Choose".to_owned(),
    }
}

fn item_label(name: &str, sub_type: Option<&SubType>) -> String {
    match sub_type.and_then(SubType::label) {
        Some(label) => format!("{name}, {}", label.to_lowercase()),
        None => name.to_owned(),
    }
}

impl Render for TradeDesk {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let Some(pending) = self.queue.get(self.current) else {
            return div().into_any_element();
        };
        let stepper = self.stepper(cx);
        let error: Option<AnyElement> = self.error.clone().map(|error| {
            kit::notice(IconName::TriangleAlert, colors.negative, error, cx).into_any_element()
        });
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .pt(px(2.))
            .pb(px(4.))
            .text_size(theme.type_scale.body)
            .text_color(colors.text)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .min_h(theme.metrics.action_button)
                    .child(div().text_color(colors.text_muted).child(format!(
                        "Read from Warframe's log on {}",
                        day_and_time(&pending.trade.time)
                    )))
                    .children(stepper),
            )
            .child(self.exchange(pending, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(kit::heading("Quantframe will record", cx))
                    .child(self.record_table(pending, cx)),
            )
            .child(self.summary(pending, cx))
            .children(error)
            .child(self.footer(pending, cx))
            .into_any_element()
    }
}
