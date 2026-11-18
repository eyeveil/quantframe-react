use super::StockStatus;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListParams {
    pub page: i64,
    pub limit: i64,
    pub sort_by: Option<String>,
    pub sort_direction: SortDirection,
    pub query: Option<String>,
}

impl Default for ListParams {
    fn default() -> Self {
        Self {
            page: 1,
            limit: 100,
            sort_by: None,
            sort_direction: SortDirection::default(),
            query: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyndicateItemListParams {
    pub stock: StockItemListParams,
    pub syndicate: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrderListParams {
    pub list: ListParams,
    pub order_type: Option<super::OrderType>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TradeEntryListParams {
    pub list: ListParams,
    pub group: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransactionListParams {
    pub list: ListParams,
    pub transaction_type: Option<super::TransactionType>,
    pub item_type: Option<super::TransactionItemType>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}
