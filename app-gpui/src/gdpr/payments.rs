use std::{cmp::Ordering, collections::BTreeMap, sync::Arc};

use gpui::{AnyElement, App, IntoElement, ParentElement, Styled};
use gpui_kit_assets::IconName;

use super::{
    ready,
    report::{Ink, Monthly, Report, Table},
    year, year_chips,
};
use crate::{
    data::{DataKind, DataSource},
    model::{GdprAccount, GdprPayment},
    ui::{
        cells, counted, date_time_with_year,
        grid::{
            Catalog, ChipGroup, ChipSpec, ColumnSpec, Emit, Load, MenuEntry, Outcome,
            PrimaryAction, Query, Stat, Tone, ToolbarItem, overflow_button,
        },
        number, numeric,
        panel::{Fact, PanelAction, PanelModel},
    },
};

#[derive(Clone, Debug)]
pub enum PaymentAction {
    Details(u64),
    CopySku(u64),
}

#[derive(Clone, Debug)]
pub struct PaymentRow {
    id: u64,
    payment: GdprPayment,
}

#[derive(Default)]
pub struct Payments {
    pub account: Option<Arc<GdprAccount>>,
}

pub fn money(value: f64) -> String {
    let cents = (value * 100.).round() as i64;
    let sign = if cents < 0 { "-" } else { "" };
    format!(
        "{sign}{}.{:02}",
        number(cents.abs() / 100),
        cents.abs() % 100
    )
}

fn priced(payment: &GdprPayment) -> String {
    format!("{} {}", money(payment.price), payment.currency)
}

pub struct CurrencyTotal {
    pub currency: String,
    pub count: i64,
    pub total: f64,
    pub largest: f64,
}

pub fn by_currency(payments: &[GdprPayment]) -> Vec<CurrencyTotal> {
    let mut totals: BTreeMap<&str, CurrencyTotal> = BTreeMap::new();
    for payment in payments {
        let entry = totals
            .entry(payment.currency.as_str())
            .or_insert_with(|| CurrencyTotal {
                currency: payment.currency.clone(),
                count: 0,
                total: 0.,
                largest: 0.,
            });
        entry.count += 1;
        entry.total += payment.price;
        entry.largest = entry.largest.max(payment.price);
    }
    let mut totals: Vec<CurrencyTotal> = totals.into_values().collect();
    totals.sort_by(|a, b| b.count.cmp(&a.count).then(a.currency.cmp(&b.currency)));
    totals
}

fn totals(payments: &[GdprPayment]) -> Vec<Stat> {
    let currencies = by_currency(payments);
    let count = payments.len() as i64;
    let Some(main) = currencies.first() else {
        return vec![Stat {
            label: "Payments",
            value: number(0),
            note: String::new(),
            tone: Tone::Plain,
        }];
    };
    let others = currencies.len() as i64 - 1;
    vec![
        Stat {
            label: "Paid",
            value: format!("{} {}", money(main.total), main.currency),
            note: if others == 0 {
                "in real money".to_owned()
            } else {
                format!("and {}", counted(others, "other currency", "other currencies"))
            },
            tone: Tone::Plain,
        },
        Stat {
            label: "Payments",
            value: number(count),
            note: counted(
                payments
                    .iter()
                    .map(|payment| payment.vendor.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len() as i64,
                "vendor",
                "vendors",
            ),
            tone: Tone::Plain,
        },
        Stat {
            label: "Average payment",
            value: format!(
                "{} {}",
                money(main.total / main.count.max(1) as f64),
                main.currency
            ),
            note: counted(main.count, "payment", "payments"),
            tone: Tone::Plain,
        },
        Stat {
            label: "Largest payment",
            value: format!("{} {}", money(main.largest), main.currency),
            note: payments
                .iter()
                .filter(|payment| payment.currency == main.currency)
                .max_by(|a, b| a.price.total_cmp(&b.price))
                .map_or_else(String::new, |payment| payment.sku.clone()),
            tone: Tone::Plain,
        },
    ]
}

pub fn report(account: &GdprAccount) -> Report {
    let payments = &account.payments;
    let currencies = by_currency(payments);
    let main = currencies
        .first()
        .map(|total| total.currency.clone())
        .unwrap_or_default();
    let mut by_sku: BTreeMap<(&str, &str), (i64, f64)> = BTreeMap::new();
    for payment in payments {
        let entry = by_sku
            .entry((payment.sku.as_str(), payment.currency.as_str()))
            .or_default();
        entry.0 += 1;
        entry.1 += payment.price;
    }
    let mut skus: Vec<_> = by_sku.into_iter().collect();
    skus.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    let mut monthly = Monthly::new("Paid per month", vec![("Paid", Ink::Accent)]);
    let mut left_out = 0;
    for payment in payments {
        if payment.currency == main {
            monthly.add(&payment.time, 0, payment.price.round() as i64);
        } else {
            left_out += 1;
        }
    }
    let mut note = format!("The chart rounds each payment to a whole {main}.");
    if left_out > 0 {
        note.push_str(&format!(
            " It leaves out {} in other currencies.",
            counted(left_out, "payment", "payments")
        ));
    }
    Report {
        stats: totals(payments),
        tables: vec![Table {
            title: "Payments by product",
            name: "SKU",
            columns: vec!["Payments", "Paid"],
            empty: "No payments.",
            rows: skus
                .into_iter()
                .take(12)
                .map(|((sku, currency), (count, total))| {
                    (
                        sku.to_owned(),
                        vec![number(count), format!("{} {currency}", money(total))],
                    )
                })
                .collect(),
        }],
        monthly,
        note: (!payments.is_empty()).then_some(note),
    }
}

impl Payments {
    fn payments(&self) -> &[GdprPayment] {
        self.account
            .as_ref()
            .map_or(&[], |account| account.payments.as_slice())
    }

    fn row(&self, id: u64) -> Option<&GdprPayment> {
        self.payments().get(id as usize)
    }
}

impl Catalog for Payments {
    type Row = PaymentRow;
    type Action = PaymentAction;
    type Data = ();

    const NOUN: &'static str = "payments";
    const SEARCH_PLACEHOLDER: &'static str = "Search payments";
    const EMPTY_HINT: &'static str = "This account's export lists no real-money payments.";
    const CHECKABLE: bool = false;
    const WATCH: &'static [DataKind] = &[];

    fn load(&self, _: &DataSource) -> Load<()> {
        ready()
    }

    fn apply(&mut self, (): ()) {}

    fn columns(&self) -> Vec<ColumnSpec> {
        vec![
            ColumnSpec::new("date", "Date", 168.).sortable(),
            ColumnSpec::new("sku", "Product (SKU)", 220.).flex().sortable(),
            ColumnSpec::new("vendor", "Vendor", 170.).sortable(),
            ColumnSpec::new("account", "Account", 170.)
                .sortable()
                .droppable(1),
            ColumnSpec::new("price", "Paid", 130.).end().sortable(),
            ColumnSpec::new("actions", "", 52.),
        ]
    }

    fn rows(&self, query: &Query) -> Vec<PaymentRow> {
        let in_year = query.filter("year");
        let vendor = query.filter("vendor");
        self.payments()
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, payment)| in_year.is_none_or(|key| year(&payment.time) == key))
            .filter(|(_, payment)| vendor.is_none_or(|key| payment.vendor == key))
            .filter(|(_, payment)| {
                query.matches(&format!(
                    "{} {} {}",
                    payment.sku, payment.vendor, payment.account
                ))
            })
            .map(|(index, payment)| PaymentRow {
                id: index as u64,
                payment: payment.clone(),
            })
            .collect()
    }

    fn chips(&self, _: &App) -> Vec<ChipGroup> {
        let payments = self.payments();
        let mut vendors: BTreeMap<&str, usize> = BTreeMap::new();
        for payment in payments {
            *vendors.entry(payment.vendor.as_str()).or_default() += 1;
        }
        vec![
            year_chips(payments.iter().map(|payment| payment.time.as_str())),
            ChipGroup {
                id: "vendor",
                chips: vendors
                    .into_iter()
                    .filter(|(vendor, _)| !vendor.is_empty())
                    .map(|(vendor, count)| ChipSpec {
                        key: vendor.to_owned(),
                        label: vendor.to_owned(),
                        count,
                        color: None,
                        tooltip: String::new(),
                    })
                    .collect(),
            },
        ]
    }

    fn summary(&self) -> Vec<Stat> {
        totals(self.payments())
    }

    fn toolbar(&self, _: &[u64]) -> Vec<ToolbarItem<PaymentAction>> {
        Vec::new()
    }

    fn primary(&self) -> Option<PrimaryAction<PaymentAction>> {
        None
    }

    fn perform(&mut self, action: PaymentAction, _: &DataSource) -> Outcome<PaymentAction> {
        match action {
            PaymentAction::Details(id) => Outcome::ShowPanel(id),
            PaymentAction::CopySku(id) => match self.row(id) {
                Some(payment) => Outcome::Copy {
                    text: payment.sku.clone(),
                    what: "the SKU".to_owned(),
                },
                None => Outcome::Done,
            },
        }
    }

    fn row_id(row: &PaymentRow) -> u64 {
        row.id
    }

    fn compare(key: &str, a: &PaymentRow, b: &PaymentRow) -> Ordering {
        match key {
            "sku" => a.payment.sku.cmp(&b.payment.sku),
            "vendor" => a.payment.vendor.cmp(&b.payment.vendor),
            "account" => a.payment.account.cmp(&b.payment.account),
            "price" => a.payment.price.total_cmp(&b.payment.price),
            _ => a.payment.time.cmp(&b.payment.time),
        }
    }

    fn cell(row: &PaymentRow, key: &str, emit: &Emit<PaymentAction>, cx: &App) -> AnyElement {
        let payment = &row.payment;
        match key {
            "date" => cells::text(date_time_with_year(&payment.time)),
            "sku" => cells::name(payment.sku.clone(), &[], cx),
            "vendor" => cells::text(payment.vendor.clone()),
            "account" => cells::text(payment.account.clone()),
            "price" => numeric(priced(payment))
                .text_color(crate::theme::Theme::global(cx).colors.text)
                .into_any_element(),
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

    fn cell_text(row: &PaymentRow, key: &str) -> Option<String> {
        let payment = &row.payment;
        match key {
            "date" => Some(date_time_with_year(&payment.time)),
            "sku" => Some(payment.sku.clone()),
            "vendor" => Some(payment.vendor.clone()),
            "account" => Some(payment.account.clone()),
            "price" => Some(priced(payment)),
            _ => None,
        }
    }

    fn row_menu(row: &PaymentRow) -> Vec<MenuEntry<PaymentAction>> {
        vec![
            MenuEntry::new(
                "Show details",
                IconName::PanelRight,
                PaymentAction::Details(row.id),
            ),
            MenuEntry::new("Copy SKU", IconName::Copy, PaymentAction::CopySku(row.id)),
        ]
    }

    fn panel(row: &PaymentRow, _: &App) -> PanelModel<PaymentAction> {
        let payment = &row.payment;
        PanelModel {
            title: payment.sku.clone(),
            subtitle: date_time_with_year(&payment.time),
            status: None,
            note: None,
            history: Vec::new(),
            lines: None,
            facts: vec![
                Fact::new("Paid", priced(payment)),
                Fact::new("Vendor", payment.vendor.clone()),
                Fact::new("Account", payment.account.clone()),
                Fact::new("Date", date_time_with_year(&payment.time)),
            ],
            primary: Vec::new(),
            secondary: vec![PanelAction::new(
                "Copy SKU",
                IconName::Copy,
                PaymentAction::CopySku(row.id),
            )],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payment(price: f64, currency: &str) -> GdprPayment {
        GdprPayment {
            time: "2025-06-14T09:41:20+00:00".to_owned(),
            sku: "Platinum1000".to_owned(),
            price,
            currency: currency.to_owned(),
            vendor: "Steam".to_owned(),
            account: "FixtureTrader".to_owned(),
        }
    }

    #[test]
    fn money_keeps_two_decimals_and_groups_thousands() {
        assert_eq!(money(49.99), "49.99");
        assert_eq!(money(1234.5), "1,234.50");
        assert_eq!(money(0.), "0.00");
        assert_eq!(money(179.95000000000002), "179.95");
    }

    #[test]
    fn the_currency_with_the_most_payments_comes_first() {
        let payments = [
            payment(9.99, "USD"),
            payment(19.99, "EUR"),
            payment(49.99, "EUR"),
        ];
        let totals = by_currency(&payments);
        assert_eq!(totals[0].currency, "EUR");
        assert_eq!((totals[0].count, money(totals[0].total)), (2, "69.98".to_owned()));
        assert_eq!(money(totals[0].largest), "49.99");
        assert_eq!(totals[1].currency, "USD");
    }
}
