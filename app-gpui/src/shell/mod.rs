//! The window frame around every screen: navigation, title bar, live scraper
//! control and status bar.

mod nav;

use gpui::{
    AnyElement, App, Context, Decorations, Div, Entity, IntoElement, ParentElement, Pixels, Render,
    Stateful, Styled, Window, div, img, prelude::*, px,
};
use gpui_component::{Icon, TitleBar, button::ButtonVariants};
use gpui_kit_assets::IconName;

use crate::{
    assets::APP_ICON,
    data::{Backend, BackendEvent, DataKind, DataSource},
    listings::ListingsView,
    model::{LiveScraperState, StockStatus, User, UserStatus},
    theme::{Navigation, Theme, Variant},
    ui::{dot, icon, label_text, number, text_button},
};
use nav::{Screen, ScreenGroup};

pub struct AppShell {
    screen: Screen,
    listings: Entity<ListingsView>,
    source: DataSource,
    scraper: LiveScraperState,
    /// The message to show again when the scraper restarts.
    last_message: Option<crate::model::LiveScraperMessage>,
    user: User,
    live_orders: usize,
    _subscription: gpui::Subscription,
}

impl AppShell {
    pub fn new(backend: Entity<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let source = backend.read(cx).source.clone();
        let subscription = cx.subscribe(&backend, |shell, _, event: &BackendEvent, cx| {
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
                BackendEvent::Changed(DataKind::User | DataKind::StockItems) => shell.refresh(cx),
                _ => return,
            }
            cx.notify();
        });
        let mut shell = Self {
            screen: Screen::Listings,
            user: User::default(),
            last_message: None,
            scraper: LiveScraperState::default(),
            live_orders: 0,
            listings: cx.new(|cx| ListingsView::new(backend, window, cx)),
            source,
            _subscription: subscription,
        };
        shell.refresh(cx);
        shell
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let source = self.source.clone();
        cx.spawn(async move |shell, cx| {
            let user = source.user().await;
            let scraper = source.live_scraper_get_state().await;
            let counts = source.get_stock_item_status_counts().await;
            shell.update(cx, |shell, cx| {
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
            shell.update(cx, |shell, cx| {
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
        })
        .detach();
    }

    fn nav_item(&self, screen: Screen, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let active = screen == self.screen;
        let badge = (screen == Screen::Chat && self.user.unread_messages > 0)
            .then_some(self.user.unread_messages);
        let item = div()
            .id(screen.label())
            .flex()
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(move |shell, _, _, cx| {
                shell.screen = screen;
                cx.notify();
            }));
        match theme.layout.navigation {
            Navigation::Sidebar { .. } => item
                .gap(px(9.))
                .h(px(30.))
                .px(px(9.))
                .rounded(theme.metrics.radius)
                .text_size(theme.type_scale.body)
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
                        .text_color(colors.on_accent)
                        .child(number(count))
                })),
            Navigation::TopBar => item
                .gap(px(6.))
                .h_full()
                .px(px(10.))
                .border_b_2()
                .text_size(theme.type_scale.body)
                .font_weight(theme.type_scale.label_weight)
                .map(|item| {
                    if active {
                        item.border_color(colors.accent).text_color(colors.text)
                    } else {
                        item.border_color(colors.accent.opacity(0.))
                            .text_color(colors.text_muted)
                            .hover(|item| item.text_color(colors.text))
                    }
                })
                .child(screen.label())
                .children(badge.map(|count| {
                    div()
                        .font_family(theme.type_scale.numeric_family.clone())
                        .text_size(theme.type_scale.caption)
                        .text_color(colors.accent)
                        .child(number(count))
                })),
            Navigation::Rail { .. } => item
                .relative()
                .justify_center()
                .size(px(40.))
                .rounded(theme.metrics.radius_large)
                .map(|item| {
                    if active {
                        item.bg(colors.row_selected)
                    } else {
                        item.hover(|item| item.bg(colors.control_hover))
                    }
                })
                .tooltip(move |window, cx| {
                    gpui_component::tooltip::Tooltip::new(screen.label()).build(window, cx)
                })
                .child(icon(
                    screen.icon(),
                    px(19.),
                    if active {
                        colors.accent
                    } else {
                        colors.text_muted
                    },
                ))
                .children(badge.map(|_| {
                    div()
                        .absolute()
                        .top(px(7.))
                        .right(px(7.))
                        .child(dot(colors.accent, px(7.)))
                })),
        }
    }

    fn nav_group(&self, group: ScreenGroup, cx: &mut Context<Self>) -> Vec<Stateful<Div>> {
        Screen::ALL
            .into_iter()
            .filter(|screen| screen.group() == group)
            .map(|screen| self.nav_item(screen, cx))
            .collect()
    }

    fn sidebar(&self, width: Pixels, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let group_label = |text: &str| {
            div()
                .px(px(9.))
                .pt(px(14.))
                .pb(px(5.))
                .text_size(theme.type_scale.caption)
                .font_weight(theme.type_scale.label_weight)
                .text_color(colors.text_faint)
                .child(label_text(&theme, text))
        };
        div()
            .w(width)
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
                                    .child(self.user.wfm_username.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(theme.type_scale.caption)
                                    .text_color(colors.text_muted)
                                    .child(dot(self.user_status_color(&theme), px(6.)))
                                    .child(self.user.wfm_status.label()),
                            ),
                    ),
            )
    }

    fn rail(&self, width: Pixels, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        div()
            .w(width)
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.))
            .pb(px(12.))
            .bg(colors.chrome)
            .border_r_1()
            .border_color(colors.border)
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .h(theme.metrics.title_bar_height)
                    .when(cfg!(target_os = "macos"), |brand| brand.pt(px(28.)))
                    .child(img(APP_ICON).size(px(30.)).rounded(px(7.))),
            )
            .children(self.nav_group(ScreenGroup::Trading, cx))
            .child(
                div()
                    .my(px(6.))
                    .w(px(24.))
                    .h(px(1.))
                    .flex_shrink_0()
                    .bg(colors.border),
            )
            .children(self.nav_group(ScreenGroup::Overview, cx))
            .child(div().flex_1())
            .children(self.nav_group(ScreenGroup::App, cx))
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
        let short = theme.variant == Variant::B;
        let button = if self.scraper.is_running {
            text_button(
                "scraper-toggle",
                if short { "Stop" } else { "Stop live scraper" },
                cx,
            )
            .icon(Icon::new(IconName::Square).size(size))
        } else {
            text_button(
                "scraper-toggle",
                if short { "Start" } else { "Start live scraper" },
                cx,
            )
            .primary()
            .icon(Icon::new(IconName::Play).size(size))
        };
        button.on_click(cx.listener(|shell, _, _, cx| shell.toggle_scraper(cx)))
    }

    /// The live scraper's running state and its current step.
    fn scraper_state(&self, with_progress: bool, cx: &App) -> Div {
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
                    .filter(|_| with_progress)
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(dot(self.user_status_color(theme), px(6.)))
                    .child(format!(
                        "{}, {}",
                        self.user.wfm_username,
                        self.user.wfm_status.label().to_lowercase()
                    )),
            )
    }

    fn status_bar(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let bar = div()
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
            .text_color(colors.text_muted);
        match theme.variant {
            Variant::A => bar
                .child(self.scraper_state(true, cx))
                .child(self.account_state(cx)),
            Variant::B | Variant::C => bar.child(self.account_state(cx)).child(
                div()
                    .text_color(colors.text_faint)
                    .child(concat!("Quantframe ", env!("CARGO_PKG_VERSION"))),
            ),
        }
    }

    fn title_bar(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let content: AnyElement = match theme.layout.navigation {
            Navigation::Sidebar { .. } => div()
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
                        .child(self.screen.label()),
                )
                .child(self.scraper_button(cx))
                .into_any_element(),
            Navigation::TopBar => div()
                .flex()
                .flex_1()
                .h_full()
                .items_center()
                .gap(px(4.))
                .pl(if cfg!(target_os = "macos") {
                    px(82.)
                } else {
                    theme.metrics.gutter
                })
                .pr(theme.metrics.gutter)
                .child(img(APP_ICON).size(px(20.)).rounded(px(4.)))
                .child(div().w(px(6.)))
                .children(self.nav_group(ScreenGroup::Trading, cx))
                .children(self.nav_group(ScreenGroup::Overview, cx))
                .children(self.nav_group(ScreenGroup::App, cx))
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .text_size(theme.type_scale.body)
                        .child(self.scraper_state(true, cx))
                        .child(self.scraper_button(cx)),
                )
                .into_any_element(),
            Navigation::Rail { .. } => div()
                .flex()
                .flex_1()
                .items_center()
                .justify_between()
                .gap(px(24.))
                .px(theme.metrics.gutter)
                .child(
                    div()
                        .text_size(theme.type_scale.title)
                        .font_weight(theme.type_scale.title_weight)
                        .text_color(colors.text)
                        .child(self.screen.label()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .min_w_0()
                        .text_size(theme.type_scale.body)
                        .child(self.scraper_state(false, cx))
                        .child(self.scraper_button(cx)),
                )
                .into_any_element(),
        };
        let bordered = theme.variant == Variant::B;
        TitleBar::new()
            .h(theme.metrics.title_bar_height)
            .pl(px(0.))
            .bg(if bordered {
                colors.chrome
            } else {
                colors.surface
            })
            .border_color(if bordered {
                colors.border
            } else {
                colors.surface
            })
            .child(content)
            .when(draws_own_window_controls(window), |bar| {
                bar.child(window_controls(cx))
            })
    }

    fn content(&self, cx: &App) -> AnyElement {
        if self.screen == Screen::Listings {
            return self.listings.clone().into_any_element();
        }
        let theme = Theme::global(cx);
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.surface)
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text_muted)
            .child(format!("{} is not built yet.", self.screen.label()))
            .into_any_element()
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
    let control = |id: &'static str, name: KitIcon| {
        div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .w(px(40.))
            .h_full()
            .text_color(colors.text_muted)
            .hover(|control| control.bg(colors.control_hover).text_color(colors.text))
            .child(Icon::new(name).size(px(15.)))
    };
    div()
        .flex()
        .flex_shrink_0()
        .h_full()
        .child(
            control("window-minimize", KitIcon::WindowMinimize)
                .on_click(|_, window, _| window.minimize_window()),
        )
        .child(
            control("window-maximize", KitIcon::WindowMaximize)
                .on_click(|_, window, _| window.zoom_window()),
        )
        .child(
            control("window-close", KitIcon::WindowClose)
                .hover(|control| control.bg(colors.negative).text_color(colors.on_accent))
                .on_click(|_, window, _| window.remove_window()),
        )
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let main = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(self.title_bar(window, cx))
            .child(div().flex_1().min_h_0().child(self.content(cx)))
            .child(self.status_bar(cx));
        let frame = div()
            .size_full()
            .flex()
            .bg(theme.colors.window)
            .font_family(theme.type_scale.family.clone())
            .text_size(theme.type_scale.body)
            .text_color(theme.colors.text);
        match theme.layout.navigation {
            Navigation::Sidebar { width } => frame.child(self.sidebar(width, cx)).child(main),
            Navigation::Rail { width } => frame.child(self.rail(width, cx)).child(main),
            Navigation::TopBar => frame.child(main),
        }
    }
}
