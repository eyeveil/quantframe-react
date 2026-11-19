use super::{OrderType, SubType};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DetectedItem {
    pub name: String,
    pub wfm_url: String,
    pub quantity: i64,
    pub sub_type: Option<SubType>,
    pub price: i64,
    pub on_wish_list: bool,
    pub needs_sub_type: bool,
    pub max_rank: Option<i64>,
    pub variants: Vec<String>,
    pub max_amber_stars: Option<i64>,
    pub max_cyan_stars: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandleItem {
    pub wfm_url: String,
    pub sub_type: Option<SubType>,
    pub quantity: i64,
    pub price: i64,
    pub user_name: String,
    pub order_type: OrderType,
    pub operations: Vec<String>,
}
