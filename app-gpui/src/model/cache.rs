#[derive(Clone, Debug, PartialEq)]
pub struct TradableItem {
    pub wfm_id: String,
    pub wfm_url: String,
    pub name: String,
    pub unique_name: String,
    pub tags: Vec<String>,
    pub bulk_tradable: bool,
    pub max_rank: Option<i64>,
    pub variants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RivenWeapon {
    pub wfm_url: String,
    pub wfm_id: String,
    pub name: String,
    pub unique_name: String,
    pub riven_type: String,
    pub disposition: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RivenStat {
    pub wfm_url: String,
    pub label: String,
    pub unit: Option<String>,
    pub positive_only: bool,
    pub negative_only: bool,
    pub exclusive_to: Vec<String>,
}

impl RivenStat {
    pub fn fits(&self, riven_type: &str) -> bool {
        self.exclusive_to.is_empty() || self.exclusive_to.iter().any(|kind| kind == riven_type)
    }
}
