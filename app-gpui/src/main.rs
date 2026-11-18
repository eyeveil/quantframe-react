mod assets;
mod data;
mod listings;
mod model;
mod orders;
mod prefs;
mod preview;
mod shell;
mod theme;
mod trade_messages;
mod ui;

use std::rc::Rc;

use gpui::{App, Bounds, WindowBounds, WindowDecorations, WindowOptions, prelude::*, px, size};
use gpui_component::{Root, TitleBar};

use data::{DataSource, FixtureSource};
use shell::AppShell;
use theme::Theme;

/// gpui-component sizes its controls in rems and draws their labels at
/// 0.875rem, so this rem size puts those labels at the theme's body size.
const KIT_LABEL_REMS: f32 = 0.875;

fn main() {
    gpui_platform::application()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);
            if let Err(error) = assets::load_fonts(cx) {
                eprintln!("Could not load the bundled fonts: {error}");
            }
            Theme::init(cx);

            let source: Rc<dyn DataSource> = Rc::new(FixtureSource::new());
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
                let shell = cx.new(|cx| AppShell::new(source, window, cx));
                cx.new(|cx| Root::new(shell, window, cx))
            });
            if let Err(error) = opened {
                eprintln!("Could not open the Quantframe window: {error}");
                cx.quit();
                return;
            }
            cx.activate(true);
        });
}
