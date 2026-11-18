#[derive(Clone, Debug, PartialEq)]
pub struct InventoryRiven {
    pub uuid: String,
    pub name: String,
    pub wfm_url: String,
    pub mod_name: String,
    pub mastery_rank: i64,
    pub re_rolls: i64,
    pub rank: i64,
    pub polarity: String,
    pub riven_type: String,
    pub is_in_stock: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InventorySyndicate {
    pub name: String,
    pub unique_name: String,
    pub standing: i64,
    pub total_standing: i64,
}
