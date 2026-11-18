//! The Auctions tab: riven auctions on Warframe Market.

use std::cmp::Ordering;

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div, px};
use gpui_component::button::ButtonVariants;
use gpui_kit_assets::IconName;

use super::{
    REFRESH_NOT_CONNECTED,
    model::{Auction, AuctionStore, row_key},
};
use crate::{
    listings::attributes_cell,
    theme::Theme,
    ui::{
        IconButton, cells, day_and_time,
        form::{Field, FormSpec},
        grid::{
            Catalog, ChipGroup, ColumnSpec, Emit, MenuEntry, Outcome, PrimaryAction, Query, Stat,
            Tone, ToolbarItem, overflow_button,
        },
        number,
        panel::{Fact, PanelAction, PanelModel},
        prompts, row_button,
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

pub struct Auctions {
    store: AuctionStore,
    auctions_limit: i64,
}

impl Auctions {
    pub fn new(rows: Vec<Auction>, auctions_limit: i64) -> Self {
        Self {
            store: AuctionStore::new(rows),
            auctions_limit,
        }
    }

    fn auction(&self, key: u64) -> Option<Auction> {
        self.store.get(key).cloned()
    }
}

impl Catalog for Auctions {
    type Row = Auction;
    type Action = AuctionAction;

    const NOUN: &'static str = "auctions";
    const SEARCH_PLACEHOLDER: &'static str = "Search auctions";
    const EMPTY_HINT: &'static str =
        "Auctions appear here once the live scraper lists your rivens.";
    const CHECKABLE: bool = false;

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("riven", "Riven", 204.).sortable(),
            ColumnSpec::new("attributes", "Attributes", 200.).flex(),
            ColumnSpec::new("mastery", "MR", 56.).end().sortable().droppable(3),
            ColumnSpec::new("re_rolls", "Rolls", 62.).end().sortable().droppable(2),
            ColumnSpec::new("starting_price", "Start price", 100.).end().sortable(),
            ColumnSpec::new("buyout_price", "Buyout", 88.).end().sortable().droppable(1),
            ColumnSpec::new("top_bid", "Top bid", 88.).end().sortable(),
            ColumnSpec::new("updated", "Updated", 124.).sortable().droppable(4),
            ColumnSpec::new("actions", "", 156.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<Auction> {
        self.store.list(&query.text)
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        Vec::new()
    }

    fn summary(&self) -> Vec<Stat> {
        let (count, starting, profit) = self.store.overview();
        let importable = self
            .store
            .list("")
            .iter()
            .filter(|auction| auction.can_import)
            .count();
        vec![
            Stat {
                label: "Start prices",
                value: number(starting),
                note: "in total".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Estimated profit",
                value: number(profit),
                note: "over stock cost".to_owned(),
                tone: Tone::of_profit(profit),
            },
            Stat {
                label: "Not in stock",
                value: number(importable as i64),
                note: "can be imported".to_owned(),
                tone: Tone::Plain,
            },
            Stat {
                label: "Auctions",
                value: number(count as i64),
                note: format!("of {} allowed", number(self.auctions_limit)),
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
                action: Some(AuctionAction::DeleteAll),
                danger: true,
            },
        ]
    }

    fn primary(&self) -> Option<PrimaryAction<AuctionAction>> {
        None
    }

    fn perform(&mut self, action: AuctionAction) -> Outcome<AuctionAction> {
        match action {
            AuctionAction::Refresh => Outcome::Notice(REFRESH_NOT_CONNECTED.to_owned()),
            AuctionAction::DeleteAll => {
                let (count, ..) = self.store.overview();
                if count == 0 {
                    return Outcome::Done;
                }
                Outcome::ask(
                    FormSpec::confirm_danger(
                        format!("Delete all {count} auctions?"),
                        "This removes the auctions from Warframe Market. Your stock rivens stay as they are.",
                        "Delete",
                    ),
                    |_| Some(AuctionAction::DeletedAll),
                )
            }
            AuctionAction::DeletedAll => {
                let count = self.store.delete_all();
                Outcome::Notice(format!("Deleted {count} auctions."))
            }
            AuctionAction::Import(key) => match self.auction(key) {
                Some(auction) => Outcome::ask(
                    FormSpec::new(format!("Import {}", auction.full_name()), "Import")
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
            },
            AuctionAction::Imported { key, bought } => match self.auction(key) {
                Some(auction) => {
                    self.store.import(key, bought);
                    Outcome::Notice(format!("Imported {} into stock.", auction.full_name()))
                }
                None => Outcome::Done,
            },
            AuctionAction::Delete(key) => Outcome::ask(
                prompts::delete(1, "auction", "auctions"),
                move |_| Some(AuctionAction::Deleted(key)),
            ),
            AuctionAction::Deleted(key) => {
                self.store.delete(key);
                Outcome::Done
            }
            AuctionAction::Details(key) => Outcome::ShowPanel(key),
            AuctionAction::CopyName(key) => match self.auction(key) {
                Some(auction) => Outcome::Copy {
                    what: auction.full_name(),
                    text: auction.full_name(),
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

    fn row_id(row: &Auction) -> u64 {
        row_key(&row.id)
    }

    fn compare(key: &str, a: &Auction, b: &Auction) -> Ordering {
        match key {
            "mastery" => a.mastery_level.cmp(&b.mastery_level),
            "re_rolls" => a.re_rolls.cmp(&b.re_rolls),
            "starting_price" => a.starting_price.cmp(&b.starting_price),
            "buyout_price" => a.buyout_price.cmp(&b.buyout_price),
            "top_bid" => a.top_bid.cmp(&b.top_bid),
            "updated" => a.updated.cmp(&b.updated),
            _ => a.full_name().cmp(&b.full_name()),
        }
    }

    fn cell(row: &Auction, key: &str, emit: &Emit<AuctionAction>, cx: &App) -> AnyElement {
        let theme = Theme::global(cx);
        let id = row_key(&row.id);
        match key {
            "riven" => {
                let mut flags = Vec::new();
                if !row.visible {
                    flags.push((IconName::EyeOff, "Hidden on Warframe Market"));
                }
                if row.can_import {
                    flags.push((IconName::PackagePlus, "Not in your stock yet"));
                }
                cells::name(row.full_name(), &flags, cx)
            }
            "attributes" => attributes_cell(id, &row.attributes, cx),
            "mastery" => cells::amount(Some(row.mastery_level), "", cx),
            "re_rolls" => cells::amount(Some(row.re_rolls), "", cx),
            "starting_price" => cells::amount(Some(row.starting_price), "", cx),
            "buyout_price" => cells::amount(row.buyout_price, "None", cx),
            "top_bid" => cells::amount(row.top_bid, "No bids", cx),
            "updated" => cells::text(day_and_time(&row.updated)),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .children(row.can_import.then(|| {
                        div().pr(px(5.)).child(
                            row_button(("import", id), "Import", cx)
                                .primary()
                                .on_click(emit.on_click(AuctionAction::Import(id))),
                        )
                    }))
                    .child(
                        IconButton::new(("delete", id), IconName::Trash, "Delete this auction")
                            .in_row()
                            .hover_tone(theme.colors.negative)
                            .on_click(emit.on_click(AuctionAction::Delete(id))),
                    )
                    .child(overflow_button(
                        id,
                        move || Self::row_menu(&menu_row),
                        emit,
                    ))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &Auction, key: &str) -> Option<String> {
        match key {
            "riven" => Some(row.full_name()),
            "attributes" => Some(
                row.attributes
                    .iter()
                    .map(|attribute| attribute.label())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            "mastery" => Some(number(row.mastery_level)),
            "re_rolls" => Some(number(row.re_rolls)),
            "starting_price" => Some(number(row.starting_price)),
            "buyout_price" => row.buyout_price.map(number),
            "top_bid" => row.top_bid.map(number),
            "updated" => Some(day_and_time(&row.updated)),
            _ => None,
        }
    }

    fn row_menu(row: &Auction) -> Vec<MenuEntry<AuctionAction>> {
        let id = row_key(&row.id);
        let mut menu = Vec::new();
        if row.can_import {
            menu.push(MenuEntry::new(
                "Import",
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

    fn panel(row: &Auction, cx: &App) -> PanelModel<AuctionAction> {
        let theme = Theme::global(cx);
        let id = row_key(&row.id);
        let profit = row.profit();
        PanelModel {
            title: row.full_name(),
            subtitle: format!("{} polarity", row.polarity),
            status: Some(if row.is_direct_sell {
                ("Direct sale".to_owned(), theme.colors.positive)
            } else {
                ("Bidding".to_owned(), theme.colors.info)
            }),
            note: Some(
                if row.can_import {
                    "No stock riven matches this auction"
                } else {
                    "Listed from your stock"
                }
                .to_owned(),
            ),
            history: Vec::new(),
            lines: Some((
                "Attributes",
                row.attributes
                    .iter()
                    .map(|attribute| {
                        (
                            attribute.label(),
                            if attribute.positive {
                                Tone::Plain
                            } else {
                                Tone::Negative
                            },
                        )
                    })
                    .collect(),
            )),
            facts: vec![
                Fact::new("Start price", number(row.starting_price)),
                Fact::amount("Buyout", row.buyout_price, "None"),
                Fact::amount("Top bid", row.top_bid, "No bids"),
                Fact::amount("Stock cost", row.stock_cost, "Not in stock"),
                match profit {
                    Some(profit) => {
                        Fact::new("Estimated profit", number(profit)).tone(Tone::of_profit(profit))
                    }
                    None => Fact::new("Estimated profit", "None to show").tone(Tone::Muted),
                },
                Fact::new("Mastery rank", number(row.mastery_level)),
                Fact::new("Rank", number(row.mod_rank)),
                Fact::new("Re-rolls", number(row.re_rolls)),
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
