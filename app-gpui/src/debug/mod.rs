use std::cmp::Ordering;

use gpui::{
    AnyElement, App, AppContext, Context, Div, Entity, IntoElement, ParentElement, Pixels, Styled,
    Window, div, px,
};
use gpui_component::{Icon, WindowExt, notification::Notification};
use gpui_kit_assets::IconName;

use crate::{
    data::{Backend, DataKind, DataSource},
    kit::{self, Loaded},
    model::{AppInfo, EeLogLine, EeLogParams, LiveScraperState, Settings, WfmDebugState},
    theme::{Theme, tabular_figures},
    ui::{
        IconButton, cells,
        grid::{
            Catalog, ChipGroup, ColumnSpec, Emit, Load, MenuEntry, Outcome, PrimaryAction, Query,
            Stat, Tone, ToolbarItem, overflow_button,
        },
        number,
        page::{ListPage, PageHandle},
        panel::{Fact, PanelAction, PanelModel},
        screen::TabbedScreen,
        text_button,
    },
};

const LOG_LINES: i64 = 1000;

pub fn screen(backend: &Entity<Backend>, window: &mut Window, cx: &mut App) -> Entity<TabbedScreen> {
    cx.new(|cx| {
        let mut screen = TabbedScreen::new("debug-tab");
        let state = cx.new(|cx| StateView::new(backend, cx));
        screen.add_view("state", "State", state, cx);
        let page = cx.new(|cx| ListPage::new(GameLog::default(), backend, window, cx));
        screen.add("game-log", "Warframe log", page, cx);
        screen
    })
}

#[derive(Clone, Debug)]
struct Snapshot {
    info: AppInfo,
    wfm: WfmDebugState,
    scraper: LiveScraperState,
    settings: Settings,
}

pub struct StateView {
    source: DataSource,
    snapshot: Loaded<Snapshot>,
}

impl StateView {
    fn new(backend: &Entity<Backend>, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            source: backend.read(cx).source.clone(),
            snapshot: Loaded::Loading,
        };
        view.load(cx);
        view
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let info = self.source.app_get_app_info();
        let wfm = self.source.debug_get_wfm_state();
        let scraper = self.source.live_scraper_get_state();
        let settings = self.source.app_get_settings();
        cx.spawn(async move |view, cx| {
            let snapshot = async {
                Ok(Snapshot {
                    info: info.await?,
                    wfm: wfm.await?,
                    scraper: scraper.await?,
                    settings: settings.await?,
                })
            }
            .await;
            view.update(cx, |view, cx| {
                view.snapshot = Loaded::from_result(snapshot);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn export_logs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let exported = self.source.log_export();
        cx.spawn_in(window, async move |_, cx| {
            let exported = exported.await;
            cx.update(|window, cx| match exported {
                Ok(path) => {
                    window.push_notification(format!("Wrote the logs to {}.", path.display()), cx);
                }
                Err(error) => window.push_notification(
                    Notification::error(kit::describe(&error)).title("Could not export the logs"),
                    cx,
                ),
            })
            .ok();
        })
        .detach();
    }

    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let page = kit::scroll_page("debug-state", cx);
        let snapshot = match &self.snapshot {
            Loaded::Loading => {
                return page
                    .child(kit::note("Reading the app's state.", cx))
                    .into_any_element();
            }
            Loaded::Failed(reason) => {
                return page
                    .child(kit::failed(
                        "Could not read the app's state",
                        reason,
                        cx.listener(|view, _, _, cx| view.load(cx)),
                        cx,
                    ))
                    .into_any_element();
            }
            Loaded::Ready(snapshot) => snapshot,
        };
        let yes_no = |flag: bool| if flag { "Yes" } else { "No" };
        let (info, wfm, settings) = (&snapshot.info, &snapshot.wfm, &snapshot.settings);
        let facts = |title: &'static str, facts: Vec<(&'static str, String)>| {
            kit::card(cx)
                .flex_1()
                .px(px(14.))
                .py(px(10.))
                .child(div().pb(px(4.)).child(kit::heading(title, cx)))
                .children(
                    facts
                        .into_iter()
                        .map(|(label, value)| kit::fact(label, value, Tone::Plain, cx)),
                )
        };
        let scraper = &settings.live_scraper.general;
        page.child(
            div()
                .flex()
                .flex_shrink_0()
                .items_start()
                .gap(px(12.))
                .child(facts(
                    "App",
                    vec![
                        ("Name", info.name.clone()),
                        ("Version", info.version.clone()),
                        ("Development build", yes_no(info.is_dev).to_owned()),
                        ("Pre-release", yes_no(info.is_pre_release).to_owned()),
                        ("Fixture mode", yes_no(info.fixture_mode).to_owned()),
                        ("Temporary database", yes_no(info.use_temp_db).to_owned()),
                    ],
                ))
                .child(facts(
                    "warframe.market session",
                    vec![
                        ("Sell orders", number(wfm.sell_orders as i64)),
                        ("Buy orders", number(wfm.buy_orders as i64)),
                        ("Auctions", number(wfm.auctions as i64)),
                        ("Order limit", number(wfm.order_limit)),
                    ],
                ))
                .child(facts(
                    "Live scraper",
                    vec![
                        ("Running", yes_no(snapshot.scraper.is_running).to_owned()),
                        (
                            "Trade modes",
                            scraper
                                .trade_modes
                                .iter()
                                .map(|mode| crate::settings::trade_mode_label(&mode.to_string()))
                                .collect::<Vec<_>>()
                                .join(", "),
                        ),
                        ("Records trades from the game", yes_no(scraper.auto_trade).to_owned()),
                        (
                            "Reads orders from files",
                            yes_no(settings.debugging.live_scraper.fake_orders).to_owned(),
                        ),
                    ],
                )),
        )
        .child(
            div()
                .flex()
                .flex_shrink_0()
                .items_start()
                .gap(px(12.))
                .child(table(
                    "Requests this session",
                    "No request has gone to warframe.market yet.",
                    &[("Route", Width::Fill), ("Requests", Width::Fixed(90.))],
                    wfm.tracking
                        .iter()
                        .map(|(route, count)| vec![route.clone(), number(*count as i64)])
                        .collect(),
                    cx,
                ))
                .child(table(
                    "Rate limits",
                    "No route has a rate limit yet.",
                    &[
                        ("Route", Width::Fill),
                        ("Limit", Width::Fixed(70.)),
                        ("Wait", Width::Fixed(80.)),
                        ("Quota", Width::Fixed(90.)),
                    ],
                    wfm.limiters
                        .iter()
                        .map(|limiter| {
                            vec![
                                limiter.route.clone(),
                                number(limiter.limit),
                                format!("{} s", kit::decimal(limiter.wait_time_sec)),
                                limiter.quota_type.clone(),
                            ]
                        })
                        .collect(),
                    cx,
                )),
        )
        .into_any_element()
    }
}

#[derive(Clone, Copy)]
enum Width {
    Fill,
    Fixed(f32),
}

fn table(
    title: &'static str,
    empty: &'static str,
    columns: &[(&'static str, Width)],
    rows: Vec<Vec<String>>,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let cell = |text: String, width: Width| match width {
        Width::Fill => div().flex_1().min_w_0().truncate().child(text),
        Width::Fixed(width) => div()
            .w(px(width))
            .flex_shrink_0()
            .flex()
            .justify_end()
            .font_features(tabular_figures())
            .child(text),
    };
    let card = kit::card(cx).flex_1().child(
        div()
            .px(px(14.))
            .h(px(40.))
            .flex()
            .items_center()
            .child(kit::heading(title, cx)),
    );
    if rows.is_empty() {
        return card.child(
            div()
                .px(px(14.))
                .pb(px(12.))
                .text_color(colors.text_faint)
                .child(empty),
        );
    }
    card.child(
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .h(theme.metrics.header_height)
            .px(px(14.))
            .border_t_1()
            .border_color(colors.border)
            .text_size(theme.type_scale.caption)
            .font_weight(theme.type_scale.label_weight)
            .text_color(colors.text_muted)
            .children(
                columns
                    .iter()
                    .map(|(label, width)| cell((*label).to_owned(), *width)),
            ),
    )
    .children(rows.into_iter().map(|row| {
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .h(px(30.))
            .px(px(14.))
            .border_t_1()
            .border_color(colors.border)
            .text_color(colors.text_muted)
            .children(
                row.into_iter()
                    .zip(columns)
                    .map(|(text, (_, width))| cell(text, *width)),
            )
    }))
}

impl PageHandle for Entity<StateView> {
    fn toolbar(&self, _: &mut Window, cx: &mut App) -> AnyElement {
        let reload = self.downgrade();
        let export = self.downgrade();
        let size = Theme::global(cx).metrics.icon;
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                IconButton::new("state-reload", IconName::RefreshCw, "Read the state again")
                    .on_click(move |_, _, cx| {
                        reload.update(cx, |view, cx| view.load(cx)).ok();
                    }),
            )
            .child(
                text_button("state-export", "Export logs", cx)
                    .icon(Icon::new(IconName::Download).size(size))
                    .on_click(move |_, window, cx| {
                        export
                            .update(cx, |view, cx| view.export_logs(window, cx))
                            .ok();
                    }),
            )
            .into_any_element()
    }

    fn body(&self, _: Pixels, _: &mut Window, cx: &mut App) -> AnyElement {
        self.update(cx, |view, cx| view.render_body(cx))
    }

    fn count(&self, _: &App) -> Option<usize> {
        None
    }

    fn focus(&self, _: &mut Window, _: &mut App) {}

    fn focus_search(&self, _: &mut Window, _: &mut App) {}

    fn on_load(&self, _: Option<usize>, _: Option<String>, _: &mut App) {}
}

#[derive(Clone, Debug)]
pub enum LogAction {
    Reload,
    ToggleEmpty,
    Export,
    Details(u64),
    CopyLine(u64),
}

pub struct GameLog {
    lines: Vec<EeLogLine>,
    total: i64,
    hide_empty: bool,
}

impl Default for GameLog {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            total: 0,
            hide_empty: true,
        }
    }
}

impl GameLog {
    fn params(&self) -> EeLogParams {
        EeLogParams {
            limit: LOG_LINES,
            hide_empty: self.hide_empty,
            ..EeLogParams::default()
        }
    }

    fn line(&self, key: u64) -> Option<&EeLogLine> {
        self.lines.iter().find(|line| line.index as u64 == key)
    }
}

impl Catalog for GameLog {
    type Row = EeLogLine;
    type Action = LogAction;
    type Data = (Vec<EeLogLine>, i64);

    const NOUN: &'static str = "log lines";
    const SEARCH_PLACEHOLDER: &'static str = "Search the loaded lines";
    const EMPTY_HINT: &'static str = "Quantframe has read no line of Warframe's EE.log. Start the game, or set the file under Settings, Logs.";
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[DataKind::Settings];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let page = source.debug_get_ee_logs(&self.params());
        Box::pin(async move {
            let page = page.await?;
            Ok((page.results, page.total))
        })
    }

    fn apply(&mut self, (lines, total): Self::Data) {
        self.lines = lines;
        self.total = total;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("index", "Line", 90.).end().sortable(),
            ColumnSpec::new("time", "Read at (UTC)", 170.).sortable(),
            ColumnSpec::new("text", "Text", 400.).flex(),
            ColumnSpec::new("actions", "", 48.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<EeLogLine> {
        self.lines
            .iter()
            .filter(|line| query.matches(&line.line))
            .cloned()
            .collect()
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        Vec::new()
    }

    fn summary(&self) -> Vec<Stat> {
        vec![Stat {
            label: "Lines loaded",
            value: number(self.lines.len() as i64),
            note: format!(
                "of {} in the log, {}",
                number(self.total),
                if self.hide_empty {
                    "without empty lines"
                } else {
                    "with empty lines"
                }
            ),
            tone: Tone::Plain,
        }]
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<LogAction>> {
        vec![
            ToolbarItem {
                id: "reload",
                icon: IconName::RefreshCw,
                tooltip: "Read the log again".to_owned(),
                action: Some(LogAction::Reload),
                danger: false,
            },
            ToolbarItem {
                id: "empty-lines",
                icon: if self.hide_empty {
                    IconName::Eye
                } else {
                    IconName::EyeOff
                },
                tooltip: if self.hide_empty {
                    "Show empty lines"
                } else {
                    "Hide empty lines"
                }
                .to_owned(),
                action: Some(LogAction::ToggleEmpty),
                danger: false,
            },
            ToolbarItem {
                id: "export",
                icon: IconName::Download,
                tooltip: "Export the log lines as JSON".to_owned(),
                action: Some(LogAction::Export),
                danger: false,
            },
        ]
    }

    fn primary(&self) -> Option<PrimaryAction<LogAction>> {
        None
    }

    fn perform(&mut self, action: LogAction, source: &DataSource) -> Outcome<LogAction> {
        match action {
            LogAction::Reload => Outcome::run_quietly(std::future::ready(Ok(()))),
            LogAction::ToggleEmpty => {
                self.hide_empty = !self.hide_empty;
                Outcome::run_quietly(std::future::ready(Ok(())))
            }
            LogAction::Export => {
                let source = source.clone();
                let params = self.params();
                Outcome::export("warframe-log.json", move |path| {
                    Box::pin(source.debug_export_ee_logs(&params, path))
                })
            }
            LogAction::Details(key) => Outcome::ShowPanel(key),
            LogAction::CopyLine(key) => match self.line(key) {
                Some(line) => Outcome::Copy {
                    what: format!("line {}", number(line.index as i64)),
                    text: line.line.clone(),
                },
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &EeLogLine) -> u64 {
        row.index as u64
    }

    fn compare(key: &str, a: &EeLogLine, b: &EeLogLine) -> Ordering {
        match key {
            "time" => a.date.cmp(&b.date),
            _ => a.index.cmp(&b.index),
        }
    }

    fn cell(row: &EeLogLine, key: &str, emit: &Emit<LogAction>, cx: &App) -> AnyElement {
        match key {
            "index" => cells::amount(Some(row.index as i64), "", cx),
            "time" => cells::text(kit::utc_date_time(row.date)),
            "text" => cells::text(row.line.clone()),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(overflow_button(
                        row.index as u64,
                        move || Self::row_menu(&menu_row),
                        emit,
                    ))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &EeLogLine, key: &str) -> Option<String> {
        match key {
            "index" => Some(number(row.index as i64)),
            "time" => Some(kit::utc_date_time(row.date)),
            "text" => Some(row.line.clone()),
            _ => None,
        }
    }

    fn row_menu(row: &EeLogLine) -> Vec<MenuEntry<LogAction>> {
        let id = row.index as u64;
        vec![
            MenuEntry::new("Show the whole line", IconName::PanelRight, LogAction::Details(id)),
            MenuEntry::new("Copy line", IconName::Copy, LogAction::CopyLine(id)),
        ]
    }

    fn panel(row: &EeLogLine, _: &App) -> PanelModel<LogAction> {
        PanelModel {
            title: format!("Line {}", number(row.index as i64)),
            subtitle: format!("Read at {} UTC", kit::utc_date_time(row.date)),
            status: None,
            note: None,
            history: Vec::new(),
            lines: Some(("Text", vec![(row.line.clone(), Tone::Plain)])),
            facts: vec![Fact::new("Characters", number(row.line.chars().count() as i64))],
            primary: vec![PanelAction::new(
                "Copy line",
                IconName::Copy,
                LogAction::CopyLine(row.index as u64),
            )],
            secondary: Vec::new(),
        }
    }
}
