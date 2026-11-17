use super::StockStatus;

/// Mirrors `entity::dto::PaginatedResult`.
#[derive(Clone, Debug, PartialEq)]
pub struct Paginated<T> {
    pub total: i64,
    pub limit: i64,
    pub page: i64,
    pub total_pages: i64,
    pub results: Vec<T>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

/// Mirrors `StockItemControllerGetListParams`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StockItemListParams {
    pub page: i64,
    pub limit: i64,
    pub sort_by: Option<String>,
    pub sort_direction: SortDirection,
    pub query: Option<String>,
    pub status: Option<StockStatus>,
}

impl Default for StockItemListParams {
    fn default() -> Self {
        Self {
            page: 1,
            limit: 100,
            sort_by: None,
            sort_direction: SortDirection::default(),
            query: None,
            status: None,
        }
    }
}
