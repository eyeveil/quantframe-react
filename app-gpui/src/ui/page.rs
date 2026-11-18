//! One list screen: a toolbar, totals, filter chips, the table and the item
//! panel, all driven by a [`Catalog`]. The screen that owns the page draws
//! [`PageHandle::toolbar`] beside its tabs and [`PageHandle::body`] below.

use std::path::PathBuf;

use gpui::{
    AnyElement, App, ClipboardItem, Context, Div, Entity, Focusable, Global, IntoElement,
    KeyBinding, ParentElement, Pixels, Stateful, Styled, Subscription, Window, actions, div,
    prelude::*, px,
};
use gpui_component::{
    FocusableExt, Icon, Selectable, Sizable, Size, WindowExt,
    button::ButtonVariants,
    input::{InputEvent, InputState},
    notification::Notification,
    skeleton::Skeleton,
    table::{DataTable, TableEvent, TableState},
    tooltip::Tooltip,
};
use gpui_kit_assets::IconName;

use super::{
    IconButton, chip, divider, dot, form,
    grid::{Catalog, ChipGroup, Command, Emit, Grid, LoadState, Outcome, Query, Signal, Stat},
    number, panel, search_field, text_button,
};
use crate::{
    data::{Backend, BackendEvent, DataError, DataSource},
    prefs,
    theme::{Theme, tabular_figures},
};

/// A backend error as one sentence for a notification or the table.
fn describe(error: &DataError) -> String {
    if error.cause.is_empty() {
        error.message.clone()
    } else {
        format!("{} ({})", error.message, error.cause)
    }
}

actions!(
    quantframe,
    [FocusSearch, ShowItemPanel, ToggleItemPanel, ToggleRowCheck]
);

pub const PAGE_CONTEXT: &str = "ListPage";
/// The key context gpui-component gives a focused table.
const TABLE_CONTEXT: &str = "DataTable";

/// Whether the item panel is open. Every list screen shares the setting.
struct ItemPanelOpen(bool);

impl Global for ItemPanelOpen {}

/// Registers the list shortcuts and restores the item panel setting.
/// `QF_PANEL` (`open` or `closed`) takes precedence over the saved setting.
pub fn init(cx: &mut App) {
    let (search, panel) = if cfg!(target_os = "macos") {
        ("cmd-f", "cmd-i")
    } else {
        ("ctrl-f", "ctrl-i")
    };
    cx.bind_keys([
        KeyBinding::new("/", FocusSearch, Some(TABLE_CONTEXT)),
        KeyBinding::new(search, FocusSearch, Some(PAGE_CONTEXT)),
        KeyBinding::new("enter", ShowItemPanel, Some(TABLE_CONTEXT)),
        KeyBinding::new("space", ToggleRowCheck, Some(TABLE_CONTEXT)),
        KeyBinding::new(panel, ToggleItemPanel, Some(PAGE_CONTEXT)),
    ]);
    let open = match std::env::var("QF_PANEL").as_deref() {
        Ok("open") => true,
        Ok("closed") => false,
        _ => prefs::load().item_panel_open,
    };
    cx.set_global(ItemPanelOpen(open));
}

fn panel_open(cx: &App) -> bool {
    cx.global::<ItemPanelOpen>().0
}

pub fn toggle_item_panel(cx: &mut App) {
    set_panel_open(!panel_open(cx), cx);
}

fn set_panel_open(open: bool, cx: &mut App) {
    if panel_open(cx) != open {
        cx.set_global(ItemPanelOpen(open));
        prefs::update(|prefs| prefs.item_panel_open = open);
        cx.refresh_windows();
    }
}

/// A table state forced from the environment, so each one can be rendered
/// and checked: `QF_STATE` is `loading`, `error` or `empty`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ForcedState {
    Loading,
    Failed,
    Empty,
}

impl ForcedState {
    fn from_env() -> Option<Self> {
        match std::env::var("QF_STATE").as_deref() {
            Ok("loading") => Some(Self::Loading),
            Ok("error") => Some(Self::Failed),
            Ok("empty") => Some(Self::Empty),
            _ => None,
        }
    }
}

/// The key that pairs with a letter in a shortcut, as the platform names it.
const fn shortcut_modifier() -> &'static str {
    if cfg!(target_os = "macos") {
        "Command"
    } else {
        "Ctrl"
    }
}

/// What a script asked the page to do once its rows have loaded.
#[derive(Default)]
struct OnLoad {
    row: Option<usize>,
    dialog: Option<String>,
}

pub struct ListPage<C: Catalog> {
    catalog: C,
    source: DataSource,
    search: Entity<InputState>,
    table: Entity<TableState<Grid<C>>>,
    filters: Vec<(&'static str, String)>,
    emit: Emit<C::Action>,
    /// Counts the loads, so a slow answer never replaces a newer one.
    load_generation: u64,
    forced: Option<ForcedState>,
    on_load: Option<OnLoad>,
    _subscriptions: Vec<Subscription>,
}

impl<C: Catalog> ListPage<C> {
    pub fn new(
        catalog: C,
        backend: &Entity<Backend>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let source = backend.read(cx).source.clone();
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder(C::SEARCH_PLACEHOLDER));
        let page = cx.entity().downgrade();
        let emit = Emit::new(move |signal, window, cx| {
            page.update(cx, |page, cx| page.receive(signal, window, cx))
                .ok();
        });
        let table = cx.new(|cx| {
            let mut grid = Grid::new(catalog.columns(), Vec::new(), px(960.), emit.clone());
            grid.set_state(LoadState::Loading);
            TableState::new(grid, window, cx)
                .col_movable(false)
                .col_selectable(false)
        });
        let table_focus = table.focus_handle(cx);
        let subscriptions = vec![
            // The table draws no focus ring, so a table reached with the
            // keyboard shows its focus as a selected row.
            cx.on_focus(&table_focus, window, |page, window, cx| {
                if window.last_input_was_keyboard() {
                    page.select_first_if_none(cx);
                }
            }),
            cx.subscribe_in(&search, window, |page, _, event: &InputEvent, window, cx| {
                match event {
                    InputEvent::Change => page.refresh_rows(cx),
                    InputEvent::PressEnter { .. } => page.focus_table(window, cx),
                    InputEvent::Focus | InputEvent::Blur => cx.notify(),
                }
            }),
            cx.observe(&table, |_, _, cx| cx.notify()),
            cx.subscribe(&table, |page, table, event: &TableEvent, cx| match event {
                TableEvent::SelectRow(row_ix) => {
                    let row_ix = *row_ix;
                    table.update(cx, |table, _| {
                        let grid = table.delegate_mut();
                        let id = grid.row(row_ix).map(C::row_id);
                        grid.set_selected_id(id);
                    });
                    cx.notify();
                }
                TableEvent::ClearSelection => {
                    table.update(cx, |table, _| table.delegate_mut().set_selected_id(None));
                    cx.notify();
                }
                TableEvent::DoubleClickedRow(_) => page.show_panel(cx),
                _ => {}
            }),
            cx.subscribe_in(
                backend,
                window,
                |page, _, event: &BackendEvent, window, cx| {
                    if matches!(event, BackendEvent::Changed(kind) if C::WATCH.contains(kind)) {
                        page.load(window, cx);
                    }
                },
            ),
        ];
        let mut page = Self {
            catalog,
            source,
            search,
            table,
            filters: Vec::new(),
            emit,
            load_generation: 0,
            forced: ForcedState::from_env(),
            on_load: None,
            _subscriptions: subscriptions,
        };
        page.load(window, cx);
        page
    }

    fn set_state(&mut self, state: LoadState, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_state(state);
            cx.notify();
        });
        cx.notify();
    }

    /// Fetches the catalog's data on the backend runtime and shows it when
    /// it arrives. The table keeps its rows while a later load runs.
    fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.forced {
            Some(ForcedState::Loading) => return,
            Some(ForcedState::Failed) => {
                let reason = "The backend didn't answer in time. Check the connection, then try again.";
                return self.set_state(LoadState::Failed(reason.to_owned()), cx);
            }
            Some(ForcedState::Empty) => return self.loaded(C::Data::default(), window, cx),
            None => {}
        }
        self.load_generation += 1;
        let generation = self.load_generation;
        let load = self.catalog.load(&self.source);
        cx.spawn_in(window, async move |page, cx| {
            let result = load.await;
            page.update_in(cx, |page, window, cx| {
                if page.load_generation != generation {
                    return;
                }
                match result {
                    Ok(data) => page.loaded(data, window, cx),
                    Err(error) => page.set_state(LoadState::Failed(describe(&error)), cx),
                }
            })
            .ok();
        })
        .detach();
    }

    fn loaded(&mut self, data: C::Data, window: &mut Window, cx: &mut Context<Self>) {
        self.catalog.apply(data);
        self.set_state(LoadState::Ready, cx);
        self.refresh_rows(cx);
        if let Some(on_load) = self.on_load.take() {
            if let Some(row_ix) = on_load.row {
                self.table.update(cx, |table, cx| {
                    if row_ix < table.delegate().len() {
                        table.set_selected_row(row_ix, cx);
                    }
                });
            }
            if let Some(label) = on_load.dialog {
                self.run_labelled(&label, window, cx);
            }
        }
    }

    fn query(&self, cx: &App) -> Query {
        Query {
            text: self.search.read(cx).value().to_string(),
            filters: self.filters.clone(),
        }
    }

    /// Filters the loaded rows again and keeps the same row selected when it
    /// survives.
    fn refresh_rows(&mut self, cx: &mut Context<Self>) {
        let groups = self.catalog.chips(cx);
        self.filters.retain(|(group, key)| {
            groups
                .iter()
                .any(|chips| chips.id == *group && chips.chips.iter().any(|chip| chip.key == *key))
        });
        let query = self.query(cx);
        let rows = self.catalog.rows(&query);
        let filtered = !query.is_empty();
        self.table.update(cx, |table, cx| {
            let selected = table.delegate().selected_id();
            table.delegate_mut().set_rows(rows, filtered);
            match selected.and_then(|id| table.delegate().position(id)) {
                Some(row_ix) => table.set_selected_row(row_ix, cx),
                None if selected.is_some() => table.clear_selection(cx),
                None => {}
            }
            cx.notify();
        });
        cx.notify();
    }

    fn receive(&mut self, signal: Signal<C::Action>, window: &mut Window, cx: &mut Context<Self>) {
        match signal {
            Signal::Action(action) => self.perform(action, window, cx),
            Signal::Retry => {
                self.forced = None;
                self.set_state(LoadState::Loading, cx);
                self.load(window, cx);
            }
            Signal::ClearFilters => {
                self.filters.clear();
                self.search
                    .update(cx, |search, cx| search.set_value("", window, cx));
                self.refresh_rows(cx);
            }
        }
    }

    fn perform(&mut self, action: C::Action, window: &mut Window, cx: &mut Context<Self>) {
        match self.catalog.perform(action, &self.source) {
            Outcome::Done => self.refresh_rows(cx),
            Outcome::Ask(spec, map) => {
                let page = cx.entity().downgrade();
                form::open(
                    spec,
                    move |values, window, cx| {
                        let Some(action) = map(values) else {
                            return;
                        };
                        page.update(cx, |page, cx| page.perform(action, window, cx))
                            .ok();
                    },
                    window,
                    cx,
                );
            }
            Outcome::Run(command) => self.run(command, window, cx),
            Outcome::Export { file_name, run } => {
                let directory = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
                let chosen = cx.prompt_for_new_path(&directory, Some(&file_name));
                cx.spawn_in(window, async move |page, cx| {
                    let path = match chosen.await {
                        Ok(Ok(Some(path))) => path,
                        Ok(Ok(None)) => return,
                        Ok(Err(_)) | Err(_) => {
                            page.update_in(cx, |_, window, cx| {
                                window.push_notification(
                                    Notification::error("Could not open the save dialog."),
                                    cx,
                                );
                            })
                            .ok();
                            return;
                        }
                    };
                    page.update_in(cx, |page, window, cx| page.run(run(path), window, cx))
                        .ok();
                })
                .detach();
            }
            Outcome::ShowPanel(id) => {
                self.select_id(id, cx);
                self.show_panel(cx);
            }
            Outcome::Copy { text, what } => {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                window.push_notification(format!("Copied {what}."), cx);
            }
            Outcome::OpenUrl(url) => cx.open_url(&url),
            Outcome::Notice(text) => window.push_notification(text, cx),
        }
    }

    /// Waits for a backend call, reports how it went and loads the list again.
    fn run(&mut self, command: Command, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |page, cx| {
            let result = command.await;
            page.update_in(cx, |page, window, cx| {
                match result {
                    Ok(notice) if notice.is_empty() => {}
                    Ok(notice) => window.push_notification(notice, cx),
                    Err(error) => {
                        window.push_notification(Notification::error(describe(&error)), cx);
                    }
                }
                page.load(window, cx);
            })
            .ok();
        })
        .detach();
    }

    fn select_id(&mut self, id: u64, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            if let Some(row_ix) = table.delegate().position(id) {
                table.set_selected_row(row_ix, cx);
            }
        });
    }

    fn selected_row(&self, cx: &App) -> Option<C::Row> {
        let table = self.table.read(cx);
        table
            .selected_row()
            .and_then(|row_ix| table.delegate().row(row_ix).cloned())
    }

    fn show_panel(&mut self, cx: &mut Context<Self>) {
        set_panel_open(true, cx);
        cx.notify();
    }

    fn select_first_if_none(&mut self, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            if table.selected_row().is_none() && table.delegate().len() > 0 {
                table.set_selected_row(0, cx);
            }
        });
    }

    fn focus_table(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.select_first_if_none(cx);
        self.table.focus_handle(cx).focus(window, cx);
    }

    fn set_filter(&mut self, group: &'static str, key: Option<String>, cx: &mut Context<Self>) {
        self.filters.retain(|(id, _)| *id != group);
        if let Some(key) = key {
            self.filters.push((group, key));
        }
        self.refresh_rows(cx);
    }

    fn sync_table_width(&mut self, width: Pixels, cx: &mut Context<Self>) {
        if self.table.read(cx).delegate().available_width() != width {
            self.table.update(cx, |table, cx| {
                table.delegate_mut().set_available_width(width);
                table.refresh(cx);
            });
        }
    }

    fn render_toolbar(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let checked = self.table.read(cx).delegate().checked();
        let items = self.catalog.toolbar(&checked).into_iter().map(|item| {
            let button = IconButton::new(item.id, item.icon, item.tooltip)
                .disabled(item.action.is_none());
            let button = if item.danger {
                button.hover_tone(colors.negative)
            } else {
                button
            };
            match item.action {
                Some(action) => button.on_click(self.emit.on_click(action)),
                None => button,
            }
        });
        let open = panel_open(cx);
        let panel_toggle = IconButton::new(
            "panel-toggle",
            IconName::PanelRight,
            format!(
                "{} the item panel ({}+I)",
                if open { "Hide" } else { "Show" },
                shortcut_modifier()
            ),
        )
        .selected(open)
        .on_click(|_, _, cx| toggle_item_panel(cx));
        let search_tooltip = format!("Search ({}+F, or / from the table)", shortcut_modifier());
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .when(!checked.is_empty(), |row| {
                        row.child(
                            div()
                                .pr(px(8.))
                                .text_size(theme.type_scale.caption + px(1.))
                                .text_color(colors.text_muted)
                                .child(format!("{} selected", checked.len())),
                        )
                    })
                    .children(items)
                    .child(div().px(px(4.)).child(divider(cx)))
                    .child(panel_toggle),
            )
            .child(
                div()
                    .id("search")
                    .tooltip(move |window, cx| {
                        Tooltip::new(search_tooltip.clone()).build(window, cx)
                    })
                    .child(search_field(&self.search, px(240.), window, cx)),
            )
            .children(self.catalog.primary().map(|primary| {
                text_button("primary-action", primary.label, cx)
                    .primary()
                    .icon(Icon::new(primary.icon).size(theme.metrics.icon))
                    .on_click(self.emit.on_click(primary.action))
            }))
            .into_any_element()
    }

    /// The totals panels. While the rows load, bars stand in for the values.
    fn summary(stats: Vec<Stat>, loading: bool, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        div()
            .flex()
            .gap(px(12.))
            .children(stats.into_iter().map(|stat| {
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .px(px(14.))
                    .py(px(10.))
                    .rounded(theme.metrics.radius_large)
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.chrome)
                    .child(
                        div()
                            .text_size(theme.type_scale.caption + px(1.))
                            .text_color(colors.text_muted)
                            .child(stat.label),
                    )
                    .when(loading, |tile| {
                        tile.child(
                            div()
                                .h(px(28.))
                                .flex()
                                .items_center()
                                .child(Skeleton::new().h(px(12.)).w(px(72.))),
                        )
                    })
                    .when(!loading, |tile| tile.child(
                        div()
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(theme.type_scale.figure)
                                    .font_weight(theme.type_scale.title_weight)
                                    .font_features(tabular_figures())
                                    .text_color(stat.tone.color(theme))
                                    .child(stat.value),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme.type_scale.caption)
                                    .text_color(colors.text_faint)
                                    .child(stat.note),
                            ),
                    ))
            }))
    }

    fn chips(&self, groups: Vec<ChipGroup>, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let count = |count: usize| {
            div()
                .font_features(tabular_figures())
                .text_color(theme.colors.text_faint)
                .child(number(count as i64))
        };
        let total = self.catalog.rows(&Query::default()).len();
        let all = chip("chip-all", self.filters.is_empty(), cx)
            .child("All")
            .child(count(total))
            .on_click(cx.listener(|page, _, _, cx| {
                page.filters.clear();
                page.refresh_rows(cx);
            }));
        let mut row = div()
            .id("filter-chips")
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.))
            .min_w_0()
            .child(all);
        for (group_ix, group) in groups.into_iter().enumerate() {
            if group_ix > 0 && !group.chips.is_empty() {
                row = row.child(div().px(px(4.)).child(divider(cx)));
            }
            let group_id = group.id;
            let active = self
                .filters
                .iter()
                .find(|(id, _)| *id == group_id)
                .map(|(_, key)| key.clone());
            for spec in group.chips {
                let selected = active.as_deref() == Some(spec.key.as_str());
                let key = spec.key.clone();
                let tooltip = spec.tooltip;
                row = row.child(
                    chip(
                        gpui::SharedString::from(format!("{group_id}-{}", spec.key)),
                        selected,
                        cx,
                    )
                    .children(spec.color.map(|color| dot(color, px(6.))))
                    .child(spec.label)
                    .child(count(spec.count))
                    .when(!tooltip.is_empty(), |chip| {
                        chip.tooltip(move |window, cx| {
                            Tooltip::new(tooltip.clone()).build(window, cx)
                        })
                    })
                    .on_click(cx.listener(move |page, _, _, cx| {
                        let next = (!selected).then(|| key.clone());
                        page.set_filter(group_id, next, cx);
                    })),
                );
            }
        }
        row
    }

    fn render_body(&mut self, width: Pixels, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let gutter = theme.metrics.gutter;
        let open = panel_open(cx);
        let table_width = if open {
            width - theme.metrics.panel_width
        } else {
            width
        };
        self.sync_table_width(table_width, cx);

        let state = self.table.read(cx).delegate().state().clone();
        let loading = state == LoadState::Loading;
        let ready = state == LoadState::Ready;
        let summary = match state {
            LoadState::Failed(_) => Vec::new(),
            LoadState::Ready | LoadState::Loading => self.catalog.summary(),
        };
        let groups = if ready {
            self.catalog.chips(cx)
        } else {
            Vec::new()
        };
        let has_chips = groups.iter().any(|group| !group.chips.is_empty());
        let list = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .when(!summary.is_empty(), |list| {
                list.child(
                    div()
                        .flex_shrink_0()
                        .px(gutter)
                        .pt(px(14.))
                        .child(Self::summary(summary, loading, cx)),
                )
            })
            .child(if has_chips {
                div()
                    .flex_shrink_0()
                    .px(gutter)
                    .py(px(12.))
                    .child(self.chips(groups, cx))
            } else {
                div().flex_shrink_0().h(px(14.))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .border_t_1()
                    .border_color(colors.border)
                    .child(
                        DataTable::new(&self.table)
                            .focus_ring(false)
                            .with_size(Size::Size(theme.metrics.row_height))
                            .header_row_height(theme.metrics.header_height)
                            .stripe(false)
                            .bordered(false)
                            .accessibility_label(C::NOUN),
                    ),
            );

        div()
            .size_full()
            .flex()
            .on_action(cx.listener(|page, _: &ShowItemPanel, _, cx| {
                if page.selected_row(cx).is_some() {
                    page.show_panel(cx);
                }
            }))
            .on_action(cx.listener(|page, _: &ToggleRowCheck, _, cx| {
                page.table.update(cx, |table, cx| {
                    if let Some(id) = table.delegate().selected_id() {
                        table.delegate_mut().toggle_checked(id);
                        cx.notify();
                    }
                });
            }))
            .child(list)
            .when(open, |body| {
                let model = self.selected_row(cx).map(|row| C::panel(&row, cx));
                body.child(panel::render(
                    model,
                    C::NOUN,
                    &self.emit,
                    |_, _, cx| set_panel_open(false, cx),
                    cx,
                ))
            })
            .into_any_element()
    }

    /// Performs the action behind a button label, for scripted screenshots:
    /// the toolbar's labelled button, or an entry of a row's menu. The row is
    /// the selected one, or the first.
    fn run_labelled(&mut self, label: &str, window: &mut Window, cx: &mut Context<Self>) {
        let matches = |candidate: &str| candidate.eq_ignore_ascii_case(label.trim());
        let action = match self.catalog.primary() {
            Some(primary) if matches(primary.label) => Some(primary.action),
            _ => self
                .selected_row(cx)
                .or_else(|| self.table.read(cx).delegate().row(0).cloned())
                .and_then(|row| {
                    C::row_menu(&row)
                        .into_iter()
                        .find(|entry| matches(&entry.label))
                })
                .map(|entry| entry.action),
        };
        match action {
            Some(action) => self.perform(action, window, cx),
            None => eprintln!("No action is labelled {label:?} on this screen."),
        }
    }
}

/// What a screen needs from one of its pages, whatever its catalog.
pub trait PageHandle {
    fn toolbar(&self, window: &mut Window, cx: &mut App) -> AnyElement;
    fn body(&self, width: Pixels, window: &mut Window, cx: &mut App) -> AnyElement;
    /// The number of rows before any search or filter, once they have loaded.
    fn count(&self, cx: &App) -> Option<usize>;
    fn focus(&self, window: &mut Window, cx: &mut App);
    fn focus_search(&self, window: &mut Window, cx: &mut App);
    /// Selects a row and opens the prompt behind a button label once the
    /// rows have loaded. Scripted screenshots use it.
    fn on_load(&self, row: Option<usize>, dialog: Option<String>, cx: &mut App);
}

impl<C: Catalog> PageHandle for Entity<ListPage<C>> {
    fn toolbar(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.update(cx, |page, cx| page.render_toolbar(window, cx))
    }

    fn body(&self, width: Pixels, window: &mut Window, cx: &mut App) -> AnyElement {
        self.update(cx, |page, cx| page.render_body(width, window, cx))
    }

    fn count(&self, cx: &App) -> Option<usize> {
        let page = self.read(cx);
        (*page.table.read(cx).delegate().state() == LoadState::Ready)
            .then(|| page.catalog.rows(&Query::default()).len())
    }

    fn focus_search(&self, window: &mut Window, cx: &mut App) {
        self.read(cx).search.focus_handle(cx).focus(window, cx);
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        self.read(cx).table.focus_handle(cx).focus(window, cx);
    }

    fn on_load(&self, row: Option<usize>, dialog: Option<String>, cx: &mut App) {
        self.update(cx, |page, _| page.on_load = Some(OnLoad { row, dialog }));
    }
}
