//! The About screen: the version, where to get help, the licence and who
//! made Quantframe.

use gpui::{
    App, Context, Div, Entity, IntoElement, ParentElement, Render, Styled, Window, div, img,
    prelude::*, px,
};
use gpui_kit_assets::IconName;

use crate::{
    assets::APP_ICON,
    data::{Backend, DataSource},
    kit::{self, Loaded},
    model::AppInfo,
    theme::Theme,
    ui::grid::Tone,
};

/// The places the About screen links to: icon, title, detail and address.
const LINKS: [(IconName, &str, &str, &str); 5] = [
    (
        IconName::BookOpen,
        "Guide",
        "How the live scraper, stock and settings work.",
        "https://quantframe.app",
    ),
    (
        IconName::MessagesSquare,
        "Discord",
        "Questions, bug reports and release notes.",
        "https://discord.gg/dPSFPG5Qt6",
    ),
    (
        IconName::Github,
        "Source of this build",
        "github.com/eyeveil/quantframe-react",
        "https://github.com/eyeveil/quantframe-react",
    ),
    (
        IconName::Github,
        "Kenya-DK's Quantframe",
        "The project this build comes from: github.com/Kenya-DK/quantframe-react",
        "https://github.com/Kenya-DK/quantframe-react",
    ),
    (
        IconName::Coffee,
        "Buy Kenya-DK a coffee",
        "Support the author of Quantframe.",
        "https://www.buymeacoffee.com/kenyadk",
    ),
];

const LICENSE_URL: &str = "https://www.gnu.org/licenses/gpl-3.0.html";

pub struct AboutView {
    source: DataSource,
    info: Loaded<AppInfo>,
}

impl AboutView {
    pub fn new(backend: &Entity<Backend>, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            source: backend.read(cx).source.clone(),
            info: Loaded::Loading,
        };
        view.load(cx);
        view
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        let info = self.source.app_get_app_info();
        cx.spawn(async move |view, cx| {
            let info = Loaded::from_result(info.await);
            view.update(cx, |view, cx| {
                view.info = info;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn identity(&self, cx: &App) -> Div {
        let theme = Theme::global(cx);
        let colors = theme.colors;
        let version = match &self.info {
            Loaded::Ready(info) => {
                let mut version = format!("Version {}", info.version);
                if info.is_pre_release {
                    version.push_str(", pre-release");
                }
                if info.is_dev {
                    version.push_str(", development build");
                }
                version
            }
            Loaded::Loading => "Reading the version".to_owned(),
            Loaded::Failed(reason) => format!("The version is unknown: {reason}"),
        };
        div()
            .flex()
            .items_center()
            .gap(px(16.))
            .child(img(APP_ICON).size(px(72.)).rounded(px(16.)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_size(px(24.))
                            .font_weight(theme.type_scale.title_weight)
                            .text_color(colors.text)
                            .child("Quantframe"),
                    )
                    .child(div().text_color(colors.text_muted).child(version))
                    .child(kit::note(
                        "A trading tool for Warframe that prices and lists your stock on warframe.market.",
                        cx,
                    )),
            )
    }

    fn build_facts(info: &AppInfo, cx: &App) -> Div {
        kit::card(cx)
            .px(px(14.))
            .py(px(8.))
            .child(kit::fact("Interface", "GPUI, written in Rust", Tone::Plain, cx))
            .child(kit::fact(
                "Data",
                if info.fixture_mode {
                    "Fixtures, no network requests"
                } else {
                    "Your account, warframe.market and Quantframe's server"
                },
                Tone::Plain,
                cx,
            ))
            .child(kit::fact(
                "Database",
                if info.use_temp_db {
                    "Temporary"
                } else {
                    "On this computer"
                },
                Tone::Plain,
                cx,
            ))
            .child(kit::fact("Author", info.authors.clone(), Tone::Plain, cx))
    }
}

fn links(cx: &App) -> Div {
    let mut card = kit::card(cx).overflow_hidden();
    for (index, (icon, title, detail, url)) in LINKS.into_iter().enumerate() {
        if index > 0 {
            card = card.child(kit::rule(cx));
        }
        card = card.child(
            kit::link_row(("about-link", index), icon, title, detail, cx)
                .on_click(move |_, _, cx| cx.open_url(url)),
        );
    }
    card
}

fn section(title: &'static str, body: impl IntoElement, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(7.))
        .child(div().px(px(2.)).child(kit::overline(title, cx)))
        .child(body)
}

fn paragraph(text: &'static str, cx: &App) -> Div {
    div()
        .line_height(px(20.))
        .text_color(Theme::global(cx).colors.text_muted)
        .child(text)
}

impl Render for AboutView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let year = kit::date_days_ago(0)[..4].to_owned();
        let license = kit::card(cx)
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .p(px(14.))
                    .child(kit::heading("GNU General Public License, version 3", cx))
                    .child(paragraph(
                        "Quantframe is free software. You may use it, share it and change it under the GPL-3.0, and anything you pass on has to stay under the same licence with its source. It comes with no warranty.",
                        cx,
                    )),
            )
            .child(kit::rule(cx))
            .child(
                kit::link_row(
                    "about-license",
                    IconName::Scale,
                    "Read the licence",
                    "gnu.org/licenses/gpl-3.0.html",
                    cx,
                )
                .on_click(|_, _, cx| cx.open_url(LICENSE_URL)),
            );
        let credits = kit::card(cx)
            .gap(px(8.))
            .p(px(14.))
            .child(paragraph(
                "Kenya-DK wrote Quantframe and maintains it with its contributors. This build keeps the backend and replaces the interface with one written in GPUI.",
                cx,
            ))
            .child(paragraph(
                "The interface uses GPUI from Zed Industries, gpui-component, the Geist typeface under the SIL Open Font License and Lucide icons under the ISC License.",
                cx,
            ))
            .child(kit::note(
                format!("Copyright 2023 to {year} Kenya-DK and the Quantframe contributors."),
                cx,
            ));
        kit::scroll_page("about", cx).child(
            div()
                .w_full()
                .max_w(px(720.))
                .flex()
                .flex_col()
                .gap(px(18.))
                .py(px(10.))
                .px(px(8.))
                .child(self.identity(cx))
                .children(
                    self.info
                        .ready()
                        .map(|info| section("This build", Self::build_facts(info, cx), cx)),
                )
                .child(section("Help and source", links(cx), cx))
                .child(section("Licence", license, cx))
                .child(section("Credits", credits, cx))
                .child(kit::note(
                    "Quantframe is a third-party app and is not affiliated with Digital Extremes.",
                    cx,
                )),
        )
    }
}
