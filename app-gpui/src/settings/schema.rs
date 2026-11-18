use gpui_kit_assets::IconName;

pub type Choices = &'static [(&'static str, &'static str)];

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Toggle,
    Int { min: i64, unit: &'static str },
    Decimal { unit: &'static str },
    Text { placeholder: &'static str },
    Long,
    Choice(Choices),
    Flags(Choices),
    Words { placeholder: &'static str },
}

#[derive(Clone, Copy, Debug)]
pub struct Field {
    pub path: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub kind: Kind,
}

const fn field(
    path: &'static str,
    label: &'static str,
    hint: &'static str,
    kind: Kind,
) -> Field {
    Field {
        path,
        label,
        hint,
        kind,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ListSpec {
    pub path: &'static str,
    pub title: &'static str,
    pub hint: &'static str,
    pub noun: &'static str,
    pub plural: &'static str,
    pub columns: &'static [Field],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extra {
    None,
    Reset,
    Account,
    Appearance,
    InventorySource,
    Logs,
    Analytics,
}

#[derive(Clone, Copy, Debug)]
pub struct Section {
    pub key: &'static str,
    pub group: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    pub icon: IconName,
    pub blocks: &'static [(&'static str, &'static [Field])],
    pub lists: &'static [ListSpec],
    pub extra: Extra,
}

const PLATINUM: Kind = Kind::Int {
    min: 0,
    unit: "platinum",
};
const PLATINUM_OR_OFF: Kind = Kind::Int {
    min: -1,
    unit: "platinum",
};
const COUNT: Kind = Kind::Int { min: 0, unit: "" };
const COUNT_OR_OFF: Kind = Kind::Int { min: -1, unit: "" };
const PERCENT_OR_OFF: Kind = Kind::Int {
    min: -1,
    unit: "percent",
};

const LANGUAGES: Choices = &[
    ("en", "English"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("fr", "Français"),
    ("it", "Italiano"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("pl", "Polski"),
    ("ru", "Русский"),
    ("tc", "繁體中文"),
    ("th", "ไทย"),
    ("tr", "Türkçe"),
    ("uk", "Українська"),
    ("zh", "简体中文"),
];

pub const TRADE_MODES: Choices = &[
    ("buy", "Buy"),
    ("sell", "Sell"),
    ("wishlist", "Wish list"),
    ("syndicate", "Syndicate"),
];

const BLACKLIST_MODES: Choices = &[("buy", "Buy"), ("sell", "Sell"), ("wishlist", "Wish list")];

const STOCK_MODES: Choices = &[
    ("all", "Items and rivens"),
    ("item", "Items only"),
    ("riven", "Rivens only"),
];

pub const SOUNDS: Choices = &[
    ("none", "No sound"),
    ("cat_meow.mp3", "Cat meow"),
    ("iphone_notification.mp3", "iPhone notification"),
    ("windows_notification.mp3", "Windows notification"),
    ("windows_xp_error.mp3", "Windows XP error"),
    ("windows_xp_startup.mp3", "Windows XP startup"),
];

macro_rules! notification_blocks {
    ($event:literal, $placeholders:literal) => {
        &[
            (
                "On this computer",
                &[
                    field(
                        concat!("notifications.", $event, ".system_notify.enabled"),
                        "Show a system notification",
                        "",
                        Kind::Toggle,
                    ),
                    field(
                        concat!("notifications.", $event, ".system_notify.title"),
                        "Title",
                        "",
                        Kind::Text { placeholder: "" },
                    ),
                    field(
                        concat!("notifications.", $event, ".system_notify.content"),
                        "Text",
                        $placeholders,
                        Kind::Long,
                    ),
                    field(
                        concat!("notifications.", $event, ".system_notify.sound_file"),
                        "Sound",
                        "",
                        Kind::Choice(SOUNDS),
                    ),
                    field(
                        concat!("notifications.", $event, ".system_notify.volume"),
                        "Volume",
                        "From 0 for silent to 1 for full volume.",
                        Kind::Decimal { unit: "" },
                    ),
                ],
            ),
            (
                "Discord",
                &[
                    field(
                        concat!("notifications.", $event, ".discord_notify.enabled"),
                        "Post to a Discord channel",
                        "",
                        Kind::Toggle,
                    ),
                    field(
                        concat!("notifications.", $event, ".discord_notify.webhook"),
                        "Webhook address",
                        "From the channel's Integrations settings in Discord.",
                        Kind::Text {
                            placeholder: "https://discord.com/api/webhooks/...",
                        },
                    ),
                    field(
                        concat!("notifications.", $event, ".discord_notify.user_ids"),
                        "People to mention",
                        "Discord user ids, with commas between them. They replace <MENTION>.",
                        Kind::Words {
                            placeholder: "123456789012345678",
                        },
                    ),
                    field(
                        concat!("notifications.", $event, ".discord_notify.content"),
                        "Message",
                        $placeholders,
                        Kind::Long,
                    ),
                ],
            ),
            (
                "Webhook",
                &[
                    field(
                        concat!("notifications.", $event, ".webhook_notify.enabled"),
                        "Send to a webhook",
                        "Quantframe posts the event as JSON.",
                        Kind::Toggle,
                    ),
                    field(
                        concat!("notifications.", $event, ".webhook_notify.url"),
                        "Webhook address",
                        "",
                        Kind::Text {
                            placeholder: "https://example.com/hook",
                        },
                    ),
                ],
            ),
        ]
    };
}

pub const SECTIONS: &[Section] = &[
    Section {
        key: "general",
        group: "App",
        title: "General",
        blurb: "The language, and a way back to the defaults.",
        icon: IconName::Settings,
        blocks: &[(
            "",
            &[field(
                "lang",
                "Language",
                "This app is in English so far. The backend and the Tauri app use the setting.",
                Kind::Choice(LANGUAGES),
            )],
        )],
        lists: &[],
        extra: Extra::Reset,
    },
    Section {
        key: "account",
        group: "App",
        title: "Account",
        blurb: "Your warframe.market account and the status other traders see.",
        icon: IconName::CircleUser,
        blocks: &[],
        lists: &[],
        extra: Extra::Account,
    },
    Section {
        key: "appearance",
        group: "App",
        title: "Appearance",
        blurb: "Light or dark, and what the dashboard counts.",
        icon: IconName::Palette,
        blocks: &[(
            "Dashboard",
            &[
                field(
                    "summary_settings.recent_days",
                    "Days in the recent chart",
                    "How many days the dashboard's last chart covers.",
                    Kind::Int {
                        min: 1,
                        unit: "days",
                    },
                ),
                field(
                    "summary_settings.recent_transactions",
                    "Latest transactions",
                    "How many transactions the dashboard lists.",
                    Kind::Int { min: 1, unit: "" },
                ),
            ],
        )],
        lists: &[ListSpec {
            path: "summary_settings.categories",
            title: "Dashboard categories",
            hint: "Each category totals the transactions whose item has one of its tags or types.",
            noun: "category",
            plural: "categories",
            columns: &[
                field("name", "Name", "", Kind::Text { placeholder: "Prime" }),
                field(
                    "tags",
                    "Tags",
                    "warframe.market tags, with commas between them.",
                    Kind::Words {
                        placeholder: "prime, set",
                    },
                ),
                field(
                    "types",
                    "Types",
                    "Transaction types, such as riven.",
                    Kind::Words {
                        placeholder: "riven",
                    },
                ),
                field(
                    "icon",
                    "Icon file",
                    "",
                    Kind::Text {
                        placeholder: "/imgs/categories/prime.png",
                    },
                ),
            ],
        }],
        extra: Extra::Appearance,
    },
    Section {
        key: "trading",
        group: "Live scraper",
        title: "Trading",
        blurb: "What the live scraper trades and what it does with your orders.",
        icon: IconName::ArrowLeftRight,
        blocks: &[
            (
                "",
                &[
                    field(
                        "live_scraper.general.trade_modes",
                        "Trade modes",
                        "The kinds of orders the live scraper places.",
                        Kind::Flags(TRADE_MODES),
                    ),
                    field(
                        "live_scraper.general.stock_mode",
                        "Stock to check",
                        "",
                        Kind::Choice(STOCK_MODES),
                    ),
                ],
            ),
            (
                "Orders",
                &[
                    field(
                        "live_scraper.general.auto_trade",
                        "Record trades from the game",
                        "When Warframe's log shows a finished trade, record the sale or purchase in stock without asking.",
                        Kind::Toggle,
                    ),
                    field(
                        "live_scraper.general.report_to_wfm",
                        "Report trades to warframe.market",
                        "Close the order there when a trade completes, so the trade counts on your profile.",
                        Kind::Toggle,
                    ),
                    field(
                        "live_scraper.general.auto_delete",
                        "Delete orders at start",
                        "When the live scraper starts, delete your warframe.market orders, except those for blacklisted items.",
                        Kind::Toggle,
                    ),
                    field(
                        "live_scraper.general.delete_conflicting_orders",
                        "Delete orders of other trade modes",
                        "Delete orders whose trade mode is switched off above, except those for blacklisted items.",
                        Kind::Toggle,
                    ),
                ],
            ),
        ],
        lists: &[
            ListSpec {
                path: "live_scraper.items.general.blacklist",
                title: "Blacklist",
                hint: "Items the live scraper leaves alone in the chosen trade modes. The Orders screen adds items here by name.",
                noun: "blacklisted item",
                plural: "blacklisted items",
                columns: &[
                    field(
                        "wfmId",
                        "warframe.market item id",
                        "",
                        Kind::Text {
                            placeholder: "54a74454e779892d5e5155d5",
                        },
                    ),
                    field(
                        "disabled_for",
                        "Left alone for",
                        "",
                        Kind::Flags(BLACKLIST_MODES),
                    ),
                ],
            },
            ListSpec {
                path: "live_scraper.items.general.buy_list",
                title: "Buy list",
                hint: "Items the live scraper buys up to a price you set.",
                noun: "buy list item",
                plural: "buy list items",
                columns: &[
                    field(
                        "wfmId",
                        "warframe.market item id",
                        "",
                        Kind::Text {
                            placeholder: "54a74454e779892d5e5155d5",
                        },
                    ),
                    field("max_price", "Highest price", "", PLATINUM),
                ],
            },
        ],
        extra: Extra::None,
    },
    Section {
        key: "buying",
        group: "Live scraper",
        title: "Buying items",
        blurb: "Which items the live scraper places buy orders for, and at what price. A limit of -1 is off.",
        icon: IconName::ShoppingCart,
        blocks: &[
            (
                "Which items",
                &[
                    field(
                        "live_scraper.items.wtb.volume_threshold",
                        "Lowest volume",
                        "Skip items with less trade volume than this.",
                        COUNT,
                    ),
                    field(
                        "live_scraper.items.wtb.profit_threshold",
                        "Lowest profit",
                        "Skip items whose expected profit is below this.",
                        PLATINUM,
                    ),
                    field(
                        "live_scraper.items.wtb.min_wtb_profit_margin",
                        "Lowest profit margin",
                        "Skip items whose profit margin is below this.",
                        PERCENT_OR_OFF,
                    ),
                    field(
                        "live_scraper.items.wtb.avg_price_cap",
                        "Highest average price",
                        "Skip items whose average price is above this.",
                        PLATINUM,
                    ),
                    field(
                        "live_scraper.items.wtb.trading_tax_cap",
                        "Highest trade tax",
                        "Skip items whose trade tax in credits is above this.",
                        Kind::Int {
                            min: -1,
                            unit: "credits",
                        },
                    ),
                    field(
                        "live_scraper.items.wtb.price_shift_threshold",
                        "Lowest price shift",
                        "Skip items whose price moved less than this.",
                        PLATINUM_OR_OFF,
                    ),
                ],
            ),
            (
                "How much",
                &[
                    field(
                        "live_scraper.items.wtb.max_total_price_cap",
                        "Budget for all buy orders",
                        "The live scraper stops adding buy orders at this total.",
                        PLATINUM_OR_OFF,
                    ),
                    field(
                        "live_scraper.items.wtb.buy_quantity",
                        "Quantity per order",
                        "How many of an item one buy order asks for.",
                        Kind::Int { min: 1, unit: "" },
                    ),
                    field(
                        "live_scraper.items.wtb.quantity_per_trade",
                        "Quantity per trade",
                        "How many of an item one trade hands over.",
                        Kind::Int { min: 1, unit: "" },
                    ),
                    field(
                        "live_scraper.items.wtb.max_stock_quantity",
                        "Highest stock quantity",
                        "Stop buying an item once you hold this many.",
                        COUNT_OR_OFF,
                    ),
                ],
            ),
            (
                "Following the market",
                &[
                    field(
                        "live_scraper.items.wtb.max_price_drop",
                        "Largest price increase",
                        "How far a buy price may rise in one cycle.",
                        PLATINUM_OR_OFF,
                    ),
                    field(
                        "live_scraper.items.wtb.min_listings_below",
                        "Buyers above to override",
                        "With more buy orders than this above yours, the price rises past the largest increase.",
                        COUNT_OR_OFF,
                    ),
                ],
            ),
        ],
        lists: &[],
        extra: Extra::None,
    },
    Section {
        key: "selling",
        group: "Live scraper",
        title: "Selling items",
        blurb: "When the live scraper lists your stock items, and how it follows the lowest price. A limit of -1 is off.",
        icon: IconName::Tags,
        blocks: &[(
            "",
            &[
                field(
                    "live_scraper.items.wts.min_sma",
                    "Distance below the closing average",
                    "Never list further than this below the item's closing average while other sellers are above your purchase price.",
                    PLATINUM,
                ),
                field(
                    "live_scraper.items.wts.min_profit",
                    "Lowest profit",
                    "Raise the price until the profit over the purchase price reaches this.",
                    PLATINUM,
                ),
                field(
                    "live_scraper.items.wts.min_profit_percentage",
                    "Lowest profit share",
                    "Profit as a share of the purchase price. The lowest profit applies as well.",
                    PERCENT_OR_OFF,
                ),
                field(
                    "live_scraper.items.wts.max_price_drop",
                    "Largest price drop",
                    "How far a sell price may fall in one cycle.",
                    PLATINUM_OR_OFF,
                ),
                field(
                    "live_scraper.items.wts.min_listings_below",
                    "Listings below to override",
                    "With more sellers than this below your price, the price falls past the largest drop.",
                    COUNT_OR_OFF,
                ),
            ],
        )],
        lists: &[],
        extra: Extra::None,
    },
    Section {
        key: "rivens",
        group: "Live scraper",
        title: "Rivens",
        blurb: "How the live scraper prices the rivens in stock.",
        icon: IconName::Gavel,
        blocks: &[(
            "",
            &[
                field(
                    "live_scraper.rivens.wts.min_profit",
                    "Lowest profit",
                    "Raise the price until the profit over the purchase price reaches this.",
                    PLATINUM,
                ),
                field(
                    "live_scraper.rivens.wts.threshold_percentage",
                    "Price spread",
                    "The average leaves out auctions that cost more than the cheapest one times 1 plus this value.",
                    Kind::Decimal { unit: "" },
                ),
                field(
                    "live_scraper.rivens.wts.max_results",
                    "Auctions to average",
                    "How many of the cheapest similar auctions set the price.",
                    Kind::Int { min: 1, unit: "" },
                ),
                field(
                    "live_scraper.rivens.general.update_interval",
                    "Time between checks",
                    "",
                    Kind::Int {
                        min: 1,
                        unit: "seconds",
                    },
                ),
                field(
                    "live_scraper.rivens.wts.use_predict",
                    "Use the price model",
                    "Price a riven with the prediction model in place of the average of live auctions.",
                    Kind::Toggle,
                ),
            ],
        )],
        lists: &[],
        extra: Extra::None,
    },
    Section {
        key: "syndicates",
        group: "Live scraper",
        title: "Syndicates",
        blurb: "Which syndicate items the live scraper sells, and from which syndicates. A limit of -1 is off.",
        icon: IconName::Handshake,
        blocks: &[(
            "",
            &[
                field(
                    "live_scraper.syndicate.wts.volume_threshold",
                    "Lowest volume",
                    "Skip items with less trade volume than this.",
                    COUNT,
                ),
                field(
                    "live_scraper.syndicate.wts.min_price",
                    "Lowest price",
                    "",
                    PLATINUM,
                ),
                field(
                    "live_scraper.syndicate.wts.max_price_drop",
                    "Largest price drop",
                    "How far a sell price may fall in one cycle.",
                    PLATINUM_OR_OFF,
                ),
                field(
                    "live_scraper.syndicate.wts.min_listings_below",
                    "Listings below to override",
                    "With more sellers than this below your price, the price falls past the largest drop.",
                    COUNT_OR_OFF,
                ),
            ],
        )],
        lists: &[ListSpec {
            path: "live_scraper.syndicate.wts.syndicates",
            title: "Syndicates to sell from",
            hint: "The live scraper lists an item only while you have the standing to buy it.",
            noun: "syndicate",
            plural: "syndicates",
            columns: &[
                field(
                    "name",
                    "Name",
                    "",
                    Kind::Text {
                        placeholder: "Steel Meridian",
                    },
                ),
                field(
                    "unique_name",
                    "Game id",
                    "",
                    Kind::Text {
                        placeholder: "SteelMeridianSyndicate",
                    },
                ),
                field("standing", "Standing", "", COUNT),
                field(
                    "ignore_standing",
                    "Ignore standing",
                    "List items whatever your standing is.",
                    Kind::Toggle,
                ),
            ],
        }],
        extra: Extra::None,
    },
    Section {
        key: "new-conversation",
        group: "Notifications",
        title: "New conversation",
        blurb: "When a player starts a conversation with you in the game.",
        icon: IconName::MessageSquare,
        blocks: notification_blocks!(
            "on_new_conversation",
            "<PLAYER_NAME> is replaced when the notification goes out."
        ),
        lists: &[],
        extra: Extra::None,
    },
    Section {
        key: "chat-message",
        group: "Notifications",
        title: "Chat message",
        blurb: "When a warframe.market chat message arrives.",
        icon: IconName::MessageCircle,
        blocks: notification_blocks!(
            "on_wfm_chat_message",
            "<CHAT_NAME>, <FROM_USER> and <WFM_MESSAGE> are replaced when the notification goes out."
        ),
        lists: &[],
        extra: Extra::None,
    },
    Section {
        key: "trade",
        group: "Notifications",
        title: "Finished trade",
        blurb: "When Warframe's log shows a completed trade.",
        icon: IconName::Handshake,
        blocks: notification_blocks!(
            "on_new_trade",
            "<PLAYER_NAME>, <TR_TYPE>, <TOTAL_PLAT>, <OF_COUNT> and <RE_COUNT> are replaced when the notification goes out."
        ),
        lists: &[],
        extra: Extra::None,
    },
    Section {
        key: "inventory",
        group: "Data",
        title: "Inventory source",
        blurb: "Where Quantframe reads your in-game rivens and syndicate standing.",
        icon: IconName::Package,
        blocks: &[],
        lists: &[],
        extra: Extra::InventorySource,
    },
    Section {
        key: "logs",
        group: "Data",
        title: "Logs",
        blurb: "Warframe's log file, which Quantframe reads to see trades, and Quantframe's own logs.",
        icon: IconName::ScrollText,
        blocks: &[(
            "",
            &[field(
                "log_settings.ee_log_path",
                "Warframe log file",
                "Leave empty for the default place of EE.log.",
                Kind::Text {
                    placeholder: "Default",
                },
            )],
        )],
        lists: &[],
        extra: Extra::Logs,
    },
    Section {
        key: "http-server",
        group: "Data",
        title: "HTTP server",
        blurb: "A local server other tools call to read and change your stock.",
        icon: IconName::Server,
        blocks: &[(
            "",
            &[
                field(
                    "advanced_settings.http_server.enable",
                    "Run the HTTP server",
                    "",
                    Kind::Toggle,
                ),
                field(
                    "advanced_settings.http_server.host",
                    "Host",
                    "127.0.0.1 answers only this computer.",
                    Kind::Text {
                        placeholder: "127.0.0.1",
                    },
                ),
                field(
                    "advanced_settings.http_server.port",
                    "Port",
                    "",
                    Kind::Int { min: 1, unit: "" },
                ),
            ],
        )],
        lists: &[],
        extra: Extra::None,
    },
    Section {
        key: "debugging",
        group: "Data",
        title: "Debugging and analytics",
        blurb: "Switches for testing, and what Quantframe reports about your use.",
        icon: IconName::Bug,
        blocks: &[(
            "",
            &[field(
                "debugging.live_scraper.fake_orders",
                "Read orders from files",
                "The live scraper reads the market's orders from the fake_orders folder in place of warframe.market.",
                Kind::Toggle,
            )],
        )],
        lists: &[],
        extra: Extra::Analytics,
    },
];
