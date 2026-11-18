use std::cmp::Ordering;

use gpui::{AnyElement, App, IntoElement, ParentElement, prelude::FluentBuilder};
use gpui_kit_assets::IconName;

use super::row_key;
use crate::{
    data::{DataKind, DataSource},
    listings::{attribute_lines, attributes_cell, attributes_text, words},
    model::{AuctionOverview, ListParams, WfmAuction},
    theme::Theme,
    ui::{
        IconButton, cells, day_and_time,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ColumnSpec, Emit, Load, MenuEntry, Outcome, PrimaryAction, Query,
            Stat, Tone, ToolbarItem, overflow_button,
        },
        number,
        panel::{Fact, PanelAction, PanelModel},
        prompts,
    },
};

#[derive(Clone, Debug)]
pub enum AuctionAction {
    Refresh,
    DeleteAll,
    DeletedAll,
    Import(u64),
    Imported { key: u64, bought: i64 },
    Delete(u64),
    Deleted(u64),
    Details(u64),
    CopyName(u64),
    OpenMarket(u64),
}

#[derive(Default)]
pub struct AuctionsData {
    rows: Vec<WfmAuction>,
    overview: AuctionOverview,
    auctions_limit: i64,
}

#[derive(Default)]
pub struct Auctions {
    data: AuctionsData,
}

fn title(auction: &WfmAuction) -> String {
    let weapon = if auction.name == auction.weapon_url {
        words(&auction.name)
    } else {
        auction.name.clone()
    };
    match &auction.mod_name {
        Some(mod_name) => {
            let mut chars = mod_name.chars();
            let mod_name: String = chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default();
            format!("{weapon} {mod_name}")
        }
        None => weapon,
    }
}

impl Auctions {
    fn auction(&self, key: u64) -> Option<WfmAuction> {
        self.data
            .rows
            .iter()
            .find(|auction| row_key(&auction.id) == key)
            .cloned()
    }
}

impl Catalog for Auctions {
    type Row = WfmAuction;
    type Action = AuctionAction;
    type Data = AuctionsData;

    const NOUN: &'static str = "auctions";
    const SEARCH_PLACEHOLDER: &'static str = "Search auctions";
    const EMPTY_HINT: &'static str =
        "Auctions appear here once the live scraper lists your rivens.";
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[DataKind::WfmAuctions, DataKind::User];

    fn load(&self, source: &DataSource) -> Load<Self::Data> {
        let rows = source.get_wfm_auctions_pagination(&ListParams {
            limit: -1,
            ..ListParams::default()
        });
        let overview = source.get_wfm_auctions_overview();
        let user = source.user();
        Box::pin(async move {
            Ok(AuctionsData {
                rows: rows.await?.results,
                overview: overview.await?,
                auctions_limit: user.await?.auctions_limit,
            })
        })
    }

    fn apply(&mut self, data: Self::Data) {
        self.data = data;
    }

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("riven", "Riven", 216.).sortable(),
            ColumnSpec::new("attributes", "Attributes", 200.).flex(),
            ColumnSpec::new("mastery", "MR", 56.)
                .end()
                .sortable()
                .droppable(3),
            ColumnSpec::new("re_rolls", "Rolls", 62.)
                .end()
                .sortable()
                .droppable(2),
            ColumnSpec::new("starting_price", "Start price", 104.)
                .end()
                .sortable(),
            ColumnSpec::new("buyout_price", "Buyout", 92.)
                .end()
                .sortable()
                .droppable(1),
            ColumnSpec::new("top_bid", "Top bid", 92.).end().sortable(),
            ColumnSpec::new("updated", "Updated", 132.)
                .sortable()
                .droppable(4),
            ColumnSpec::new("actions", "", 104.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<WfmAuction> {
        self.data
            .rows
            .iter()
            .filter(|auction| query.matches(&title(auction)))
            .cloned()
            .collect()
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        Vec::new()
    }

    fn summary(&self) -> Vec<Stat> {
        let overview = &self.data.overview;
        vec![
            Stat {
                label: "Start prices",
                value: number(overview.revenue),
                note: "of the direct sales".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Estimated profit",
                value: number(overview.potential_profit),
                note: "over stock cost".to_owned(),
                tone: Tone::of_profit(overview.potential_profit),
            },
            Stat {
                label: "Auctions",
                value: number(self.data.rows.len() as i64),
                note: format!("of {} allowed", number(self.data.auctions_limit)),
                tone: Tone::Plain,
            },
        ]
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<AuctionAction>> {
        vec![
            ToolbarItem {
                id: "refresh",
                icon: IconName::RefreshCw,
                tooltip: "Refresh the auctions from Warframe Market".to_owned(),
                action: Some(AuctionAction::Refresh),
                danger: false,
            },
            ToolbarItem {
                id: "delete-all",
                icon: IconName::Trash,
                tooltip: "Delete every auction from Warframe Market".to_owned(),
                action: (!self.data.rows.is_empty()).then_some(AuctionAction::DeleteAll),
                danger: true,
            },
        ]
    }

    fn primary(&self) -> Option<PrimaryAction<AuctionAction>> {
        None
    }

    fn perform(&mut self, action: AuctionAction, source: &DataSource) -> Outcome<AuctionAction> {
        match action {
            AuctionAction::Refresh => Outcome::run(
                source.auction_refresh(),
                "Refreshed the auctions from Warframe Market.",
            ),
            AuctionAction::DeleteAll => Outcome::ask(
                FormSpec::confirm_danger(
                    match self.data.rows.len() {
                        1 => "Delete the auction?".to_owned(),
                        count => format!("Delete all {count} auctions?"),
                    },
                    "This removes the auctions from Warframe Market. Your stock rivens stay as they are.",
                    "Delete",
                ),
                |_| Some(AuctionAction::DeletedAll),
            ),
            AuctionAction::DeletedAll => Outcome::run(
                source.auction_delete_all(),
                "Deleted the auctions from Warframe Market.",
            ),
            AuctionAction::Import(key) => {
                match self.auction(key).filter(|auction| auction.can_import) {
                    Some(auction) => Outcome::ask(
                        FormSpec::new(format!("Import {}", title(&auction)), "Import")
                            .description(
                                "This adds the riven to your stock, so the live scraper prices it.",
                            )
                            .field(
                                Field::platinum("bought", "Bought", Some(0))
                                    .hint("What you paid for the riven."),
                            ),
                        move |values| {
                            Some(AuctionAction::Imported {
                                key,
                                bought: values.number("bought")?,
                            })
                        },
                    ),
                    None => Outcome::Done,
                }
            }
            AuctionAction::Imported { key, bought } => match self.auction(key) {
                Some(auction) => Outcome::run(
                    source.auction_import_by_id(auction.id.clone(), bought),
                    format!("Imported {} into stock.", title(&auction)),
                ),
                None => Outcome::Done,
            },
            AuctionAction::Delete(key) => {
                Outcome::ask(prompts::delete(1, "auction", "auctions"), move |_| {
                    Some(AuctionAction::Deleted(key))
                })
            }
            AuctionAction::Deleted(key) => match self.auction(key) {
                Some(auction) => Outcome::run_quietly(source.auction_delete_by_id(auction.id)),
                None => Outcome::Done,
            },
            AuctionAction::Details(key) => Outcome::ShowPanel(key),
            AuctionAction::CopyName(key) => match self.auction(key) {
                Some(auction) => Outcome::Copy {
                    what: title(&auction),
                    text: title(&auction),
                },
                None => Outcome::Done,
            },
            AuctionAction::OpenMarket(key) => match self.auction(key) {
                Some(auction) => {
                    Outcome::OpenUrl(format!("https://warframe.market/auction/{}", auction.id))
                }
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &WfmAuction) -> u64 {
        row_key(&row.id)
    }

    fn compare(key: &str, a: &WfmAuction, b: &WfmAuction) -> Ordering {
        match key {
            "mastery" => a.mastery_level.cmp(&b.mastery_level),
            "re_rolls" => a.re_rolls.cmp(&b.re_rolls),
            "starting_price" => a.starting_price.cmp(&b.starting_price),
            "buyout_price" => a.buyout_price.cmp(&b.buyout_price),
            "top_bid" => a.top_bid.cmp(&b.top_bid),
            "updated" => a.updated.cmp(&b.updated),
            _ => title(a).cmp(&title(b)),
        }
    }

    fn cell(row: &WfmAuction, key: &str, emit: &Emit<AuctionAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let id = row_key(&row.id);
        match key {
            "riven" => {
                let mut flags = Vec::new();
                if !row.visible {
                    flags.push((IconName::EyeOff, "Hidden on Warframe Market"));
                }
                cells::name(title(row), &flags, cx)
            }
            "attributes" => attributes_cell(id, &row.attributes, cx),
            "mastery" => cells::amount(row.mastery_level, "", cx),
            "re_rolls" => cells::amount(row.re_rolls, "", cx),
            "starting_price" => cells::amount(Some(row.starting_price), "", cx),
            "buyout_price" => cells::amount(row.buyout_price, "None", cx),
            "top_bid" => cells::amount(row.top_bid, "No bids", cx),
            "updated" => cells::text(day_and_time(&row.updated)),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .when(row.can_import, |actions| {
                        actions.child(
                            IconButton::new(
                                ("import", id),
                                IconName::PackagePlus,
                                "Import into stock",
                            )
                            .in_row()
                            .on_click(emit.on_click(AuctionAction::Import(id))),
                        )
                    })
                    .child(
                        IconButton::new(("delete", id), IconName::Trash, "Delete this auction")
                            .in_row()
                            .hover_tone(theme.colors.negative)
                            .on_click(emit.on_click(AuctionAction::Delete(id))),
                    )
                    .child(overflow_button(id, move || Self::row_menu(&menu_row), emit))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &WfmAuction, key: &str) -> Option<String> {
        match key {
            "riven" => Some(title(row)),
            "attributes" => Some(attributes_text(&row.attributes).join(", ")),
            "mastery" => row.mastery_level.map(number),
            "re_rolls" => row.re_rolls.map(number),
            "starting_price" => Some(number(row.starting_price)),
            "buyout_price" => row.buyout_price.map(number),
            "top_bid" => row.top_bid.map(number),
            "updated" => Some(day_and_time(&row.updated)),
            _ => None,
        }
    }

    fn row_menu(row: &WfmAuction) -> Vec<MenuEntry<AuctionAction>> {
        let id = row_key(&row.id);
        let mut menu = Vec::new();
        if row.can_import {
            menu.push(MenuEntry::new(
                "Import into stock",
                IconName::PackagePlus,
                AuctionAction::Import(id),
            ));
        }
        menu.extend([
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                AuctionAction::Details(id),
            ),
            MenuEntry::new("Copy name", IconName::Copy, AuctionAction::CopyName(id)),
            MenuEntry::new(
                "Open on Warframe Market",
                IconName::ExternalLink,
                AuctionAction::OpenMarket(id),
            ),
            MenuEntry::new("Delete", IconName::Trash, AuctionAction::Delete(id)).separated(),
        ]);
        menu
    }

    fn panel(row: &WfmAuction, cx: &App) -> PanelModel<AuctionAction> {
        let theme = Theme::global(cx);
        let id = row_key(&row.id);
        PanelModel {
            title: title(row),
            subtitle: match &row.polarity {
                Some(polarity) => format!("{} polarity", words(polarity)),
                None => words(&row.item_type),
            },
            status: Some(if row.closed {
                ("Closed".to_owned(), theme.colors.text_faint)
            } else if row.is_direct_sell {
                ("Direct sale".to_owned(), theme.colors.positive)
            } else {
                ("Bidding".to_owned(), theme.colors.info)
            }),
            note: (!row.note.is_empty()).then(|| row.note.clone()),
            history: Vec::new(),
            lines: (!row.attributes.is_empty())
                .then(|| ("Attributes", attribute_lines(&row.attributes))),
            facts: vec![
                Fact::new("Start price", number(row.starting_price)),
                Fact::amount("Buyout", row.buyout_price, "None"),
                Fact::amount("Top bid", row.top_bid, "No bids"),
                Fact::amount("Mastery rank", row.mastery_level, "Unknown"),
                Fact::amount("Rank", row.mod_rank, "Unknown"),
                Fact::amount("Re-rolls", row.re_rolls, "Unknown"),
                Fact::new(
                    "In stock",
                    if row.can_import {
                        "No, can be imported"
                    } else if row.is_direct_sell {
                        "Yes"
                    } else {
                        "Bidding auctions don't import"
                    },
                ),
                Fact::new("Created", day_and_time(&row.created)),
                Fact::new("Updated", day_and_time(&row.updated)),
            ],
            primary: vec![
                PanelAction::new(
                    "Import into stock",
                    IconName::PackagePlus,
                    AuctionAction::Import(id),
                )
                .enabled(row.can_import),
            ],
            secondary: vec![
                PanelAction::new("Copy name", IconName::Copy, AuctionAction::CopyName(id)),
                PanelAction::new(
                    "Open on Warframe Market",
                    IconName::ExternalLink,
                    AuctionAction::OpenMarket(id),
                ),
                PanelAction::new("Delete", IconName::Trash, AuctionAction::Delete(id)).danger(),
            ],
        }
    }
}
