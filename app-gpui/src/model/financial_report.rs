/// Mirrors `entity::dto::FinancialReport`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FinancialReport {
    pub total_transactions: i64,
    pub average_transaction: f64,
    pub total_profit: i64,
    pub average_profit: f64,
    pub profit_margin: f64,
    pub roi: f64,
    pub sale_count: i64,
    pub highest_revenue: i64,
    pub lowest_revenue: i64,
    pub average_revenue: f64,
    pub revenue: i64,
    pub purchases_count: i64,
    pub highest_expense: i64,
    pub lowest_expense: i64,
    pub average_expense: f64,
    pub expenses: i64,
}
