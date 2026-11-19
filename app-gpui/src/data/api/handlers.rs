use std::future::Future;

use qf_app_lib::commands::handlers;
use serde_json::{Value, json};

use super::dto;
use crate::{
    data::{DataResult, DataSource, convert},
    model::HandleItem,
};

impl DataSource {
    pub fn handles_handle_items(
        &self,
        items: Vec<HandleItem>,
    ) -> impl Future<Output = DataResult<i64>> + Send + 'static {
        let payload = Value::Array(
            items
                .into_iter()
                .map(|item| {
                    json!({
                        "wfm_url": item.wfm_url,
                        "sub_type": convert::backend_sub_type(item.sub_type),
                        "quantity": item.quantity,
                        "price": item.price,
                        "user_name": item.user_name,
                        "order_type": convert::backend_order_type(item.order_type),
                        "operations": item.operations,
                    })
                })
                .collect(),
        );
        self.call(async move {
            let handled = handlers::handles_handle_items(dto(payload)?).await?;
            Ok(i64::from(handled))
        })
    }
}
