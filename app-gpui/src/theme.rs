use std::sync::Arc;

use gpui::{
    App, FontFeatures, FontWeight, Global, Hsla, Pixels, SharedString, WindowAppearance, hsla, px,
    rgb,
};
use gpui_component::{Theme as KitTheme, ThemeMode};

use crate::{model::StockStatus, prefs};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppearanceChoice {
    #[default]
    System,
    Light,
    Dark,
}

impl AppearanceChoice {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|choice| choice.as_str().eq_ignore_ascii_case(value.trim()))
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "Match the system",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }

    fn resolve(self, system: WindowAppearance) -> Appearance {
        match self {
            Self::Light => Appearance::Light,
            Self::Dark => Appearance::Dark,
            Self::System => match system {
                WindowAppearance::Dark | WindowAppearance::VibrantDark => Appearance::Dark,
                WindowAppearance::Light | WindowAppearance::VibrantLight => Appearance::Light,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub appearance: Appearance,
    pub choice: AppearanceChoice,
    pub colors: Palette,
    pub type_scale: TypeScale,
    pub metrics: Metrics,
}

impl Global for Theme {}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub window: Hsla,
    pub chrome: Hsla,
    pub surface: Hsla,
    pub raised: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_faint: Hsla,
    pub accent: Hsla,
    pub on_accent: Hsla,
    pub positive: Hsla,
    pub negative: Hsla,
    pub warning: Hsla,
    pub info: Hsla,
    pub row_hover: Hsla,
    pub row_selected: Hsla,
    pub control_hover: Hsla,
    pub control_active: Hsla,
    pub segment_active: Hsla,
    pub overlay: Hsla,
}

#[derive(Clone, Debug)]
pub struct TypeScale {
    pub family: SharedString,
    pub caption: Pixels,
    pub body: Pixels,
    pub title: Pixels,
    pub figure: Pixels,
    pub label_weight: FontWeight,
    pub title_weight: FontWeight,
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub row_height: Pixels,
    pub header_height: Pixels,
    pub control_height: Pixels,
    pub title_bar_height: Pixels,
    pub status_bar_height: Pixels,
    pub sidebar_width: Pixels,
    pub panel_width: Pixels,
    pub radius: Pixels,
    pub radius_large: Pixels,
    pub gutter: Pixels,
    pub cell_padding: Pixels,
    pub icon: Pixels,
    pub action_button: Pixels,
    pub focus_ring: Pixels,
}

fn hex(value: u32) -> Hsla {
    rgb(value).into()
}

pub fn tabular_figures() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".to_owned(), 1)]))
}

impl Theme {
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn init(cx: &mut App) {
        let choice = std::env::var("QF_THEME")
            .ok()
            .as_deref()
            .and_then(AppearanceChoice::parse)
            .unwrap_or_else(|| prefs::load().appearance);
        Self::with_choice(choice, cx.window_appearance()).install(cx);
    }

    pub fn set_choice(choice: AppearanceChoice, system: WindowAppearance, cx: &mut App) {
        prefs::update(|prefs| prefs.appearance = choice);
        Self::with_choice(choice, system).install(cx);
    }

    pub fn follow_system(system: WindowAppearance, cx: &mut App) {
        let choice = Self::global(cx).choice;
        if choice == AppearanceChoice::System {
            Self::with_choice(choice, system).install(cx);
        }
    }

    fn with_choice(choice: AppearanceChoice, system: WindowAppearance) -> Self {
        let appearance = choice.resolve(system);
        Self {
            appearance,
            choice,
            colors: match appearance {
                Appearance::Dark => Palette::graphite(),
                Appearance::Light => Palette::paper(),
            },
            type_scale: TypeScale {
                family: "Geist".into(),
                caption: px(11.),
                body: px(13.),
                title: px(15.),
                figure: px(20.),
                label_weight: FontWeight::MEDIUM,
                title_weight: FontWeight::SEMIBOLD,
            },
            metrics: Metrics {
                row_height: px(36.),
                header_height: px(30.),
                control_height: px(28.),
                title_bar_height: px(46.),
                status_bar_height: px(28.),
                sidebar_width: px(216.),
                panel_width: px(320.),
                radius: px(6.),
                radius_large: px(10.),
                gutter: px(16.),
                cell_padding: px(10.),
                icon: px(15.),
                action_button: px(26.),
                focus_ring: px(2.),
            },
        }
    }

    pub fn status_color(&self, status: StockStatus) -> Hsla {
        let colors = &self.colors;
        match status {
            StockStatus::Live => colors.positive,
            StockStatus::Pending | StockStatus::Unknown => colors.text_muted,
            StockStatus::InActive => colors.text_faint,
            StockStatus::ToLowProfit
            | StockStatus::SMALimit
            | StockStatus::OrderLimit
            | StockStatus::MaxPriceDrop
            | StockStatus::InsufficientStanding => colors.warning,
            StockStatus::Overpriced | StockStatus::Underpriced => colors.info,
            StockStatus::NoSellers | StockStatus::NoBuyers | StockStatus::Error => colors.negative,
        }
    }

    pub fn install(self, cx: &mut App) {
        let mode = match self.appearance {
            Appearance::Dark => ThemeMode::Dark,
            Appearance::Light => ThemeMode::Light,
        };
        KitTheme::change(mode, None, cx);
        let colors = self.colors;
        let family = self.type_scale.family.clone();
        let body = self.type_scale.body;
        let radius = self.metrics.radius;
        let radius_large = self.metrics.radius_large;
        KitTheme::update(cx, |kit| {
            kit.font_family = family;
            kit.font_size = body;
            kit.radius = radius;
            kit.radius_lg = radius_large;
            kit.shadow = false;

            let kit = &mut kit.colors;
            kit.background = colors.window;
            kit.foreground = colors.text;
            kit.border = colors.border;
            kit.input = colors.border_strong;
            kit.ring = colors.accent;
            kit.caret = colors.accent;
            kit.selection = colors.accent.opacity(0.35);
            kit.muted = colors.raised;
            kit.muted_foreground = colors.text_muted;
            kit.accent = colors.control_hover;
            kit.accent_foreground = colors.text;
            kit.popover = colors.raised;
            kit.popover_foreground = colors.text;
            kit.link = colors.accent;
            kit.overlay = colors.overlay;
            kit.skeleton = colors.control_hover;

            kit.primary = colors.accent;
            kit.primary_hover = colors.accent.opacity(0.88);
            kit.primary_active = colors.accent.opacity(0.76);
            kit.primary_foreground = colors.on_accent;
            kit.button_primary = colors.accent;
            kit.button_primary_hover = colors.accent.opacity(0.88);
            kit.button_primary_active = colors.accent.opacity(0.76);
            kit.button_primary_foreground = colors.on_accent;
            kit.secondary = colors.raised;
            kit.secondary_hover = colors.control_hover;
            kit.secondary_active = colors.control_active;
            kit.secondary_foreground = colors.text;
            kit.button = colors.raised;
            kit.button_hover = colors.border;
            kit.button_active = colors.border_strong;
            kit.button_foreground = colors.text;
            kit.button_secondary = colors.raised;
            kit.button_secondary_hover = colors.border;
            kit.button_secondary_active = colors.border_strong;
            kit.button_secondary_foreground = colors.text;
            kit.button_danger = colors.negative;
            kit.button_danger_hover = colors.negative.opacity(0.88);
            kit.button_danger_active = colors.negative.opacity(0.76);
            kit.button_danger_foreground = colors.on_accent;
            kit.danger = colors.negative;
            kit.success = colors.positive;
            kit.warning = colors.warning;
            kit.info = colors.info;

            kit.list = colors.raised;
            kit.list_hover = colors.control_hover;
            kit.list_active = colors.row_selected;
            kit.list_active_border = colors.accent.opacity(0.0);
            kit.switch = colors.border_strong;

            kit.table = colors.surface;
            kit.table_head = colors.surface;
            kit.table_head_foreground = colors.text_muted;
            kit.table_even = colors.surface.opacity(0.0);
            kit.table_hover = colors.row_hover;
            kit.table_active = colors.row_selected;
            kit.table_active_border = colors.accent.opacity(0.0);
            kit.table_row_border = colors.border;
            kit.scrollbar = colors.surface.opacity(0.0);
            kit.scrollbar_thumb = colors.text.opacity(0.22);
            kit.scrollbar_thumb_hover = colors.text.opacity(0.34);
            kit.title_bar = colors.chrome;
            kit.title_bar_border = colors.border;
        });
        cx.set_global(self);
        cx.refresh_windows();
    }
}

impl Palette {
    fn graphite() -> Self {
        let text = hex(0xf2f2f4);
        let accent = hex(0x0a84ff);
        Self {
            window: hex(0x1c1c1e),
            chrome: hex(0x242426),
            surface: hex(0x1c1c1e),
            raised: hex(0x2a2a2d),
            border: hex(0x323236),
            border_strong: hex(0x45454a),
            text,
            text_muted: hex(0xa1a1aa),
            text_faint: hex(0x74747d),
            accent,
            on_accent: hex(0xffffff),
            positive: hex(0x32d15d),
            negative: hex(0xff5c52),
            warning: hex(0xffb340),
            info: hex(0x6ac4ff),
            row_hover: text.opacity(0.045),
            row_selected: accent.opacity(0.2),
            control_hover: text.opacity(0.08),
            control_active: text.opacity(0.13),
            segment_active: hex(0x4a4a50),
            overlay: hsla(0., 0., 0., 0.5),
        }
    }

    fn paper() -> Self {
        let text = hex(0x1d1d1f);
        let accent = hex(0x0071e3);
        Self {
            window: hex(0xffffff),
            chrome: hex(0xf5f5f7),
            surface: hex(0xffffff),
            raised: hex(0xf0f0f3),
            border: hex(0xe3e3e8),
            border_strong: hex(0xcdcdd4),
            text,
            text_muted: hex(0x5f5f67),
            text_faint: hex(0x85858d),
            accent,
            on_accent: hex(0xffffff),
            positive: hex(0x16873a),
            negative: hex(0xd3301f),
            warning: hex(0xb26800),
            info: hex(0x0b74b0),
            row_hover: text.opacity(0.04),
            row_selected: accent.opacity(0.13),
            control_hover: text.opacity(0.06),
            control_active: text.opacity(0.1),
            segment_active: hex(0xffffff),
            overlay: hsla(0., 0., 0., 0.28),
        }
    }
}
