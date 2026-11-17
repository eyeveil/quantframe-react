use std::borrow::Cow;

use gpui::{App, AssetSource, Result, SharedString};

pub const APP_ICON: &str = "brand/app-icon.png";

const APP_ICON_BYTES: &[u8] = include_bytes!("../../src-tauri/icons/128x128.png");

const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/Geist-Regular.ttf"),
    include_bytes!("../assets/fonts/Geist-Medium.ttf"),
    include_bytes!("../assets/fonts/Geist-SemiBold.ttf"),
    include_bytes!("../assets/fonts/GeistMono-Regular.ttf"),
    include_bytes!("../assets/fonts/GeistMono-Medium.ttf"),
    include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf"),
    include_bytes!("../assets/fonts/IBMPlexSans-Medium.ttf"),
    include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf"),
];

gpui_kit_assets::icon_assets!(
    AppIcons,
    [
        ArrowDown,
        ArrowLeftRight,
        ArrowUp,
        ChartColumn,
        Check,
        ChevronDown,
        CircleUser,
        Clock,
        Download,
        Ellipsis,
        ExternalLink,
        Eye,
        EyeOff,
        Funnel,
        Gavel,
        Globe,
        HandCoins,
        Info,
        LayoutDashboard,
        Megaphone,
        MessageCircle,
        MessageSquare,
        Minus,
        Package,
        Play,
        Plus,
        Search,
        Settings,
        SlidersHorizontal,
        Square,
        SquarePen,
        Tags,
        Trash,
        X,
    ]
);

/// The app's icons and logo, with the gpui-component default icons behind them.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == APP_ICON {
            return Ok(Some(Cow::Borrowed(APP_ICON_BYTES)));
        }
        if let Some(bytes) = AppIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit_assets::Assets.list(path)?;
        paths.extend(AppIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

pub fn load_fonts(cx: &App) -> Result<()> {
    cx.text_system()
        .add_fonts(FONTS.iter().map(|font| Cow::Borrowed(*font)).collect())
}
