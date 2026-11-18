use sea_orm::Set;
use serde::{Deserialize, Serialize};

use crate::{enums::*, trade_entry::*};
use utils::SubType;
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateTradeEntry {
    pub id: i64,
    #[serde(default)]
    pub price: FieldChange<i64>,
    #[serde(default)]
    pub tags: FieldChange<Vec<String>>,
    #[serde(default)]
    pub sub_type: FieldChange<Option<SubType>>,
    #[serde(default)]
    pub properties: FieldChange<serde_json::Value>,
}

impl UpdateTradeEntry {
    pub fn apply_to(self, mut item: trade_entry::ActiveModel) -> trade_entry::ActiveModel {
        use FieldChange::*;

        match self.price {
            Value(v) => item.price = Set(v),
            _ => {}
        }
        match self.tags {
            Value(v) => item.tags = Set(v.join(",")),
            _ => {}
        }
        match self.sub_type {
            Value(v) => item.sub_type = Set(v),
            _ => {}
        }
        match self.properties {
            Value(v) => item.properties = Set(Some(v)),
            Null => item.properties = Set(None),
            Ignore => {}
        }
        item
    }
    pub fn new(id: i64) -> Self {
        UpdateTradeEntry {
            id,
            price: FieldChange::Ignore,
            tags: FieldChange::Ignore,
            sub_type: FieldChange::Ignore,
            properties: FieldChange::Ignore,
        }
    }
}
