//! The design tokens every view reads. Views take colours, sizes and layout
//! choices from [`Theme`] and never hard-code them.

use gpui::{App, FontWeight, Global, Hsla, Pixels, SharedString, px, rgb};
use gpui_component::{Theme as KitTheme, ThemeMode};

use crate::model::StockStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    A,
    B,
    C,
}

impl Variant {
    /// Reads `QF_VARIANT`. Anything other than `b` or `c` selects variant A.
    pub fn from_env() -> Self {
        match std::env::var("QF_VARIANT").as_deref() {
            Ok("b" | "B") => Self::B,
            Ok("c" | "C") => Self::C,
            _ => Self::A,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub variant: Variant,
    pub colors: Palette,
    pub type_scale: TypeScale,
    pub metrics: Metrics,
    pub layout: Layout,
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
    pub row_stripe: Hsla,
    pub control_hover: Hsla,
    pub control_active: Hsla,
}

#[derive(Clone, Debug)]
pub struct TypeScale {
    pub family: SharedString,
    /// The family for prices and quantities, so digits line up in columns.
    pub numeric_family: SharedString,
    pub caption: Pixels,
    pub body: Pixels,
    pub emphasis: Pixels,
    pub title: Pixels,
    pub figure: Pixels,
    pub label_weight: FontWeight,
    pub title_weight: FontWeight,
    /// Column headers and group labels in capitals.
    pub capital_labels: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub row_height: Pixels,
    pub header_height: Pixels,
    pub control_height: Pixels,
    pub title_bar_height: Pixels,
    pub status_bar_height: Pixels,
    pub radius: Pixels,
    pub radius_large: Pixels,
    pub gutter: Pixels,
    pub cell_padding: Pixels,
    pub icon: Pixels,
    pub action_button: Pixels,
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub navigation: Navigation,
    pub row: RowLayout,
    pub status: StatusStyle,
    pub actions: ActionStyle,
    pub inspector_width: Option<Pixels>,
    pub striped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Navigation {
    Sidebar { width: Pixels },
    TopBar,
    Rail { width: Pixels },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowLayout {
    SingleLine,
    /// The item name with its rank and market slug on a second line.
    TwoLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusStyle {
    Dot,
    OutlinedTag,
    ColoredText,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionStyle {
    /// Every row action as an icon button.
    Icons,
    /// A labelled Sell button, then icon buttons for the rest.
    LabelledSell,
}

fn hex(value: u32) -> Hsla {
    rgb(value).into()
}

impl Theme {
    pub fn for_variant(variant: Variant) -> Self {
        match variant {
            Variant::A => Self::graphite(),
            Variant::B => Self::ink(),
            Variant::C => Self::ember(),
        }
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Neutral graphite with a blue accent, a sidebar and medium rows.
    fn graphite() -> Self {
        let text = hex(0xf2f2f4);
        let accent = hex(0x0a84ff);
        Self {
            variant: Variant::A,
            colors: Palette {
                window: hex(0x1c1c1e),
                chrome: hex(0x242426),
                surface: hex(0x1c1c1e),
                raised: hex(0x2a2a2d),
                border: hex(0x323236),
                border_strong: hex(0x45454a),
                text,
                text_muted: hex(0xa1a1aa),
                text_faint: hex(0x6f6f78),
                accent,
                on_accent: hex(0xffffff),
                positive: hex(0x32d15d),
                negative: hex(0xff5c52),
                warning: hex(0xffb340),
                info: hex(0x6ac4ff),
                row_hover: text.opacity(0.04),
                row_selected: accent.opacity(0.18),
                row_stripe: text.opacity(0.0),
                control_hover: text.opacity(0.08),
                control_active: text.opacity(0.13),
            },
            type_scale: TypeScale {
                family: "Geist".into(),
                numeric_family: "Geist".into(),
                caption: px(11.),
                body: px(13.),
                emphasis: px(13.),
                title: px(15.),
                figure: px(20.),
                label_weight: FontWeight::MEDIUM,
                title_weight: FontWeight::SEMIBOLD,
                capital_labels: false,
            },
            metrics: Metrics {
                row_height: px(36.),
                header_height: px(30.),
                control_height: px(28.),
                title_bar_height: px(46.),
                status_bar_height: px(28.),
                radius: px(6.),
                radius_large: px(10.),
                gutter: px(16.),
                cell_padding: px(10.),
                icon: px(15.),
                action_button: px(26.),
            },
            layout: Layout {
                navigation: Navigation::Sidebar { width: px(216.) },
                row: RowLayout::SingleLine,
                status: StatusStyle::Dot,
                actions: ActionStyle::Icons,
                inspector_width: None,
                striped: false,
            },
        }
    }

    /// Blue-black with a teal accent, top navigation, dense striped rows and
    /// an item panel on the right.
    fn ink() -> Self {
        let text = hex(0xd9dfe7);
        let accent = hex(0x35c9b8);
        Self {
            variant: Variant::B,
            colors: Palette {
                window: hex(0x0b0f14),
                chrome: hex(0x10151c),
                surface: hex(0x0d1117),
                raised: hex(0x161c25),
                border: hex(0x202834),
                border_strong: hex(0x2f3a49),
                text,
                text_muted: hex(0x8a96a6),
                text_faint: hex(0x5a6575),
                accent,
                on_accent: hex(0x04211e),
                positive: hex(0x4ade80),
                negative: hex(0xf87171),
                warning: hex(0xf5b83d),
                info: hex(0x6aa9f7),
                row_hover: text.opacity(0.05),
                row_selected: accent.opacity(0.14),
                row_stripe: text.opacity(0.022),
                control_hover: text.opacity(0.08),
                control_active: text.opacity(0.14),
            },
            type_scale: TypeScale {
                family: "IBM Plex Sans".into(),
                numeric_family: "IBM Plex Mono".into(),
                caption: px(10.5),
                body: px(12.),
                emphasis: px(12.),
                title: px(13.),
                figure: px(15.),
                label_weight: FontWeight::MEDIUM,
                title_weight: FontWeight::SEMIBOLD,
                capital_labels: true,
            },
            metrics: Metrics {
                row_height: px(27.),
                header_height: px(26.),
                control_height: px(24.),
                title_bar_height: px(40.),
                status_bar_height: px(26.),
                radius: px(3.),
                radius_large: px(4.),
                gutter: px(10.),
                cell_padding: px(8.),
                icon: px(13.),
                action_button: px(22.),
            },
            layout: Layout {
                navigation: Navigation::TopBar,
                row: RowLayout::SingleLine,
                status: StatusStyle::OutlinedTag,
                actions: ActionStyle::Icons,
                inspector_width: Some(px(320.)),
                striped: true,
            },
        }
    }

    /// Warm charcoal with a gold accent, an icon rail, large type and
    /// two-line rows.
    fn ember() -> Self {
        let text = hex(0xf3eee6);
        let accent = hex(0xe0aa45);
        Self {
            variant: Variant::C,
            colors: Palette {
                window: hex(0x151311),
                chrome: hex(0x1b1916),
                surface: hex(0x151311),
                raised: hex(0x23201c),
                border: hex(0x2e2a25),
                border_strong: hex(0x433e37),
                text,
                text_muted: hex(0xa79f93),
                text_faint: hex(0x756e63),
                accent,
                on_accent: hex(0x1d1503),
                positive: hex(0x86c97f),
                negative: hex(0xec7663),
                warning: hex(0xee8f45),
                info: hex(0x7fb0e6),
                row_hover: text.opacity(0.04),
                row_selected: accent.opacity(0.13),
                row_stripe: text.opacity(0.0),
                control_hover: text.opacity(0.08),
                control_active: text.opacity(0.13),
            },
            type_scale: TypeScale {
                family: "Geist".into(),
                numeric_family: "Geist".into(),
                caption: px(12.),
                body: px(14.),
                emphasis: px(15.),
                title: px(22.),
                figure: px(28.),
                label_weight: FontWeight::MEDIUM,
                title_weight: FontWeight::SEMIBOLD,
                capital_labels: false,
            },
            metrics: Metrics {
                row_height: px(52.),
                header_height: px(34.),
                control_height: px(34.),
                title_bar_height: px(64.),
                status_bar_height: px(30.),
                radius: px(8.),
                radius_large: px(12.),
                gutter: px(24.),
                cell_padding: px(12.),
                icon: px(17.),
                action_button: px(32.),
            },
            layout: Layout {
                navigation: Navigation::Rail { width: px(60.) },
                row: RowLayout::TwoLine,
                status: StatusStyle::ColoredText,
                actions: ActionStyle::LabelledSell,
                inspector_width: None,
                striped: false,
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

    /// Installs these tokens as the app theme and maps them onto the
    /// gpui-component theme, so its inputs, buttons, table and scrollbars
    /// match the views drawn here.
    pub fn install(self, cx: &mut App) {
        KitTheme::change(ThemeMode::Dark, None, cx);
        let colors = self.colors;
        let family = self.type_scale.family.clone();
        let numeric_family = self.type_scale.numeric_family.clone();
        let body = self.type_scale.body;
        let radius = self.metrics.radius;
        let radius_large = self.metrics.radius_large;
        KitTheme::update(cx, |kit| {
            kit.font_family = family;
            kit.mono_font_family = numeric_family;
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
            kit.danger = colors.negative;
            kit.success = colors.positive;
            kit.warning = colors.warning;
            kit.info = colors.info;

            kit.table = colors.surface;
            kit.table_head = colors.surface;
            kit.table_head_foreground = colors.text_muted;
            kit.table_even = colors.row_stripe;
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
    }
}
