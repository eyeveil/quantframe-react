use std::{cmp::Ordering, sync::Arc};

use gpui::{AnyElement, App, IntoElement, ParentElement};
use gpui_kit_assets::IconName;

use super::{
    item_text, quantities, ready,
    report::{Ink, Monthly, Report, Table},
    year, year_chips,
};
use crate::{
    data::{DataKind, DataSource},
    model::{GdprAccount, GdprPurchase},
    ui::{
        cells, counted, date_time_with_year,
        grid::{
            Catalog, ChipGroup, ColumnSpec, Emit, Load, MenuEntry, Outcome, PrimaryAction, Query,
            Stat, Tone, ToolbarItem, overflow_button,
        },
        number,
        panel::{Fact, PanelAction, PanelModel},
    },
};

#[derive(Clone, Debug)]
pub enum PurchaseAction {
    Details(u64),
    CopyItems(u64),
    CopyShopId(u64),
}

#[derive(Clone, Debug)]
pub struct PurchaseRow {
    id: u64,
    purchase: GdprPurchase,
}

#[derive(Default)]
pub struct Purchases {
    pub account: Option<Arc<GdprAccount>>,
}

fn shop_item(shop_id: &str) -> &str {
    shop_id.rsplit('/').next().unwrap_or(shop_id)
}

fn totals(purchases: &[GdprPurchase]) -> Vec<Stat> {
    let count = purchases.len() as i64;
    let spent: i64 = purchases.iter().map(|purchase| purchase.platinum).sum();
    let largest = purchases
        .iter()
        .max_by_key(|purchase| purchase.platinum);
    let items: i64 = purchases
        .iter()
        .flat_map(|purchase| &purchase.items)
        .map(|item| item.quantity)
        .sum();
    vec![
        Stat {
            label: "Platinum spent",
            value: number(spent),
            note: "in Warframe's market".to_owned(),
            tone: Tone::Plain,
        },
        Stat {
            label: "Purchases",
            value: number(count),
            note: counted(items, "item", "items"),
            tone: Tone::Plain,
        },
        Stat {
            label: "Average purchase",
            value: number(if count == 0 { 0 } else { spent / count }),
            note: "platinum".to_owned(),
            tone: Tone::Plain,
        },
        Stat {
            label: "Largest purchase",
            value: number(largest.map_or(0, |purchase| purchase.platinum)),
            note: largest.map_or_else(String::new, |purchase| item_text(&purchase.items)),
            tone: Tone::Plain,
        },
    ]
}

pub fn report(account: &GdprAccount) -> Report {
    let purchases = &account.purchases;
    let bought = quantities(purchases.iter().flat_map(|purchase| {
        purchase
            .items
            .iter()
            .map(|item| (item.name.as_str(), item.quantity, purchase.platinum))
    }));
    let mut monthly = Monthly::new("Platinum spent per month", vec![("Spent", Ink::Accent)]);
    for purchase in purchases {
        monthly.add(&purchase.time, 0, purchase.platinum);
    }
    Report {
        stats: totals(purchases),
        tables: vec![Table {
            title: "Most bought items",
            name: "Item",
            columns: vec!["Quantity", "Platinum"],
            empty: "No purchases.",
            rows: bought
                .into_iter()
                .take(12)
                .map(|(name, quantity, platinum)| (name, vec![number(quantity), number(platinum)]))
                .collect(),
        }],
        monthly,
        note: Some(
            "The platinum of a purchase counts for every item that purchase delivered.".to_owned(),
        ),
    }
}

impl Purchases {
    fn purchases(&self) -> &[GdprPurchase] {
        self.account
            .as_ref()
            .map_or(&[], |account| account.purchases.as_slice())
    }

    fn row(&self, id: u64) -> Option<&GdprPurchase> {
        self.purchases().get(id as usize)
    }
}

impl Catalog for Purchases {
    type Row = PurchaseRow;
    type Action = PurchaseAction;
    type Data = ();

    const NOUN: &'static str = "market purchases";
    const SEARCH_PLACEHOLDER: &'static str = "Search purchases";
    const EMPTY_HINT: &'static str = "This account's export lists no market purchases.";
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[];

    fn load(&self, _: &DataSource) -> Load<()> {
        ready()
    }

    fn apply(&mut self, (): ()) {}

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("date", "Date", 168.).sortable(),
            ColumnSpec::new("items", "Items", 240.).flex().sortable(),
            ColumnSpec::new("shop", "Store item", 260.)
                .sortable()
                .droppable(1),
            ColumnSpec::new("platinum", "Platinum", 104.).end().sortable(),
            ColumnSpec::new("actions", "", 52.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<PurchaseRow> {
        let in_year = query.filter("year");
        self.purchases()
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, purchase)| in_year.is_none_or(|key| year(&purchase.time) == key))
            .filter(|(_, purchase)| {
                query.matches(&format!(
                    "{} {}",
                    item_text(&purchase.items),
                    purchase.shop_id
                ))
            })
            .map(|(index, purchase)| PurchaseRow {
                id: index as u64,
                purchase: purchase.clone(),
            })
            .collect()
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        vec![year_chips(
            self.purchases().iter().map(|purchase| purchase.time.as_str()),
        )]
    }

    fn summary(&self) -> Vec<Stat> {
        totals(self.purchases())
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<PurchaseAction>> {
        Vec::new()
    }

    fn primary(&self) -> Option<PrimaryAction<PurchaseAction>> {
        None
    }

    fn perform(&mut self, action: PurchaseAction, _: &DataSource) -> Outcome<PurchaseAction> {
        match action {
            PurchaseAction::Details(id) => Outcome::ShowPanel(id),
            PurchaseAction::CopyItems(id) => match self.row(id) {
                Some(purchase) => Outcome::Copy {
                    text: item_text(&purchase.items),
                    what: "the item names".to_owned(),
                },
                None => Outcome::Done,
            },
            PurchaseAction::CopyShopId(id) => match self.row(id) {
                Some(purchase) => Outcome::Copy {
                    text: purchase.shop_id.clone(),
                    what: "the store item path".to_owned(),
                },
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &PurchaseRow) -> u64 {
        row.id
    }

    fn compare(key: &str, a: &PurchaseRow, b: &PurchaseRow) -> Ordering {
        match key {
            "items" => item_text(&a.purchase.items).cmp(&item_text(&b.purchase.items)),
            "shop" => shop_item(&a.purchase.shop_id).cmp(shop_item(&b.purchase.shop_id)),
            "platinum" => a.purchase.platinum.cmp(&b.purchase.platinum),
            _ => a.purchase.time.cmp(&b.purchase.time),
        }
    }

    fn cell(row: &PurchaseRow, key: &str, emit: &Emit<PurchaseAction>, cx: &App) -> AnyElement {
        let purchase = &row.purchase;
        match key {
            "date" => cells::text(date_time_with_year(&purchase.time)),
            "items" => cells::name(item_text(&purchase.items), &[], cx),
            "shop" => cells::text(shop_item(&purchase.shop_id).to_owned()),
            "platinum" => cells::amount(Some(purchase.platinum), "", cx),
            _ => {
                let menu_row = row.clone();
                cells::actions()
                    .child(overflow_button(
                        row.id,
                        move || Self::row_menu(&menu_row),
                        emit,
                    ))
                    .into_any_element()
            }
        }
    }

    fn cell_text(row: &PurchaseRow, key: &str) -> Option<String> {
        let purchase = &row.purchase;
        match key {
            "date" => Some(date_time_with_year(&purchase.time)),
            "items" => Some(item_text(&purchase.items)),
            "shop" => Some(shop_item(&purchase.shop_id).to_owned()),
            "platinum" => Some(number(purchase.platinum)),
            _ => None,
        }
    }

    fn row_menu(row: &PurchaseRow) -> Vec<MenuEntry<PurchaseAction>> {
        vec![
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                PurchaseAction::Details(row.id),
            ),
            MenuEntry::new(
                "Copy item names",
                IconName::Copy,
                PurchaseAction::CopyItems(row.id),
            ),
            MenuEntry::new(
                "Copy store item path",
                IconName::Copy,
                PurchaseAction::CopyShopId(row.id),
            ),
        ]
    }

    fn panel(row: &PurchaseRow, _: &App) -> PanelModel<PurchaseAction> {
        let purchase = &row.purchase;
        PanelModel {
            title: item_text(&purchase.items),
            subtitle: date_time_with_year(&purchase.time),
            status: None,
            note: Some(purchase.shop_id.clone()),
            history: Vec::new(),
            lines: Some((
                "Delivered",
                purchase
                    .items
                    .iter()
                    .map(|item| (format!("{} {}", number(item.quantity), item.name), Tone::Plain))
                    .collect(),
            )),
            facts: vec![
                Fact::new("Platinum", number(purchase.platinum)),
                Fact::new(
                    "Items",
                    number(purchase.items.iter().map(|item| item.quantity).sum()),
                ),
                Fact::new("Store item", shop_item(&purchase.shop_id)),
            ],
            primary: Vec::new(),
            secondary: vec![
                PanelAction::new(
                    "Copy item names",
                    IconName::Copy,
                    PurchaseAction::CopyItems(row.id),
                ),
                PanelAction::new(
                    "Copy store item path",
                    IconName::Copy,
                    PurchaseAction::CopyShopId(row.id),
                ),
            ],
        }
    }
}
