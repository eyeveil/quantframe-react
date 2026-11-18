use super::{FinancialReport, Transaction};

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PeriodSummary {
    pub summary: FinancialReport,
    pub chart: Chart,
}

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
