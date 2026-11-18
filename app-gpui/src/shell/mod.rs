mod nav;

use gpui::{
    AnyElement, App, Context, Decorations, Div, Entity, IntoElement, ParentElement, Render,
    Stateful, Styled, Subscription, Window, div, img, prelude::*, px,
};
use gpui_component::{
    Icon, TitleBar,
    button::ButtonVariants,
    menu::{DropdownMenu, PopupMenuItem},
    tooltip::Tooltip,
};
use gpui_kit_assets::IconName;

use crate::{
    about::AboutView,
    analytics,
    assets::APP_ICON,
    auth::{self, LoginView, SignedIn},
    chat::ChatView,
    data::{Backend, BackendEvent, DataKind, DataSource},
    debug,
    home::HomeView,
    inventory, listings,
    model::{Ban, BanService, LiveScraperMessage, LiveScraperState, StockStatus, User, UserStatus},
    orders,
    settings::SettingsView,
    theme::{Appearance, AppearanceChoice, Theme, tabular_figures},
    trade_messages,
    ui::{IconButton, dot, focus_ring, icon, number, page, screen::TabbedScreen, text_button},
};
use nav::{Screen, ScreenGroup};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Gate {
    SignIn,
    Banned(Ban),
    Failed(String),
}

impl Gate {
    fn preview(key: &str) -> Option<Self> {
        match key {
            "login" => Some(Self::SignIn),
            "banned" => Some(Self::Banned(Ban {
                service: BanService::WarframeMarket,
                until: Some("2026-12-02T08:00:00Z".to_owned()),
                reason: Some("Repeated no-shows on accepted trades.".to_owned()),
            })),
            "error" => Some(Self::Failed(
                "The warframe.market session could not be restored (401 Unauthorized)."
                    .to_owned(),
            )),
            _ => None,
        }
    }
}

pub struct AppShell {
    backend: Entity<Backend>,
    source: DataSource,
    screen: Screen,
    listings: Entity<TabbedScreen>,
    orders: Entity<TabbedScreen>,
    trade_messages: Entity<TabbedScreen>,
    analytics: Option<Entity<TabbedScreen>>,
    inventory: Option<Entity<TabbedScreen>>,
    debug: Option<Entity<TabbedScreen>>,
    home: Option<Entity<HomeView>>,
    chat: Option<Entity<ChatView>>,
    settings: Option<Entity<SettingsView>>,
    about: Option<Entity<AboutView>>,
    login: Entity<LoginView>,
    preview: Option<Gate>,
    app_error: Option<String>,
    scraper: LiveScraperState,
    last_message: Option<LiveScraperMessage>,
    user: User,
    live_orders: usize,
    _subscriptions: Vec<Subscription>,
}

struct Startup {
    screen: Option<Screen>,
    preview: Option<Gate>,
    tab: Option<String>,
    row: Option<usize>,
    dialog: Option<String>,
}

impl Startup {
    fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        Self {
            screen: var("QF_SCREEN").and_then(|key| Screen::from_key(&key)),
            preview: var("QF_SCREEN").and_then(|key| Gate::preview(&key)),
            tab: var("QF_TAB"),
            row: var("QF_ROW").and_then(|row| row.parse().ok()),
            dialog: var("QF_DIALOG"),
        }
    }
}

impl AppShell {
    pub fn new(backend: Entity<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        page::init(cx);
        Theme::follow_system(window.appearance(), cx);
        let source = backend.read(cx).source.clone();
        let startup = Startup::from_env();
        let login = cx.new(|cx| LoginView::new(&backend, window, cx));
        let mut shell = Self {
            analytics: None,
            inventory: None,
            debug: None,
            home: None,
            chat: None,
            settings: None,
            about: None,
            preview: startup.preview.clone(),
            app_error: None,
            screen: startup.screen.unwrap_or(Screen::Listings),
            listings: listings::screen(&backend, window, cx),
            orders: orders::screen(&backend, window, cx),
            trade_messages: trade_messages::screen(&backend, window, cx),
            last_message: None,
            scraper: LiveScraperState::default(),
            user: User::default(),
            live_orders: 0,
            source,
            _subscriptions: vec![
                cx.observe_window_appearance(window, |_, window, cx| {
                    Theme::follow_system(window.appearance(), cx);
                }),
                cx.subscribe(&backend, |shell, _, event: &BackendEvent, cx| {
                    match event {
                        BackendEvent::LiveScraperRunning(running) => {
                            shell.scraper.is_running = *running;
                            if !running {
                                shell.scraper.message = None;
                            }
                        }
                        BackendEvent::LiveScraperMessage(message) => {
                            shell.last_message = Some(message.clone());
                            shell.scraper.message = Some(message.clone());
                        }
                        BackendEvent::Changed(DataKind::User | DataKind::StockItems) => {
                            shell.refresh(cx);
                        }
                        BackendEvent::Error(error) => shell.app_error = error.clone(),
                        _ => return,
                    }
                    cx.notify();
                }),
                cx.subscribe(&login, |shell, _, _: &SignedIn, cx| shell.refresh(cx)),
            ],
            login,
            backend,
        };
        shell.refresh(cx);
        cx.defer_in(window, move |shell, window, cx| {
            shell.apply_startup(startup, window, cx);
        });
        shell
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let source = self.source.clone();
        cx.spawn(async move |shell, cx| {
            let user = source.user().await;
            let scraper = source.live_scraper_get_state().await;
            let counts = source.get_stock_item_status_counts().await;
            shell
                .update(cx, |shell, cx| {
                    if let Ok(user) = user {
                        shell.user = user;
                    }
                    if let Ok(scraper) = scraper {
                        shell.scraper.is_running = scraper.is_running;
                    }
                    if let Ok(counts) = counts {
                        shell.live_orders = counts.get(&StockStatus::Live).copied().unwrap_or(0);
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn apply_startup(&mut self, startup: Startup, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_built(window, cx);
        if let Some(screen) = self.built_screen().cloned() {
            screen.update(cx, |screen, cx| {
                if let Some(tab) = &startup.tab {
                    screen.open_tab(tab, window, cx);
                }
                let page = screen.page();
                page.focus(window, cx);
                page.on_load(startup.row, startup.dialog, cx);
            });
        } else if let (Screen::Settings, Some(settings), Some(tab)) =
            (self.screen, &self.settings, &startup.tab)
        {
            settings.update(cx, |settings, cx| settings.open_section(tab, cx));
        } else if let (Screen::Chat, Some(chat)) = (self.screen, &self.chat) {
            chat.update(cx, |chat, _| chat.on_load(startup.row, startup.dialog));
        }
        if self.gate() == Some(Gate::SignIn) {
            self.login
                .clone()
                .update(cx, |login, cx| login.focus(window, cx));
        }
    }

    fn ensure_built(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let backend = self.backend.clone();
        match self.screen {
            Screen::Listings | Screen::Orders | Screen::TradeMessages => {}
            Screen::Analytics => {
                self.analytics
                    .get_or_insert_with(|| analytics::screen(&backend, window, cx));
            }
            Screen::Inventory => {
                self.inventory
                    .get_or_insert_with(|| inventory::screen(&backend, window, cx));
            }
            Screen::Debug => {
                self.debug
                    .get_or_insert_with(|| debug::screen(&backend, window, cx));
            }
            Screen::Home => {
                self.home
                    .get_or_insert_with(|| cx.new(|cx| HomeView::new(&backend, cx)));
            }
            Screen::Chat => {
                self.chat
                    .get_or_insert_with(|| cx.new(|cx| ChatView::new(&backend, window, cx)));
            }
            Screen::Settings => {
                self.settings
                    .get_or_insert_with(|| cx.new(|cx| SettingsView::new(&backend, window, cx)));
            }
            Screen::About => {
                self.about
                    .get_or_insert_with(|| cx.new(|cx| AboutView::new(&backend, cx)));
            }
        }
    }

    fn built_screen(&self) -> Option<&Entity<TabbedScreen>> {
        match self.screen {
            Screen::Listings => Some(&self.listings),
            Screen::Orders => Some(&self.orders),
            Screen::TradeMessages => Some(&self.trade_messages),
            Screen::Analytics => self.analytics.as_ref(),
            Screen::Inventory => self.inventory.as_ref(),
            Screen::Debug => self.debug.as_ref(),
            Screen::Home | Screen::Chat | Screen::Settings | Screen::About => None,
        }
    }

    fn open_screen(&mut self, screen: Screen, window: &mut Window, cx: &mut Context<Self>) {
        if let (Screen::Chat, Some(chat)) = (self.screen, &self.chat) {
            chat.clone().update(cx, |chat, cx| chat.leave(cx));
        }
        self.screen = screen;
        self.ensure_built(window, cx);
        if let Some(view) = self.built_screen().cloned() {
            view.update(cx, |view, cx| view.page().focus(window, cx));
        } else if let (Screen::Chat, Some(chat)) = (screen, &self.chat) {
            chat.clone().update(cx, |chat, cx| {
                chat.enter(cx);
                chat.focus(window, cx);
            });
        }
        cx.notify();
    }

    fn gate(&self) -> Option<Gate> {
        if self.preview.is_some() {
            return self.preview.clone();
        }
        if let Some(ban) = &self.user.banned {
            return Some(Gate::Banned(ban.clone()));
        }
        self.user.anonymous.then_some(Gate::SignIn)
    }

    fn sign_out(&mut self, cx: &mut Context<Self>) {
        self.preview = None;
        let signed_out = self.source.auth_logout();
        cx.spawn(async move |shell, cx| {
            let user = signed_out.await;
            shell
                .update(cx, |shell, cx| {
                    if let Ok(user) = user {
                        shell.user = user;
                    }
                    shell.refresh(cx);
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn toggle_scraper(&mut self, cx: &mut Context<Self>) {
        let source = self.source.clone();
        let start = !self.scraper.is_running;
        cx.spawn(async move |shell, cx| {
            let state = if start {
                source.live_scraper_start().await
            } else {
                source.live_scraper_stop().await
            };
            shell
                .update(cx, |shell, cx| {
                    if let Ok(state) = state {
                        shell.scraper.is_running = state.is_running;
                        shell.scraper.message = shell
                            .scraper
                            .is_running
                            .then(|| shell.last_message.clone())
                            .flatten();
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }

    fn account_name(&self) -> Option<&str> {
        let name = self.user.wfm_username.trim();
        (!self.user.anonymous && !name.is_empty()).then_some(name)
    }

    fn nav_item(&self, screen: Screen, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let active = screen == self.screen;
        let badge = (screen == Screen::Chat && self.user.unread_messages > 0)
            .then_some(self.user.unread_messages);
        let ring = focus_ring(&theme);
        div()
            .id(screen.label())
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(9.))
            .h(px(30.))
            .px(px(9.))
            .rounded(theme.metrics.radius)
            .text_size(theme.type_scale.body)
            .cursor_pointer()
            .tab_index(0)
            .focus_visible(move |style| style.shadow(ring))
            .on_click(cx.listener(move |shell, _, window, cx| {
                shell.open_screen(screen, window, cx);
            }))
            .map(|item| {
                if active {
                    item.bg(colors.row_selected)
                        .text_color(colors.text)
                        .font_weight(theme.type_scale.label_weight)
                } else {
                    item.text_color(colors.text_muted)
                        .hover(|item| item.bg(colors.control_hover).text_color(colors.text))
                }
            })
            .child(icon(
                screen.icon(),
                theme.metrics.icon + px(1.),
                if active {
                    colors.accent
                } else {
                    colors.text_muted
                },
            ))
            .child(div().flex_1().child(screen.label()))
            .children(badge.map(|count| {
                div()
                    .px(px(6.))
                    .h(px(17.))
                    .flex()
                    .items_center()
                    .rounded(px(9.))
                    .bg(colors.accent)
                    .text_size(theme.type_scale.caption)
                    .font_weight(theme.type_scale.label_weight)
                    .font_features(tabular_figures())
                    .text_color(colors.on_accent)
                    .child(number(count))
            }))
    }

    fn nav_group(&self, group: ScreenGroup, cx: &mut Context<Self>) -> Vec<Stateful<Div>> {
        Screen::ALL
            .into_iter()
            .filter(|screen| screen.group() == group)
            .map(|screen| self.nav_item(screen, cx))
            .collect()
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let group_label = |text: &'static str| {
            div()
                .px(px(9.))
                .pt(px(14.))
                .pb(px(5.))
                .text_size(theme.type_scale.caption)
                .font_weight(theme.type_scale.label_weight)
                .text_color(colors.text_faint)
                .child(text)
        };
        div()
            .w(theme.metrics.sidebar_width)
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(colors.chrome)
            .border_r_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(9.))
                    .h(theme.metrics.title_bar_height)
                    .px(px(14.))
                    .when(cfg!(target_os = "macos"), |brand| brand.pl(px(82.)))
                    .child(img(APP_ICON).size(px(22.)).rounded(px(5.)))
                    .child(
                        div()
                            .text_size(theme.type_scale.title)
                            .font_weight(theme.type_scale.title_weight)
                            .text_color(colors.text)
                            .child("Quantframe"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .px(px(8.))
                    .child(group_label("Trading"))
                    .children(self.nav_group(ScreenGroup::Trading, cx))
                    .child(group_label("Overview"))
                    .children(self.nav_group(ScreenGroup::Overview, cx))
                    .child(div().flex_1())
                    .children(self.nav_group(ScreenGroup::App, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(9.))
                    .mt(px(8.))
                    .px(px(14.))
                    .h(px(48.))
                    .border_t_1()
                    .border_color(colors.border)
                    .child(icon(IconName::CircleUser, px(22.), colors.text_muted))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(theme.type_scale.body)
                                    .font_weight(theme.type_scale.label_weight)
                                    .text_color(colors.text)
                                    .child(self.account_name().unwrap_or("Not signed in").to_owned()),
                            )
                            .children(self.account_name().map(|_| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(theme.type_scale.caption)
                                    .text_color(colors.text_muted)
                                    .child(dot(self.user_status_color(&theme), px(6.)))
                                    .child(self.user.wfm_status.label())
                            })),
                    ),
            )
    }

    fn user_status_color(&self, theme: &Theme) -> gpui::Hsla {
        match self.user.wfm_status {
            UserStatus::InGame => theme.colors.accent,
            UserStatus::Online => theme.colors.positive,
            UserStatus::Invisible => theme.colors.text_faint,
        }
    }

    fn scraper_button(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = Theme::global(cx);
        let size = theme.metrics.icon - px(2.);
        let button = if self.scraper.is_running {
            text_button("scraper-toggle", "Stop live scraper", cx)
                .icon(Icon::new(IconName::Square).size(size))
        } else {
            text_button("scraper-toggle", "Start live scraper", cx)
                .primary()
                .icon(Icon::new(IconName::Play).size(size))
        };
        button.on_click(cx.listener(|shell, _, _, cx| shell.toggle_scraper(cx)))
    }

    fn appearance_button(&self, cx: &App) -> impl IntoElement + use<> {
        let theme = Theme::global(cx);
        let current = theme.choice;
        let name = match (current, theme.appearance) {
            (AppearanceChoice::System, _) => IconName::SunMoon,
            (_, Appearance::Light) => IconName::Sun,
            (_, Appearance::Dark) => IconName::Moon,
        };
        IconButton::new("appearance", name, "Appearance").dropdown_menu(move |mut menu, _, _| {
            for choice in AppearanceChoice::ALL {
                menu = menu.item(
                    PopupMenuItem::new(choice.label())
                        .checked(choice == current)
                        .on_click(move |_, window, cx| {
                            Theme::set_choice(choice, window.appearance(), cx);
                        }),
                );
            }
            menu
        })
    }

    fn scraper_state(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let running = self.scraper.is_running;
        let message = self.scraper.message.as_ref();
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .min_w_0()
            .child(dot(
                if running {
                    colors.positive
                } else {
                    colors.text_faint
                },
                px(7.),
            ))
            .child(
                div()
                    .flex_shrink_0()
                    .font_weight(theme.type_scale.label_weight)
                    .text_color(colors.text)
                    .child(if running {
                        "Live scraper running"
                    } else {
                        "Live scraper stopped"
                    }),
            )
            .children(message.map(|message| {
                div()
                    .truncate()
                    .text_color(colors.text_muted)
                    .child(message.text())
            }))
            .children(
                message
                    .and_then(|message| message.progress())
                    .map(|(current, total)| {
                        let width = 120.;
                        let done = width * current as f32 / total.max(1) as f32;
                        div()
                            .flex_shrink_0()
                            .w(px(width))
                            .h(px(4.))
                            .rounded(px(2.))
                            .bg(colors.border_strong)
                            .child(div().w(px(done)).h_full().rounded(px(2.)).bg(colors.accent))
                    }),
            )
    }

    fn account_state(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(icon(
                        IconName::Globe,
                        theme.metrics.icon - px(2.),
                        colors.text_faint,
                    ))
                    .child(format!(
                        "{} of {} orders on Warframe Market",
                        self.live_orders, self.user.order_limit
                    )),
            )
            .child(match self.account_name() {
                Some(name) => div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(dot(self.user_status_color(theme), px(6.)))
                    .child(format!(
                        "{name}, {}",
                        self.user.wfm_status.label().to_lowercase()
                    )),
                None => div().child("Not signed in"),
            })
    }

    fn status_bar(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .gap(px(16.))
            .h(theme.metrics.status_bar_height)
            .px(theme.metrics.gutter)
            .border_t_1()
            .border_color(colors.border)
            .bg(colors.chrome)
            .text_size(theme.type_scale.caption + px(1.))
            .text_color(colors.text_muted)
            .child(self.scraper_state(cx))
            .child(self.account_state(cx))
    }

    fn title_bar(
        &self,
        gated: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        TitleBar::new()
            .h(theme.metrics.title_bar_height)
            .pl(px(0.))
            .bg(colors.surface)
            .border_color(colors.surface)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_between()
                    .px(theme.metrics.gutter)
                    .child(
                        div()
                            .text_size(theme.type_scale.title)
                            .font_weight(theme.type_scale.title_weight)
                            .text_color(colors.text)
                            .when(gated && cfg!(target_os = "macos"), |title| title.pl(px(66.)))
                            .child(if gated {
                                "Quantframe"
                            } else {
                                self.screen.label()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(self.appearance_button(cx))
                            .when(!gated, |buttons| buttons.child(self.scraper_button(cx))),
                    ),
            )
            .when(draws_own_window_controls(window), |bar| {
                bar.child(window_controls(cx))
            })
    }

    fn content(&self, cx: &App) -> AnyElement {
        let view = match self.screen {
            Screen::Home => self.home.clone().map(IntoElement::into_any_element),
            Screen::Chat => self.chat.clone().map(IntoElement::into_any_element),
            Screen::Settings => self.settings.clone().map(IntoElement::into_any_element),
            Screen::About => self.about.clone().map(IntoElement::into_any_element),
            _ => self
                .built_screen()
                .cloned()
                .map(IntoElement::into_any_element),
        };
        view.unwrap_or_else(|| {
            div()
                .size_full()
                .bg(Theme::global(cx).colors.surface)
                .into_any_element()
        })
    }

    fn error_banner(&self, error: &str, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(9.))
            .h(px(36.))
            .pl(theme.metrics.gutter)
            .pr(px(8.))
            .border_b_1()
            .border_color(colors.border)
            .bg(colors.chrome)
            .child(icon(
                IconName::TriangleAlert,
                theme.metrics.icon,
                colors.negative,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(colors.text)
                    .child(error.to_owned()),
            )
            .child(
                IconButton::new("error-dismiss", IconName::X, "Hide this error").on_click(
                    cx.listener(|shell, _, _, cx| {
                        shell.app_error = None;
                        cx.notify();
                    }),
                ),
            )
    }

    fn gate_view(&self, gate: &Gate, cx: &mut Context<Self>) -> AnyElement {
        match gate {
            Gate::SignIn => self.login.clone().into_any_element(),
            Gate::Banned(ban) => auth::banned(
                ban,
                cx.listener(|shell, _, _, cx| shell.sign_out(cx)),
                cx,
            )
            .into_any_element(),
            Gate::Failed(reason) => auth::error_page(
                "Quantframe ran into a problem",
                reason,
                "Quit Quantframe",
                |_, _, cx| cx.quit(),
                cx,
            )
            .into_any_element(),
        }
    }
}

/// True when nothing else draws window buttons: on Linux without client-side
/// decorations, when `QF_WINDOW_CONTROLS` is set. A bare X server such as Xvfb
/// has no window manager, so gpui-component's own buttons stay hidden there.
fn draws_own_window_controls(window: &Window) -> bool {
    cfg!(target_os = "linux")
        && !matches!(window.window_decorations(), Decorations::Client { .. })
        && std::env::var_os("QF_WINDOW_CONTROLS").is_some()
}

fn window_controls(cx: &App) -> Div {
    use gpui_component::IconName as KitIcon;
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let control = |id: &'static str, name: KitIcon, tooltip: &'static str| {
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(40.))
            .h_full()
            .text_color(colors.text_muted)
            .hover(|control| control.bg(colors.control_hover).text_color(colors.text))
            .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
            .child(Icon::new(name).size(px(15.)))
    };
    div()
        .flex()
        .flex_shrink_0()
        .h_full()
        .child(
            control("window-minimize", KitIcon::WindowMinimize, "Minimise")
                .on_click(|_, window, _| window.minimize_window()),
        )
        .child(
            control("window-maximize", KitIcon::WindowMaximize, "Maximise")
                .on_click(|_, window, _| window.zoom_window()),
        )
        .child(
            control("window-close", KitIcon::WindowClose, "Close")
                .hover(|control| control.bg(colors.negative).text_color(colors.on_accent))
                .on_click(|_, window, _| window.remove_window()),
        )
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let frame = div()
            .size_full()
            .flex()
            .bg(theme.colors.window)
            .font_family(theme.type_scale.family.clone())
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text);
        if let Some(gate) = self.gate() {
            return frame
                .flex_col()
                .child(self.title_bar(true, window, cx))
                .child(div().flex_1().min_h_0().child(self.gate_view(&gate, cx)));
        }
        let error = self.app_error.clone();
        frame.child(self.sidebar(cx)).child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .child(self.title_bar(false, window, cx))
                .children(error.map(|error| self.error_banner(&error, cx)))
                .child(div().flex_1().min_h_0().child(self.content(cx)))
                .child(self.status_bar(cx)),
        )
    }
}
