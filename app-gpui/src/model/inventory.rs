//! The in-game inventory read through AlecaFrame or the public profile.

/// One riven from `wf_inventory_get_rivens`.
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

/// One syndicate standing from `wf_inventory_get_syndicates`.
#[derive(Clone, Debug, PartialEq)]
pub struct InventorySyndicate {
    pub name: String,
    pub unique_name: String,
    /// Standing above the current title's minimum.
    pub standing: i64,
    pub total_standing: i64,
}
