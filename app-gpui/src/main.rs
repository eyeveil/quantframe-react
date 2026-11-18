mod about;
mod analytics;
mod assets;
mod auth;
mod chart;
mod chat;
mod debug;
mod home;
mod inventory;
mod kit;
mod listings;
mod orders;
mod prefs;
mod settings;
mod shell;
mod theme;
mod trade_messages;
mod ui;

use gpui::{
    App, Bounds, Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window,
    WindowBounds, WindowDecorations, WindowOptions, div, prelude::*, px, size,
};
use gpui_component::{Root, TitleBar};

use quantframe_gpui::{data, model};

use data::{Backend, BackendStatus, DataSource, StartOptions};
use shell::AppShell;
use theme::Theme;

/// gpui-component sizes its controls in rems and draws their labels at
/// 0.875rem, so this rem size puts those labels at the theme's body size.
const KIT_LABEL_REMS: f32 = 0.875;

struct AppRoot {
    backend: Entity<Backend>,
    shell: Option<Entity<AppShell>>,
    _subscription: Subscription,
}

impl AppRoot {
    fn new(backend: Entity<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe_in(&backend, window, |root, _, window, cx| {
            root.open_shell_when_ready(window, cx);
        });
        let mut root = Self {
            backend,
            shell: None,
            _subscription: subscription,
        };
        root.open_shell_when_ready(window, cx);
        root
    }

    fn open_shell_when_ready(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shell.is_none() && self.backend.read(cx).status == BackendStatus::Ready {
            let backend = self.backend.clone();
            self.shell = Some(cx.new(|cx| AppShell::new(backend, window, cx)));
        }
        cx.notify();
    }
}

impl Render for AppRoot {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(shell) = &self.shell {
            return shell.clone().into_any_element();
        }
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let step = match &self.backend.read(cx).status {
            BackendStatus::Failed(error) => {
                return auth::error_page(
                    "Quantframe could not start",
                    error,
                    "Quit Quantframe",
                    |_, _, cx| cx.quit(),
                    cx,
                )
                .font_family(theme.type_scale.family.clone())
                .into_any_element();
            }
            BackendStatus::Starting(step) => step.clone().unwrap_or_default(),
            BackendStatus::Ready => String::new(),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .bg(colors.window)
            .font_family(theme.type_scale.family.clone())
            .child(
                div()
                    .text_size(theme.type_scale.title)
                    .font_weight(theme.type_scale.title_weight)
                    .text_color(colors.text)
                    .child("Starting Quantframe"),
            )
            .child(
                div()
                    .text_size(theme.type_scale.body)
                    .text_color(colors.text_muted)
                    .child(step),
            )
            .into_any_element()
    }
}

fn main() {
    let (source, events) = match DataSource::start(StartOptions::from_env()) {
        Ok(started) => started,
        Err(error) => {
            eprintln!("Could not start the backend: {error}");
            std::process::exit(1);
        }
    };
    gpui_platform::application()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            if let Err(error) = assets::load_fonts(cx) {
                eprintln!("Could not load the bundled fonts: {error}");
            }
            Theme::init(cx);

            let backend = Backend::new(source, events, cx);
            let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1100.), px(640.))),
                window_decorations: Some(WindowDecorations::Client),
                app_id: Some("quantframe".to_owned()),
                ..TitleBar::window_options()
            };
            let opened = cx.open_window(options, |window, cx| {
                window.set_window_title("Quantframe");
                window.set_rem_size(Theme::global(cx).type_scale.body / KIT_LABEL_REMS);
                let root = cx.new(|cx| AppRoot::new(backend, window, cx));
                cx.new(|cx| Root::new(root, window, cx))
            });
            if let Err(error) = opened {
                eprintln!("Could not open the Quantframe window: {error}");
                cx.quit();
                return;
            }
            cx.activate(true);
        });
}
