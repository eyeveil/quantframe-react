//! warframe.market orders, auctions and chats.

use std::{collections::BTreeMap, future::Future};

use qf_app_lib::{
    State,
    commands::{auction, chat, order},
};
use serde_json::json;

use super::{dto, list};
use crate::{
    data::{DataResult, DataSource, convert},
    model::*,
};

impl DataSource {
    pub fn get_wfm_orders_pagination(
        &self,
        params: &OrderListParams,
    ) -> impl Future<Output = DataResult<Paginated<WfmOrder>>> + Send + 'static {
        let mut query = list(&params.list);
        if let Some(kind) = params.order_type {
            query.insert(
                "order_type".into(),
                json!(convert::backend_order_type(kind)),
            );
        }
        self.call(async move {
            let page = order::get_wfm_orders_pagination(dto(query.into())?, State::get())?;
            Ok(convert::paginated(page, convert::order))
        })
    }

    pub fn get_wfm_orders_status_counts(
        &self,
    ) -> impl Future<Output = DataResult<BTreeMap<OrderType, OrderTotals>>> + Send + 'static {
        self.call(async move {
            let counts =
                order::get_wfm_orders_status_counts(dto(super::all())?, State::get()).await?;
            Ok(counts
                .into_iter()
                .filter_map(|(kind, (count, platinum, potential_profit))| {
                    let kind = match kind.as_str() {
                        "buy" => OrderType::Buy,
                        "sell" => OrderType::Sell,
                        _ => return None,
                    };
                    Some((
                        kind,
                        OrderTotals {
                            count,
                            platinum,
                            potential_profit,
                        },
                    ))
                })
                .collect())
        })
    }

    pub fn get_wfm_order_by_id(
        &self,
        id: String,
    ) -> impl Future<Output = DataResult<WfmOrder>> + Send + 'static {
        self.call(async move {
            let order = order::get_wfm_order_by_id(
                id,
                Some(vec![
                    "TransactionInfo".to_owned(),
                    "ProfitabilityInfo".to_owned(),
                ]),
                State::get(),
                State::get(),
            )
            .await?;
            Ok(convert::order(order))
        })
    }

    /// Reloads your orders from warframe.market. Fixture mode returns an error.
    pub fn order_refresh(&self) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(order::order_refresh(State::get(), State::get()).await?) })
    }

    pub fn order_delete_all(
        &self,
        order_type: Option<OrderType>,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move {
            Ok(order::order_delete_all(
                order_type.map(convert::backend_order_type),
                State::get(),
                State::get(),
                State::get(),
            )
            .await?)
        })
    }

    pub fn order_delete_by_id(
        &self,
        id: String,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(order::order_delete_by_id(id, State::get()).await?) })
    }

    pub fn get_wfm_auctions_pagination(
        &self,
        params: &ListParams,
    ) -> impl Future<Output = DataResult<Paginated<WfmAuction>>> + Send + 'static {
        let query = list(params);
        self.call(async move {
            let page =
                auction::get_wfm_auctions_pagination(dto(query.into())?, State::get()).await?;
            Ok(convert::paginated(page, convert::auction))
        })
    }

    pub fn get_wfm_auctions_overview(
        &self,
    ) -> impl Future<Output = DataResult<AuctionOverview>> + Send + 'static {
        self.call(async move {
            let (count, revenue, potential_profit) =
                auction::get_wfm_auctions_overview(dto(super::all())?, State::get()).await?;
            Ok(AuctionOverview {
                count,
                revenue: i64::from(revenue),
                potential_profit,
            })
        })
    }

    pub fn get_wfm_auction_by_id(
        &self,
        id: String,
    ) -> impl Future<Output = DataResult<WfmAuction>> + Send + 'static {
        self.call(async move {
            let auction = auction::get_wfm_auction_by_id(
                id,
                Some(vec!["TransactionInfo".to_owned()]),
                State::get(),
                State::get(),
            )
            .await?;
            Ok(convert::auction(auction))
        })
    }

    /// Reloads your auctions from warframe.market. Fixture mode returns an error.
    pub fn auction_refresh(&self) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(auction::auction_refresh(State::get(), State::get()).await?) })
    }

    pub fn auction_delete_all(&self) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(auction::auction_delete_all(State::get(), State::get()).await?) })
    }

    pub fn auction_delete_by_id(
        &self,
        id: String,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(auction::auction_delete_by_id(id, State::get()).await?) })
    }

    /// Adds one of your warframe.market auctions to the stock rivens.
    pub fn auction_import_by_id(
        &self,
        id: String,
        bought: i64,
    ) -> impl Future<Output = DataResult<StockRiven>> + Send + 'static {
        self.call(async move {
            Ok(convert::stock_riven(
                auction::auction_import_by_id(id, bought, State::get()).await?,
            ))
        })
    }

    pub fn get_chat_pagination(
        &self,
        params: &ListParams,
    ) -> impl Future<Output = DataResult<Paginated<Chat>>> + Send + 'static {
        let query = list(params);
        self.call(async move {
            let page = chat::get_chat_pagination(dto(query.into())?, State::get()).await?;
            Ok(convert::paginated(page, convert::chat))
        })
    }

    pub fn chat_get_messages_by_id(
        &self,
        id: String,
    ) -> impl Future<Output = DataResult<Vec<ChatMessage>>> + Send + 'static {
        self.call(async move {
            let messages = chat::chat_get_messages_by_id(id, State::get()).await?;
            Ok(messages.into_iter().map(convert::chat_message).collect())
        })
    }

    pub fn chat_send_message(
        &self,
        id: String,
        message: String,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(chat::chat_send_message(id, message, State::get()).await?) })
    }

    pub fn chat_delete(&self, id: String) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move {
            chat::chat_delete(id, State::get()).await?;
            Ok(())
        })
    }

    pub fn chat_refresh(&self) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(chat::chat_refresh(State::get()).await?) })
    }

    /// Marks the chat on screen, so its new messages don't count as unread.
    pub fn chat_set_active(
        &self,
        id: Option<String>,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move { Ok(chat::chat_set_active(id).await?) })
    }
}
