use super::{FinancialReport, Transaction};

/// The payload of `dashboard_summary`.
#[derive(Clone, Debug, PartialEq)]
pub struct DashboardSummary {
    pub total: FinancialReport,
    pub present_year: PeriodSummary,
    pub last_year: PeriodSummary,
    pub today: PeriodSummary,
    pub recent_days: PeriodSummary,
    pub best_seller: BestSeller,
    pub categories: Vec<CategorySummary>,
    pub recent_transactions: Vec<Transaction>,
}

/// A report and its chart, as built by `generate_transaction_summary`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PeriodSummary {
    pub summary: FinancialReport,
    pub chart: Chart,
}

/// Mirrors `entity::dto::FinancialGraph<i64>`: profit per label.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chart {
    pub labels: Vec<String>,
    pub values: Vec<i64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BestSeller {
    pub wfm_id: String,
    pub item_name: String,
    pub report: FinancialReport,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CategorySummary {
    pub name: String,
    pub icon: String,
    pub report: FinancialReport,
}
