//! The table every list screen uses. A screen describes its rows with a
//! [`Catalog`], and [`Grid`] draws them with the shared row states, selection
//! boxes, sorting, row menu and empty, loading and error states.

use std::{
    cmp::Ordering, collections::HashSet, future::Future, marker::PhantomData, path::PathBuf,
    pin::Pin, rc::Rc,
};

use gpui::{
    AnyElement, App, ClickEvent, Context, Div, Edges, IntoElement, ParentElement, Pixels,
    SharedString, Stateful, Styled, Window, div, prelude::*, px,
};
use gpui_component::{
    Icon, Sizable,
    button::ButtonVariants,
    checkbox::Checkbox,
    menu::{DropdownMenu, PopupMenu, PopupMenuItem},
    skeleton::Skeleton,
    table::{Column, ColumnSort, TableDelegate, TableState},
};
use gpui_kit_assets::IconName;

use super::{IconButton, form::FormSpec, form::FormValues, panel::PanelModel, placeholder, text_button};
use crate::{
    data::{DataKind, DataResult, DataSource},
    theme::Theme,
};

/// What a table sends back to the screen that owns it.
#[derive(Clone)]
pub enum Signal<A> {
    Action(A),
    Retry,
    ClearFilters,
}

type SignalHandler<A> = Rc<dyn Fn(Signal<A>, &mut Window, &mut App)>;

/// The callback a table and an item panel use to reach their screen.
pub struct Emit<A>(SignalHandler<A>);

impl<A> Clone for Emit<A> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<A: Clone + 'static> Emit<A> {
    pub fn new(handler: impl Fn(Signal<A>, &mut Window, &mut App) + 'static) -> Self {
        Self(Rc::new(handler))
    }

    pub fn send(&self, signal: Signal<A>, window: &mut Window, cx: &mut App) {
        (self.0)(signal, window, cx);
    }

    /// A click listener that sends `action`. It stops the click there, so a
    /// button inside a row doesn't also select the row.
    pub fn on_click(&self, action: A) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        let emit = self.clone();
        move |_, window, cx| {
            cx.stop_propagation();
            emit.send(Signal::Action(action.clone()), window, cx);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ColumnSpec {
    pub key: &'static str,
    pub title: &'static str,
    pub width: f32,
    /// The one column that takes the width the fixed columns leave over.
    pub flex: bool,
    pub align_end: bool,
    pub sortable: bool,
    /// When the table is narrower than its columns, the column with the
    /// highest number goes first. Zero keeps the column at any width.
    pub drop_order: u8,
}

impl ColumnSpec {
    pub const fn new(key: &'static str, title: &'static str, width: f32) -> Self {
        Self {
            key,
            title,
            width,
            flex: false,
            align_end: false,
            sortable: false,
            drop_order: 0,
        }
    }

    /// Lets the table drop this column when it runs out of width. The item
    /// panel shows the same value for the selected row.
    pub const fn droppable(mut self, order: u8) -> Self {
        self.drop_order = order;
        self
    }

    pub const fn flex(mut self) -> Self {
        self.flex = true;
        self
    }

    pub const fn end(mut self) -> Self {
        self.align_end = true;
        self
    }

    pub const fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

/// One entry of a row's menu, shown on right-click and under the row's
/// overflow button.
pub struct MenuEntry<A> {
    pub label: SharedString,
    pub icon: IconName,
    pub action: A,
    pub separated: bool,
}

impl<A> MenuEntry<A> {
    pub fn new(label: impl Into<SharedString>, icon: IconName, action: A) -> Self {
        Self {
            label: label.into(),
            icon,
            action,
            separated: false,
        }
    }

    /// Draws a separator above this entry.
    pub fn separated(mut self) -> Self {
        self.separated = true;
        self
    }
}

/// A filter chip above the table.
pub struct ChipSpec {
    pub key: String,
    pub label: String,
    pub count: usize,
    pub color: Option<gpui::Hsla>,
    pub tooltip: String,
}

/// Chips of which one at a time filters the table.
pub struct ChipGroup {
    pub id: &'static str,
    pub chips: Vec<ChipSpec>,
}

/// One of the totals panels above the table.
pub struct Stat {
    pub label: &'static str,
    pub value: String,
    pub note: String,
    pub tone: Tone,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Plain,
    Muted,
    Positive,
    Negative,
}

impl Tone {
    pub fn color(self, theme: &Theme) -> gpui::Hsla {
        let colors = &theme.colors;
        match self {
            Self::Plain => colors.text,
            Self::Muted => colors.text_faint,
            Self::Positive => colors.positive,
            Self::Negative => colors.negative,
        }
    }

    /// Green for a gain, red for a loss.
    pub fn of_profit(profit: i64) -> Self {
        match profit {
            0 => Self::Plain,
            profit if profit < 0 => Self::Negative,
            _ => Self::Positive,
        }
    }
}

/// An icon button in the toolbar. `action` is `None` while the button has
/// nothing to act on, for example a bulk action with no row checked.
pub struct ToolbarItem<A> {
    pub id: &'static str,
    pub icon: IconName,
    pub tooltip: String,
    pub action: Option<A>,
    pub danger: bool,
}

/// The labelled button at the right of the toolbar.
pub struct PrimaryAction<A> {
    pub label: &'static str,
    pub icon: IconName,
    pub action: A,
}

/// The search text and the selected chip of each chip group.
#[derive(Clone, Debug, Default)]
pub struct Query {
    pub text: String,
    pub filters: Vec<(&'static str, String)>,
}

impl Query {
    pub fn filter(&self, group: &str) -> Option<&str> {
        self.filters
            .iter()
            .find(|(id, _)| *id == group)
            .map(|(_, key)| key.as_str())
    }

    /// True when `haystack` contains the search text, ignoring case.
    pub fn matches(&self, haystack: &str) -> bool {
        let needle = self.text.trim().to_lowercase();
        needle.is_empty() || haystack.to_lowercase().contains(&needle)
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty() && self.filters.is_empty()
    }
}

/// Turns a submitted form into the action that applies it.
pub type FormMap<A> = Rc<dyn Fn(&FormValues) -> Option<A>>;

/// A backend call in flight. It resolves to the notice to show, or to an
/// empty string when the reloaded list says enough.
pub type Command = Pin<Box<dyn Future<Output = DataResult<String>> + Send>>;

/// An export in flight. It resolves to the file it wrote.
pub type Export = Pin<Box<dyn Future<Output = DataResult<PathBuf>> + Send>>;

/// A load of everything a catalog shows.
pub type Load<D> = Pin<Box<dyn Future<Output = DataResult<D>> + Send>>;

/// Adds a cache list to a load. `fetch` is `None` once the catalog has the
/// list. A failed fetch leaves the list out, so the rows still show and the
/// add dialog falls back to a typed name.
pub fn with_cache<T: Send + 'static, C: Send + 'static>(
    load: Load<T>,
    fetch: Option<impl Future<Output = DataResult<C>> + Send + 'static>,
) -> Load<(T, Option<C>)> {
    Box::pin(async move {
        let data = load.await?;
        let list = match fetch {
            Some(fetch) => fetch.await.ok(),
            None => None,
        };
        Ok((data, list))
    })
}

/// What the screen does after a catalog has handled an action.
pub enum Outcome<A> {
    /// Nothing is left to do.
    Done,
    /// Ask the user for values, then perform the mapped action.
    Ask(FormSpec, FormMap<A>),
    /// Wait for the backend, then load the list again.
    Run(Command),
    /// Ask where to save a file, then run the export to that path.
    Export {
        file_name: String,
        run: Box<dyn FnOnce(PathBuf) -> Command>,
    },
    /// Select the row and open the item panel on it.
    ShowPanel(u64),
    /// Copy the text and say so.
    Copy { text: String, what: String },
    OpenUrl(String),
    /// Tell the user something in a notification.
    Notice(String),
}

impl<A> Outcome<A> {
    pub fn ask(spec: FormSpec, map: impl Fn(&FormValues) -> Option<A> + 'static) -> Self {
        Self::Ask(spec, Rc::new(map))
    }

    /// Runs a backend call and shows `notice` when it succeeds.
    pub fn run<T>(
        call: impl Future<Output = DataResult<T>> + Send + 'static,
        notice: impl Into<String>,
    ) -> Self {
        let notice = notice.into();
        Self::Run(Box::pin(async move { call.await.map(|_| notice) }))
    }

    /// Runs a backend call whose result needs no notice.
    pub fn run_quietly<T>(call: impl Future<Output = DataResult<T>> + Send + 'static) -> Self {
        Self::run(call, String::new())
    }

    /// Asks for a file to save to, then runs the export `call` builds.
    pub fn export(file_name: &str, call: impl FnOnce(PathBuf) -> Export + 'static) -> Self {
        Self::Export {
            file_name: file_name.to_owned(),
            run: Box::new(move |path| {
                let export = call(path);
                Box::pin(async move {
                    export
                        .await
                        .map(|path| format!("Exported the list to {}.", path.display()))
                })
            }),
        }
    }
}

/// The rows of one list screen and everything a trader can do with them.
/// The catalog keeps what the backend last sent and filters it for the
/// table, so a search needs no round trip.
pub trait Catalog: Sized + 'static {
    type Row: Clone + 'static;
    type Action: Clone + 'static;
    /// Everything one load fetches. The default is the empty list.
    type Data: Default + Send + 'static;

    /// The plural noun for the rows, such as "stock items".
    const NOUN: &'static str;
    const SEARCH_PLACEHOLDER: &'static str;
    /// The line shown when the list has no rows at all.
    const EMPTY_HINT: &'static str;
    /// Whether rows carry a selection box for bulk actions.
    const CHECKABLE: bool = true;
    /// The backend lists whose change loads this catalog again.
    const WATCH: &'static [DataKind];

    fn load(&self, source: &DataSource) -> Load<Self::Data>;
    fn apply(&mut self, data: Self::Data);

    fn columns(&self) -> Vec<ColumnSpec>;
    fn rows(&self, query: &Query) -> Vec<Self::Row>;
    fn chips(&self, cx: &App) -> Vec<ChipGroup>;
    fn summary(&self) -> Vec<Stat>;
    fn toolbar(&self, checked: &[u64]) -> Vec<ToolbarItem<Self::Action>>;
    fn primary(&self) -> Option<PrimaryAction<Self::Action>>;
    fn perform(&mut self, action: Self::Action, source: &DataSource) -> Outcome<Self::Action>;

    fn row_id(row: &Self::Row) -> u64;
    fn compare(key: &str, a: &Self::Row, b: &Self::Row) -> Ordering;
    fn cell(row: &Self::Row, key: &str, emit: &Emit<Self::Action>, cx: &App) -> AnyElement;
    /// The cell as plain text, for assistive technology.
    fn cell_text(row: &Self::Row, key: &str) -> Option<String>;
    fn row_menu(row: &Self::Row) -> Vec<MenuEntry<Self::Action>>;
    fn panel(row: &Self::Row, cx: &App) -> PanelModel<Self::Action>;

    /// Rows drawn at reduced opacity, such as hidden stock.
    fn dimmed(_row: &Self::Row) -> bool {
        false
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LoadState {
    #[default]
    Ready,
    Loading,
    Failed(String),
}

const SELECT_KEY: &str = "select";

pub struct Grid<C: Catalog> {
    /// Every column the catalog describes.
    all_specs: Vec<ColumnSpec>,
    /// The columns that fit the available width, and their widths.
    specs: Vec<ColumnSpec>,
    widths: Vec<Pixels>,
    rows: Vec<C::Row>,
    checked: HashSet<u64>,
    /// The id of the highlighted row, so a sort or a reload can find it again.
    selected_id: Option<u64>,
    state: LoadState,
    filtered: bool,
    available_width: Pixels,
    emit: Emit<C::Action>,
    catalog: PhantomData<C>,
}

impl<C: Catalog> Grid<C> {
    pub fn new(
        columns: Vec<ColumnSpec>,
        rows: Vec<C::Row>,
        available_width: Pixels,
        emit: Emit<C::Action>,
    ) -> Self {
        let mut specs = Vec::with_capacity(columns.len() + 1);
        if C::CHECKABLE {
            specs.push(ColumnSpec::new(SELECT_KEY, "", 46.));
        }
        specs.extend(columns);
        let mut grid = Self {
            widths: Vec::new(),
            specs: Vec::new(),
            all_specs: specs,
            rows,
            checked: HashSet::new(),
            selected_id: None,
            state: LoadState::Ready,
            filtered: false,
            available_width,
            emit,
            catalog: PhantomData,
        };
        grid.layout_columns();
        grid
    }

    pub fn set_rows(&mut self, rows: Vec<C::Row>, filtered: bool) {
        self.checked
            .retain(|id| rows.iter().any(|row| C::row_id(row) == *id));
        self.rows = rows;
        self.filtered = filtered;
    }

    pub fn set_state(&mut self, state: LoadState) {
        self.state = state;
    }

    pub fn state(&self) -> &LoadState {
        &self.state
    }

    pub fn selected_id(&self) -> Option<u64> {
        self.selected_id
    }

    pub fn set_selected_id(&mut self, id: Option<u64>) {
        self.selected_id = id;
    }

    pub fn row(&self, row_ix: usize) -> Option<&C::Row> {
        self.rows.get(row_ix)
    }

    pub fn position(&self, id: u64) -> Option<usize> {
        self.rows.iter().position(|row| C::row_id(row) == id)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Checks the row's selection box, or clears it when it is checked.
    pub fn toggle_checked(&mut self, id: u64) {
        if C::CHECKABLE && !self.checked.remove(&id) {
            self.checked.insert(id);
        }
    }

    /// The checked row ids, in the order the rows are shown.
    pub fn checked(&self) -> Vec<u64> {
        self.rows
            .iter()
            .map(C::row_id)
            .filter(|id| self.checked.contains(id))
            .collect()
    }

    pub fn available_width(&self) -> Pixels {
        self.available_width
    }

    pub fn set_available_width(&mut self, width: Pixels) {
        self.available_width = width;
        self.layout_columns();
    }

    /// Drops columns until the rest fit, then gives the flex column the
    /// width the fixed columns leave over.
    fn layout_columns(&mut self) {
        let mut specs = self.all_specs.clone();
        let needed = |specs: &[ColumnSpec]| px(specs.iter().map(|spec| spec.width).sum());
        while needed(&specs) > self.available_width {
            let Some(drop) = specs
                .iter()
                .enumerate()
                .filter(|(_, spec)| spec.drop_order > 0)
                .max_by_key(|(_, spec)| spec.drop_order)
                .map(|(index, _)| index)
            else {
                break;
            };
            specs.remove(drop);
        }
        let fixed: f32 = specs
            .iter()
            .filter(|spec| !spec.flex)
            .map(|spec| spec.width)
            .sum();
        self.widths = specs
            .iter()
            .map(|spec| {
                if spec.flex {
                    (self.available_width - px(fixed)).max(px(spec.width))
                } else {
                    px(spec.width)
                }
            })
            .collect();
        self.specs = specs;
    }

    fn all_checked(&self) -> bool {
        !self.rows.is_empty() && self.checked.len() == self.rows.len()
    }

    fn menu(&self, row: &C::Row, mut menu: PopupMenu) -> PopupMenu {
        for entry in C::row_menu(row) {
            if entry.separated {
                menu = menu.separator();
            }
            let emit = self.emit.clone();
            let action = entry.action;
            menu = menu.item(
                PopupMenuItem::new(entry.label)
                    .icon(Icon::new(entry.icon))
                    .on_click(move |_, window, cx| {
                        emit.send(Signal::Action(action.clone()), window, cx);
                    }),
            );
        }
        menu
    }
}

/// The overflow button at the end of a row. It opens the row's menu.
pub fn overflow_button<A: Clone + 'static>(
    id: u64,
    entries: impl Fn() -> Vec<MenuEntry<A>> + 'static,
    emit: &Emit<A>,
) -> impl IntoElement {
    let emit = emit.clone();
    IconButton::new(("more", id), IconName::Ellipsis, "More actions")
        .in_row()
        .dropdown_menu(move |mut menu, _, _| {
            for entry in entries() {
                if entry.separated {
                    menu = menu.separator();
                }
                let emit = emit.clone();
                let action = entry.action;
                menu = menu.item(
                    PopupMenuItem::new(entry.label)
                        .icon(Icon::new(entry.icon))
                        .on_click(move |_, window, cx| {
                            emit.send(Signal::Action(action.clone()), window, cx);
                        }),
                );
            }
            menu
        })
}

impl DropdownMenu for IconButton {}

/// Fills the cell and centres its content on the row's height.
pub fn centered(content: impl IntoElement) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .min_w_0()
        .child(content)
        .into_any_element()
}

impl<C: Catalog> TableDelegate for Grid<C> {
    fn columns_count(&self, _: &App) -> usize {
        self.specs.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        match self.state {
            LoadState::Ready => self.rows.len(),
            LoadState::Loading | LoadState::Failed(_) => 0,
        }
    }

    fn column(&self, col_ix: usize, cx: &App) -> Column {
        let spec = self.specs[col_ix];
        let metrics = Theme::global(cx).metrics;
        let padding = metrics.cell_padding;
        let column = Column::new(spec.key, spec.title)
            .width(self.widths[col_ix])
            .movable(false)
            .paddings(Edges {
                top: px(0.),
                bottom: px(0.),
                // Without selection boxes the first column starts at the
                // page gutter, in line with the tabs above the table.
                left: if col_ix == 0 && !C::CHECKABLE {
                    metrics.gutter
                } else {
                    padding
                },
                right: padding,
            });
        let column = if spec.sortable {
            column.sortable()
        } else {
            column
        };
        if spec.key == SELECT_KEY || spec.title.is_empty() {
            column.resizable(false).selectable(false)
        } else {
            column
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        let key = self.specs[col_ix].key;
        self.rows.sort_by(|a, b| match sort {
            ColumnSort::Ascending => C::compare(key, a, b),
            ColumnSort::Descending => C::compare(key, a, b).reverse(),
            ColumnSort::Default => C::row_id(a).cmp(&C::row_id(b)),
        });
        if let Some(row_ix) = self.selected_id.and_then(|id| self.position(id)) {
            let table = cx.entity();
            cx.defer(move |cx| {
                table.update(cx, |table, cx| table.set_selected_row(row_ix, cx));
            });
        }
        cx.notify();
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let theme = Theme::global(cx);
        let spec = self.specs[col_ix];
        if spec.key == SELECT_KEY {
            let table = cx.entity();
            return centered(
                Checkbox::new("select-all")
                    .small()
                    .tooltip("Select every row")
                    .checked(self.all_checked())
                    .on_click(move |checked, _, cx| {
                        let checked = *checked;
                        table.update(cx, |table, cx| {
                            let grid = table.delegate_mut();
                            grid.checked = if checked {
                                grid.rows.iter().map(C::row_id).collect()
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
            .when(spec.align_end, |cell| cell.justify_end())
            .text_size(theme.type_scale.caption)
            .font_weight(theme.type_scale.label_weight)
            .text_color(theme.colors.text_muted)
            .child(spec.title)
            .into_any_element()
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        let theme = Theme::global(cx);
        let dimmed = self.rows.get(row_ix).is_some_and(C::dimmed);
        div()
            .id(("row", row_ix))
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text_muted)
            .when(dimmed, |row| row.opacity(0.55))
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        match self.rows.get(row_ix).cloned() {
            Some(row) => self.menu(&row, menu),
            None => menu,
        }
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(row) = self.rows.get(row_ix) else {
            return div().into_any_element();
        };
        let key = self.specs[col_ix].key;
        if key != SELECT_KEY {
            return C::cell(row, key, &self.emit, cx);
        }
        let table = cx.entity();
        let id = C::row_id(row);
        centered(
            // Space checks the selected row, so the boxes stay out of the
            // Tab order and Tab leaves the table in one press.
            Checkbox::new(("select", id))
                .small()
                .tab_stop(false)
                .checked(self.checked.contains(&id))
                .on_click(move |checked, _, cx| {
                    let checked = *checked;
                    table.update(cx, |table, cx| {
                        let boxes = &mut table.delegate_mut().checked;
                        if checked {
                            boxes.insert(id);
                        } else {
                            boxes.remove(&id);
                        }
                        cx.notify();
                    });
                }),
        )
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _: &App) -> Option<SharedString> {
        let row = self.rows.get(row_ix)?;
        C::cell_text(row, self.specs[col_ix].key).map(Into::into)
    }

    fn render_last_empty_col(
        &mut self,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div()
    }

    fn loading(&self, _: &App) -> bool {
        self.state == LoadState::Loading
    }

    fn render_loading(
        &mut self,
        _: gpui_component::Size,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let theme = Theme::global(cx);
        let widths = [0.34, 0.22, 0.28, 0.18, 0.3, 0.24, 0.2, 0.32];
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .h(theme.metrics.header_height)
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(theme.colors.border),
            )
            .children((0..14).map(|row| {
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(24.))
                    .h(theme.metrics.row_height)
                    .px(theme.metrics.gutter)
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(
                        Skeleton::new()
                            .h(px(10.))
                            .w(gpui::relative(widths[row % widths.len()])),
                    )
                    .child(div().flex_1())
                    .children((0..4).map(|column| {
                        Skeleton::new()
                            .h(px(10.))
                            .w(px(48. + 14. * ((row + column) % 3) as f32))
                    }))
            }))
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let colors = Theme::global(cx).colors;
        let emit = self.emit.clone();
        match &self.state {
            LoadState::Failed(reason) => placeholder(
                IconName::TriangleAlert,
                &format!("Could not load the {}", C::NOUN),
                reason,
                cx,
            )
            .child(
                div().pt(px(8.)).child(
                    text_button("retry", "Try again", cx)
                        .icon(Icon::new(IconName::RotateCcw).size(px(13.)))
                        .on_click(move |_, window, cx| emit.send(Signal::Retry, window, cx)),
                ),
            ),
            _ if self.filtered => placeholder(
                IconName::Search,
                &format!("No {} match", C::NOUN),
                "Change the search text or clear the filters to see the full list.",
                cx,
            )
            .child(
                div().pt(px(8.)).child(
                    text_button("clear-filters", "Clear search and filters", cx)
                        .ghost()
                        .text_color(colors.accent)
                        .on_click(move |_, window, cx| {
                            emit.send(Signal::ClearFilters, window, cx);
                        }),
                ),
            ),
            _ => placeholder(
                IconName::Inbox,
                &format!("No {} yet", C::NOUN),
                C::EMPTY_HINT,
                cx,
            ),
        }
    }
}
