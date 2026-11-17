use gpui_kit_assets::IconName;

/// The app's screens, in the trader's order of use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Listings,
    Orders,
    TradeMessages,
    Home,
    Chat,
    Analytics,
    Inventory,
    Settings,
    About,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenGroup {
    Trading,
    Overview,
    App,
}

impl Screen {
    pub const ALL: [Self; 9] = [
        Self::Listings,
        Self::Orders,
        Self::TradeMessages,
        Self::Home,
        Self::Chat,
        Self::Analytics,
        Self::Inventory,
        Self::Settings,
        Self::About,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Listings => "Listings",
            Self::Orders => "Orders",
            Self::TradeMessages => "Trade messages",
            Self::Home => "Home",
            Self::Chat => "Chat",
            Self::Analytics => "Analytics",
            Self::Inventory => "Inventory",
            Self::Settings => "Settings",
            Self::About => "About",
        }
    }

    pub const fn icon(self) -> IconName {
        match self {
            Self::Listings => IconName::Tags,
            Self::Orders => IconName::ArrowLeftRight,
            Self::TradeMessages => IconName::Megaphone,
            Self::Home => IconName::LayoutDashboard,
            Self::Chat => IconName::MessageCircle,
            Self::Analytics => IconName::ChartColumn,
            Self::Inventory => IconName::Package,
            Self::Settings => IconName::Settings,
            Self::About => IconName::Info,
        }
    }

    pub const fn group(self) -> ScreenGroup {
        match self {
            Self::Listings | Self::Orders | Self::TradeMessages => ScreenGroup::Trading,
            Self::Home | Self::Chat | Self::Analytics | Self::Inventory => ScreenGroup::Overview,
            Self::Settings | Self::About => ScreenGroup::App,
        }
    }
}
