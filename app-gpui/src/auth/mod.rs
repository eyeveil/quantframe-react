use gpui::{
    App, ClickEvent, Context, Div, Entity, EventEmitter, Focusable, IntoElement, ParentElement,
    Render, Styled, Subscription, Window, div, img, prelude::*, px,
};
use gpui_component::{
    Disableable, Icon, WindowExt,
    button::ButtonVariants,
    input::{Input, InputEvent, InputState},
    notification::Notification,
};
use gpui_kit_assets::IconName;

use crate::{
    assets::APP_ICON,
    data::{Backend, DataSource},
    kit,
    model::{Ban, BanService},
    theme::Theme,
    ui::{day_and_time, icon, text_button},
};

pub struct SignedIn;

pub struct LoginView {
    source: DataSource,
    email: Entity<InputState>,
    password: Entity<InputState>,
    busy: bool,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SignedIn> for LoginView {}

impl LoginView {
    pub fn new(backend: &Entity<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let email = cx.new(|cx| InputState::new(window, cx).placeholder("you@example.com"));
        let password = cx.new(|cx| InputState::new(window, cx).masked(true));
        let submit = |view: &mut Self, _: &_, event: &InputEvent, window: &mut Window, cx: &mut Context<Self>| {
            match event {
                InputEvent::PressEnter { .. } => view.sign_in(window, cx),
                InputEvent::Change => cx.notify(),
                InputEvent::Focus | InputEvent::Blur => {}
            }
        };
        let subscriptions = vec![
            cx.subscribe_in(&email, window, submit),
            cx.subscribe_in(&password, window, submit),
        ];
        Self {
            source: backend.read(cx).source.clone(),
            email,
            password,
            busy: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.email.focus_handle(cx).focus(window, cx);
    }

    fn filled(&self, cx: &App) -> bool {
        !self.email.read(cx).value().trim().is_empty() && !self.password.read(cx).value().is_empty()
    }

    fn sign_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || !self.filled(cx) {
            return;
        }
        self.busy = true;
        self.error = None;
        let signed_in = self.source.auth_login(
            self.email.read(cx).value().trim().to_owned(),
            self.password.read(cx).value().to_string(),
        );
        cx.spawn_in(window, async move |view, cx| {
            let signed_in = signed_in.await;
            view.update_in(cx, |view, window, cx| {
                view.busy = false;
                match signed_in {
                    Ok(user) if !user.verification && user.banned.is_none() => {
                        view.error = Some(
                            "Verify your account on warframe.market, then sign in again."
                                .to_owned(),
                        );
                    }
                    Ok(user) => {
                        view.password
                            .update(cx, |password, cx| password.set_value("", window, cx));
                        if user.banned.is_none() {
                            window.push_notification(
                                format!("Signed in as {}.", user.wfm_username),
                                cx,
                            );
                        }
                        cx.emit(SignedIn);
                    }
                    Err(error) => {
                        let reason = kit::describe(&error);
                        window.push_notification(
                            Notification::error(reason.clone()).title("Could not sign in"),
                            cx,
                        );
                        view.error = Some(reason);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }
}

fn stage(cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .bg(theme.colors.surface)
        .text_size(theme.type_scale.body)
        .text_color(theme.colors.text)
}

fn title(text: &str, cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .text_size(px(19.))
        .font_weight(theme.type_scale.title_weight)
        .text_color(theme.colors.text)
        .child(text.to_owned())
}

fn label(text: &'static str, cx: &App) -> Div {
    let theme = Theme::global(cx);
    div()
        .text_size(theme.type_scale.caption + px(1.))
        .font_weight(theme.type_scale.label_weight)
        .text_color(theme.colors.text_muted)
        .child(text)
}

impl Render for LoginView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let colors = theme.colors;
        let field = |name: &'static str, input: &Entity<InputState>| {
            div()
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(label(name, cx))
                .child(Input::new(input))
        };
        stage(cx).child(
            div()
                .w(px(360.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.))
                        .pb(px(4.))
                        .child(img(APP_ICON).size(px(64.)).rounded(px(14.)))
                        .child(title("Sign in to Quantframe", cx))
                        .child(kit::note("Use your warframe.market account.", cx)),
                )
                .child(field("Email", &self.email))
                .child(field("Password", &self.password))
                .children(self.error.clone().map(|error| {
                    kit::notice(IconName::TriangleAlert, colors.negative, error, cx)
                }))
                .child(
                    text_button(
                        "sign-in",
                        if self.busy { "Signing in" } else { "Sign in" },
                        cx,
                    )
                    .primary()
                    .w_full()
                    .h(px(32.))
                    .icon(Icon::new(IconName::LogIn).size(theme.metrics.icon))
                    .disabled(self.busy || !self.filled(cx))
                    .on_click(cx.listener(|view, _, window, cx| view.sign_in(window, cx))),
                )
                .child(
                    div().flex().justify_center().child(
                        text_button("open-market", "Open warframe.market", cx)
                            .ghost()
                            .text_color(colors.accent)
                            .on_click(|_, _, cx| cx.open_url("https://warframe.market")),
                    ),
                ),
        )
    }
}

pub fn banned(
    ban: &Ban,
    on_sign_out: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    let service = match ban.service {
        BanService::WarframeMarket => "warframe.market",
        BanService::Quantframe => "Quantframe",
    };
    let line = |name: &'static str, value: String| {
        div()
            .flex()
            .gap(px(8.))
            .child(div().w(px(60.)).flex_shrink_0().text_color(colors.text_muted).child(name))
            .child(div().flex_1().min_w_0().line_height(px(19.)).child(value))
    };
    stage(cx).child(
        div()
            .w(px(440.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(14.))
            .child(icon(IconName::ShieldBan, px(34.), colors.negative))
            .child(title(&format!("Your {service} account is suspended"), cx))
            .child(kit::note(
                "Quantframe can't trade for a suspended account, so the app is closed until the suspension ends.",
                cx,
            )
            .text_center())
            .child(
                kit::card(cx)
                    .w_full()
                    .gap(px(8.))
                    .p(px(14.))
                    .child(line(
                        "Reason",
                        ban.reason
                            .clone()
                            .filter(|reason| !reason.trim().is_empty())
                            .unwrap_or_else(|| format!("{service} gave no reason.")),
                    ))
                    .child(line(
                        "Until",
                        match &ban.until {
                            Some(until) => day_and_time(until),
                            None => "No end date was given.".to_owned(),
                        },
                    )),
            )
            .child(
                text_button("banned-sign-out", "Sign out", cx)
                    .icon(Icon::new(IconName::LogOut).size(theme.metrics.icon))
                    .on_click(on_sign_out),
            ),
    )
}

pub fn error_page(
    heading: &str,
    reason: &str,
    action: &'static str,
    on_action: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Div {
    let theme = Theme::global(cx);
    let colors = theme.colors;
    stage(cx).child(
        div()
            .w(px(520.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(14.))
            .child(icon(IconName::TriangleAlert, px(34.), colors.warning))
            .child(title(heading, cx))
            .child(
                kit::card(cx)
                    .w_full()
                    .p(px(14.))
                    .line_height(px(19.))
                    .text_color(colors.text)
                    .child(reason.to_owned()),
            )
            .child(
                kit::note(
                    "Quantframe's log files hold the details for a bug report.",
                    cx,
                )
                .text_center(),
            )
            .child(text_button("error-action", action, cx).on_click(on_action)),
    )
}
