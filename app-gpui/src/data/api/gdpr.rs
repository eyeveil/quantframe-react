use std::{future::Future, path::PathBuf};

use qf_app_lib::{
    State,
    commands::warframe_gdpr,
    enums::TradeItemType,
    log_parser::{Account, PlayerTrade, TradeClassification, TradeItem},
};

use crate::{
    data::{DataResult, DataSource},
    model::{
        GdprAccount, GdprItem, GdprLogin, GdprPayment, GdprPurchase, GdprTrade, GdprTradeKind,
    },
};

fn items(items: Vec<TradeItem>) -> Vec<GdprItem> {
    items
        .into_iter()
        .filter(|item| {
            !matches!(
                item.item_type,
                TradeItemType::Platinum | TradeItemType::Credits
            )
        })
        .map(|item| {
            let name = item
                .properties
                .get_property_value("item_name", String::new());
            GdprItem {
                name: if name.is_empty() { item.raw } else { name },
                quantity: item.quantity,
                tags: item.properties.get_property_value("tags", Vec::new()),
            }
        })
        .collect()
}

fn trade(trade: PlayerTrade) -> GdprTrade {
    GdprTrade {
        time: trade.trade_time.to_rfc3339(),
        kind: match trade.trade_type {
            TradeClassification::Sale => GdprTradeKind::Sale,
            TradeClassification::Purchase => GdprTradeKind::Purchase,
            _ => GdprTradeKind::Swap,
        },
        platinum: trade.platinum,
        credits: trade.credits,
        gave: items(trade.offered_items),
        received: items(trade.received_items),
    }
}

fn account(account: Account) -> GdprAccount {
    GdprAccount {
        oid: account.oid,
        email: account.email,
        display_name: account.display_name,
        activated: account.activated,
        subscribed_to_emails: account.subscribed_to_emails,
        signup_language: account.signup_language,
        signup_country_code: account.signup_country_code,
        country_code: account.country_code,
        signup_page: account.signup_page,
        language: account.language,
        ips: account.ips,
        created_at: account.account_creation_date.map(|date| date.to_rfc3339()),
        last_login_at: account.last_login_date.map(|date| date.to_rfc3339()),
        trades: account.trades.into_iter().map(trade).collect(),
        logins: account
            .logins
            .into_iter()
            .map(|login| GdprLogin {
                time: login.date.to_rfc3339(),
                ip: login.ip,
                client: login.client_type,
            })
            .collect(),
        purchases: account
            .purchases
            .into_iter()
            .map(|purchase| GdprPurchase {
                time: purchase.date.to_rfc3339(),
                shop_id: purchase.shop_id,
                platinum: purchase.price,
                items: purchase
                    .items_received
                    .into_iter()
                    .map(|item| GdprItem {
                        name: item.name,
                        quantity: item.quantity,
                        tags: Vec::new(),
                    })
                    .collect(),
            })
            .collect(),
        payments: account
            .transactions
            .into_iter()
            .map(|payment| GdprPayment {
                time: payment.date.to_rfc3339(),
                sku: payment.sku,
                price: payment.price,
                currency: payment.currency,
                vendor: payment.vendor,
                account: payment.account,
            })
            .collect(),
    }
}

impl DataSource {
    pub fn wfgdpr_load(
        &self,
        path: PathBuf,
    ) -> impl Future<Output = DataResult<()>> + Send + 'static {
        self.call(async move {
            let path = path.to_string_lossy().into_owned();
            Ok(warframe_gdpr::wfgdpr_load(path, State::get()).await?)
        })
    }

    pub fn wfgdpr_get_accounts(
        &self,
    ) -> impl Future<Output = DataResult<Vec<GdprAccount>>> + Send + 'static {
        self.call(async move {
            let value = warframe_gdpr::wfgdpr_get_accounts(State::get()).await?;
            let accounts: Vec<Account> = serde_json::from_value(value)?;
            Ok(accounts.into_iter().map(account).collect())
        })
    }
}
