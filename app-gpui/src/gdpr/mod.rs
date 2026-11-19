mod overview;
mod payments;
mod purchases;
mod report;
mod trades;

use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use gpui::{
    AnyElement, App, AppContext, Context, Div, Entity, ExternalPaths, IntoElement, ParentElement,
    PathPromptOptions, Pixels, Styled, Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    Icon, WindowExt,
    button::ButtonVariants,
    menu::{DropdownMenu, PopupMenuItem},
    notification::Notification,
};
use gpui_kit_assets::IconName;

use crate::{
    data::{Backend, DataResult, DataSource},
    kit,
    model::{GdprAccount, GdprItem},
    theme::Theme,
    ui::{
        Segment,
        grid::{ChipGroup, ChipSpec, Load},
        icon, number,
        page::{ListPage, PageHandle},
        placeholder, segmented, text_button,
    },
};
use payments::Payments;
use purchases::Purchases;
use report::Report;
use trades::Trades;

pub fn view(backend: &Entity<Backend>, window: &mut Window, cx: &mut App) -> Entity<GdprView> {
    cx.new(|cx| GdprView::new(backend, window, cx))
}

fn year(time: &str) -> &str {
    time.get(..4).unwrap_or_default()
}

fn month_index(time: &str) -> Option<usize> {
    time.get(5..7)?
        .parse::<usize>()
        .ok()?
        .checked_sub(1)
        .filter(|month| *month < 12)
}

fn item_text(items: &[GdprItem]) -> String {
    if items.is_empty() {
        return "No items".to_owned();
    }
    items
        .iter()
        .map(|item| {
            if item.quantity == 1 {
                item.name.clone()
            } else {
                format!("{} x {}", item.name, number(item.quantity))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn quantities<'a>(items: impl Iterator<Item = (&'a str, i64, i64)>) -> Vec<(String, i64, i64)> {
    let mut totals: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    for (name, quantity, amount) in items {
        let entry = totals.entry(name).or_default();
        entry.0 += quantity;
        entry.1 += amount;
    }
    let mut totals: Vec<(String, i64, i64)> = totals
        .into_iter()
        .map(|(name, (quantity, amount))| (name.to_owned(), quantity, amount))
        .collect();
    totals.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
    totals
}

fn ready() -> Load<()> {
    Box::pin(async { Ok(()) })
}

fn year_chips<'a>(times: impl Iterator<Item = &'a str>) -> ChipGroup {
    let mut years: BTreeMap<&str, usize> = BTreeMap::new();
    for time in times {
        *years.entry(year(time)).or_default() += 1;
    }
    ChipGroup {
        id: "year",
        chips: years
            .into_iter()
            .rev()
            .filter(|(year, _)| !year.is_empty())
            .map(|(year, count)| ChipSpec {
                key: year.to_owned(),
                label: year.to_owned(),
                count,
                color: None,
                tooltip: String::new(),
            })
            .collect(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Overview,
    Trades,
    Purchases,
    Payments,
}

impl Section {
    const ALL: [Self; 4] = [
        Self::Overview,
        Self::Trades,
        Self::Purchases,
        Self::Payments,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Trades => "Trades",
            Self::Purchases => "Market purchases",
            Self::Payments => "Payments",
        }
    }

    fn from_env() -> (Self, bool) {
        let view = std::env::var("QF_VIEW").unwrap_or_default();
        let (name, report) = match view.strip_suffix("-report") {
            Some(name) => (name, true),
            None => (view.as_str(), false),
        };
        let section = match name {
            "trades" => Self::Trades,
            "purchases" => Self::Purchases,
            "payments" => Self::Payments,
            _ => Self::Overview,
        };
        (section, report && section != Self::Overview)
    }
}

enum State {
    Loading,
    Missing,
    Failed(String),
    Ready(Vec<Arc<GdprAccount>>),
}

pub struct GdprView {
    source: DataSource,
    state: State,
    account: usize,
    section: Section,
    report: bool,
    year: Option<String>,
    importing: bool,
    problem: Option<String>,
    forced_missing: bool,
    pending_row: Option<usize>,
    trades: Entity<ListPage<Trades>>,
    purchases: Entity<ListPage<Purchases>>,
    payments: Entity<ListPage<Payments>>,
    _subscriptions: Vec<Subscription>,
}

impl GdprView {
    fn new(backend: &Entity<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let trades = cx.new(|cx| ListPage::new(Trades::default(), backend, window, cx));
        let purchases = cx.new(|cx| ListPage::new(Purchases::default(), backend, window, cx));
        let payments = cx.new(|cx| ListPage::new(Payments::default(), backend, window, cx));
        let subscriptions = vec![
            cx.observe(&trades, |_, _, cx| cx.notify()),
            cx.observe(&purchases, |_, _, cx| cx.notify()),
            cx.observe(&payments, |_, _, cx| cx.notify()),
        ];
        let (section, report) = Section::from_env();
        let mut view = Self {
            source: backend.read(cx).source.clone(),
            state: State::Loading,
            account: 0,
            section,
            report,
            year: None,
            importing: false,
            problem: None,
            forced_missing: std::env::var("QF_VIEW").as_deref() == Ok("import"),
            pending_row: None,
            trades,
            purchases,
            payments,
            _subscriptions: subscriptions,
        };
        view.load(window, cx);
        view
    }

    fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.forced_missing {
            self.state = State::Missing;
            cx.notify();
            return;
        }
        self.state = State::Loading;
        let state = self.source.wfgdpr_get_state();
        let accounts = self.source.wfgdpr_get_accounts();
        cx.spawn_in(window, async move |view, cx| {
            let result: DataResult<Option<Vec<GdprAccount>>> = async {
                if state.await?.was_initialized {
                    Ok(Some(accounts.await?))
                } else {
                    Ok(None)
                }
            }
            .await;
            view.update_in(cx, |view, window, cx| view.loaded(result, window, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    fn loaded(
        &mut self,
        result: DataResult<Option<Vec<GdprAccount>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state = match result {
            Ok(None) => State::Missing,
            Ok(Some(accounts)) if accounts.is_empty() => {
                self.problem =
                    Some("The last file has no account section, so there is nothing to show.".to_owned());
                State::Missing
            }
            Ok(Some(accounts)) => {
                self.problem = None;
                State::Ready(accounts.into_iter().map(Arc::new).collect())
            }
            Err(error) => State::Failed(kit::describe(&error)),
        };
        self.account = 0;
        self.share_account(window, cx);
        cx.notify();
    }

    fn accounts(&self) -> &[Arc<GdprAccount>] {
        match &self.state {
            State::Ready(accounts) => accounts,
            State::Loading | State::Missing | State::Failed(_) => &[],
        }
    }

    fn share_account(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let account = self.accounts().get(self.account).cloned();
        if account.is_some()
            && let Some(row) = self.pending_row.take()
        {
            match self.section {
                Section::Overview => {}
                Section::Trades => self.trades.on_load(Some(row), None, cx),
                Section::Purchases => self.purchases.on_load(Some(row), None, cx),
                Section::Payments => self.payments.on_load(Some(row), None, cx),
            }
        }
        let shared = account.clone();
        self.trades.update(cx, |page, cx| {
            page.update_catalog(|catalog| catalog.account = shared, window, cx);
        });
        let shared = account.clone();
        self.purchases.update(cx, |page, cx| {
            page.update_catalog(|catalog| catalog.account = shared, window, cx);
        });
        self.payments.update(cx, |page, cx| {
            page.update_catalog(|catalog| catalog.account = account, window, cx);
        });
    }

    fn pick_account(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index < self.accounts().len() && index != self.account {
            self.account = index;
            self.year = None;
            self.share_account(window, cx);
            cx.notify();
        }
    }

    fn choose_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            match chosen.await {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        view.update_in(cx, |view, window, cx| view.import(path, window, cx))
                            .ok();
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(_)) | Err(_) => {
                    view.update_in(cx, |_, window, cx| {
                        window.push_notification(
                            Notification::error(
                                "Could not open the file dialog. Drop the file on this tab instead.",
                            ),
                            cx,
                        );
                    })
                    .ok();
                }
            }
        })
        .detach();
    }

    fn import(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if self.importing {
            return;
        }
        self.importing = true;
        let name = path
            .file_name()
            .map_or_else(|| path.display().to_string(), |name| {
                name.to_string_lossy().into_owned()
            });
        let load = self.source.wfgdpr_load(path);
        cx.spawn_in(window, async move |view, cx| {
            let result = load.await;
            view.update_in(cx, |view, window, cx| {
                view.importing = false;
                match result {
                    Ok(()) => {
                        view.forced_missing = false;
                        view.problem = None;
                        view.year = None;
                        window.push_notification(format!("Imported {name}."), cx);
                        view.load(window, cx);
                    }
                    Err(error) => {
                        let reason = kit::describe(&error);
                        view.problem = Some(format!("Could not import {name}: {reason}"));
                        window.push_notification(
                            Notification::error(reason).title(format!("Could not import {name}")),
                            cx,
                        );
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn import_button(&self, label: &str, primary: bool, cx: &Context<Self>) -> impl IntoElement + use<> {
        let theme = Theme::global(cx);
        let button = text_button("gdpr-import", label, cx)
            .icon(Icon::new(IconName::FileUp).size(theme.metrics.icon - px(1.)))
            .loading(self.importing)
            .on_click(cx.listener(|view, _, window, cx| view.choose_file(window, cx)));
        if primary { button.primary() } else { button }
    }

    fn missing(&self, cx: &Context<Self>) -> Div {
        let colors = Theme::global(cx).colors;
        placeholder(
            IconName::FileUp,
            "No Warframe export imported yet",
            "Choose the text file from your Warframe GDPR data request, or drop it on this tab. Quantframe reads it on this computer and shows your trades, market purchases and payments.",
            cx,
        )
        .bg(colors.surface)
        .child(
            div()
                .pt(px(8.))
                .child(self.import_button("Choose file", true, cx)),
        )
        .children(self.problem.clone().map(|problem| {
            div().pt(px(6.)).max_w(px(560.)).child(kit::notice(
                IconName::TriangleAlert,
                colors.negative,
                problem,
                cx,
            ))
        }))
    }

    fn account_control(&self, account: &GdprAccount, cx: &Context<Self>) -> AnyElement {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let accounts = self.accounts();
        if accounts.len() < 2 {
            return div()
                .flex()
                .items_center()
                .gap(px(8.))
                .min_w_0()
                .child(icon(
                    IconName::CircleUser,
                    theme.metrics.icon + px(2.),
                    colors.text_muted,
                ))
                .child(
                    div()
                        .truncate()
                        .font_weight(theme.type_scale.label_weight)
                        .text_color(colors.text)
                        .child(account.display_name.clone()),
                )
                .into_any_element();
        }
        let names: Vec<String> = accounts
            .iter()
            .map(|account| account.display_name.clone())
            .collect();
        let current = self.account;
        let view = cx.entity().downgrade();
        text_button("gdpr-account", &account.display_name, cx)
            .icon(Icon::new(IconName::CircleUser).size(theme.metrics.icon))
            .dropdown_caret(true)
            .dropdown_menu(move |mut menu, _, _| {
                for (index, name) in names.iter().enumerate() {
                    let view = view.clone();
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(index == current)
                            .on_click(move |_, window, cx| {
                                view.update(cx, |view, cx| view.pick_account(index, window, cx))
                                    .ok();
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }

    fn header(&self, account: &GdprAccount, cx: &Context<Self>) -> Div {
        let theme = Theme::global(cx);
        let counts = [
            None,
            Some(account.trades.len()),
            Some(account.purchases.len()),
            Some(account.payments.len()),
        ];
        let segments = Section::ALL
            .into_iter()
            .zip(counts)
            .map(|(section, count)| Segment {
                label: section.label(),
                count,
            })
            .collect();
        let active = Section::ALL
            .iter()
            .position(|section| *section == self.section)
            .unwrap_or(0);
        let view = cx.entity().downgrade();
        let sections = segmented(
            "gdpr-section",
            segments,
            active,
            move |index, _, cx| {
                view.update(cx, |view, cx| {
                    view.section = Section::ALL[index];
                    cx.notify();
                })
                .ok();
            },
            cx,
        );
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .px(theme.metrics.gutter)
            .pt(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .min_w_0()
                    .child(sections)
                    .child(self.account_control(account, cx)),
            )
            .child(self.import_button("Import another file", false, cx))
    }

    fn report_view(&self, id: &'static str, report: &Report, cx: &Context<Self>) -> AnyElement {
        let view = cx.entity().downgrade();
        report::render(
            id,
            report,
            self.year.as_deref(),
            move |year, _, cx| {
                view.update(cx, |view, cx| {
                    view.year = Some(year);
                    cx.notify();
                })
                .ok();
            },
            cx,
        )
        .into_any_element()
    }

    fn render_body(
        &mut self,
        width: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.pending_row.is_some() && !self.accounts().is_empty() {
            self.share_account(window, cx);
        }
        let surface = Theme::global(cx).colors.surface;
        let frame = div()
            .size_full()
            .flex()
            .flex_col()
            .bg(surface)
            .on_drop(cx.listener(|view, paths: &ExternalPaths, window, cx| {
                if let Some(path) = paths.paths().first() {
                    view.import(path.clone(), window, cx);
                }
            }));
        let account = match &self.state {
            State::Loading => {
                return frame
                    .child(
                        kit::scroll_page("gdpr-loading", cx)
                            .child(kit::note("Reading the Warframe export.", cx)),
                    )
                    .into_any_element();
            }
            State::Failed(reason) => {
                return frame
                    .child(kit::failed(
                        "Could not read the Warframe export",
                        reason,
                        cx.listener(|view, _, window, cx| view.load(window, cx)),
                        cx,
                    ))
                    .into_any_element();
            }
            State::Missing => return frame.child(self.missing(cx)).into_any_element(),
            State::Ready(accounts) => accounts[self.account.min(accounts.len() - 1)].clone(),
        };
        let content = match (self.section, self.report) {
            (Section::Overview, _) => overview::render(&account, cx).into_any_element(),
            (Section::Trades, false) => self.trades.body(width, window, cx),
            (Section::Purchases, false) => self.purchases.body(width, window, cx),
            (Section::Payments, false) => self.payments.body(width, window, cx),
            (Section::Trades, true) => {
                self.report_view("gdpr-trades-report", &trades::report(&account), cx)
            }
            (Section::Purchases, true) => {
                self.report_view("gdpr-purchases-report", &purchases::report(&account), cx)
            }
            (Section::Payments, true) => {
                self.report_view("gdpr-payments-report", &payments::report(&account), cx)
            }
        };
        frame
            .child(self.header(&account, cx))
            .child(div().flex_1().min_h_0().child(content))
            .into_any_element()
    }
}

fn active_list(view: &Entity<GdprView>, cx: &App) -> Option<Box<dyn PageHandle>> {
    let view = view.read(cx);
    if view.accounts().is_empty() {
        return None;
    }
    match view.section {
        Section::Overview => None,
        Section::Trades => Some(Box::new(view.trades.clone())),
        Section::Purchases => Some(Box::new(view.purchases.clone())),
        Section::Payments => Some(Box::new(view.payments.clone())),
    }
}

impl PageHandle for Entity<GdprView> {
    fn toolbar(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let Some(list) = active_list(self, cx) else {
            return div().into_any_element();
        };
        let report = self.read(cx).report;
        let view = self.downgrade();
        let mode = segmented(
            "gdpr-mode",
            vec![
                Segment {
                    label: "List",
                    count: None,
                },
                Segment {
                    label: "Report",
                    count: None,
                },
            ],
            usize::from(report),
            move |index, _, cx| {
                view.update(cx, |view, cx| {
                    view.report = index == 1;
                    cx.notify();
                })
                .ok();
            },
            cx,
        );
        div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(mode)
            .children((!report).then(|| list.toolbar(window, cx)))
            .into_any_element()
    }

    fn body(&self, width: Pixels, window: &mut Window, cx: &mut App) -> AnyElement {
        self.update(cx, |view, cx| view.render_body(width, window, cx))
    }

    fn count(&self, _: &App) -> Option<usize> {
        None
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        if !self.read(cx).report
            && let Some(list) = active_list(self, cx)
        {
            list.focus(window, cx);
        }
    }

    fn focus_search(&self, window: &mut Window, cx: &mut App) {
        if !self.read(cx).report
            && let Some(list) = active_list(self, cx)
        {
            list.focus_search(window, cx);
        }
    }

    fn on_load(&self, row: Option<usize>, _: Option<String>, cx: &mut App) {
        self.update(cx, |view, cx| {
            view.pending_row = row;
            cx.notify();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, quantity: i64) -> GdprItem {
        GdprItem {
            name: name.to_owned(),
            quantity,
            tags: Vec::new(),
        }
    }

    #[test]
    fn item_text_names_each_item_and_its_quantity() {
        assert_eq!(item_text(&[]), "No items");
        assert_eq!(
            item_text(&[item("Rubico Prime Set", 1), item("Weapon Slot", 2)]),
            "Rubico Prime Set, Weapon Slot x 2"
        );
    }

    #[test]
    fn timestamps_give_their_year_and_month() {
        assert_eq!(year("2025-06-14T09:41:20+00:00"), "2025");
        assert_eq!(month_index("2025-06-14T09:41:20+00:00"), Some(5));
        assert_eq!(month_index("2025-13-14T09:41:20+00:00"), None);
        assert_eq!(month_index("soon"), None);
    }

    #[test]
    fn quantities_add_up_per_name_with_the_largest_first() {
        let totals = quantities(
            [("Weapon Slot", 2, 12), ("Orokin Reactor", 1, 20), ("Weapon Slot", 4, 24)]
                .into_iter(),
        );
        assert_eq!(totals[0], ("Weapon Slot".to_owned(), 6, 36));
        assert_eq!(totals[1], ("Orokin Reactor".to_owned(), 1, 20));
    }

    #[test]
    fn year_chips_put_the_newest_year_first() {
        let group = year_chips(
            ["2025-01-18T19:04:11Z", "2026-02-14T19:15:50Z", "2026-11-08T21:14:33Z"].into_iter(),
        );
        let chips: Vec<(&str, usize)> = group
            .chips
            .iter()
            .map(|chip| (chip.key.as_str(), chip.count))
            .collect();
        assert_eq!(chips, [("2026", 2), ("2025", 1)]);
    }
}
