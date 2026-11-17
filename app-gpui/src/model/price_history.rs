/// Mirrors `entity::dto::PriceHistory`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriceHistory {
    pub created_at: String,
    pub price: i64,
}
