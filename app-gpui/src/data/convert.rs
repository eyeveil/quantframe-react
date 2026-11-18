//! Maps backend types onto the view models in `crate::model`.

use qf_app_lib::{
    entity::{
        dto::{FinancialReport as BackendReport, PaginatedResult, PriceHistory as BackendPrice},
        enums::{
            StockStatus as BackendStatus, TransactionItemType as BackendItemType,
            TransactionType as BackendTransactionType,
        },
        stock_item, stock_riven, syndicate_item, trade_entry, transaction, wish_list,
    },
    qf_api::types as qf,
    qf_utils::{Properties, SubType as BackendSubType},
    utils::SubTypeExt,
    wf_market::{enums as wfm_enums, types as wfm},
};
use serde_json::Value;

use crate::model::*;

pub fn paginated<A, B>(page: PaginatedResult<A>, map: impl Fn(A) -> B) -> Paginated<B> {
    Paginated {
        total: page.total,
        limit: page.limit,
        page: page.page,
        total_pages: page.total_pages,
        results: page.results.into_iter().map(map).collect(),
    }
}

pub fn qf_paginated<A, B>(page: qf::Paginated<A>, map: impl Fn(A) -> B) -> Paginated<B> {
    Paginated {
        total: page.total,
        limit: page.limit,
        page: page.page,
        total_pages: page.total_pages,
        results: page.results.into_iter().map(map).collect(),
    }
}

pub fn sub_type(sub: Option<BackendSubType>) -> Option<SubType> {
    sub.map(|sub| SubType {
        rank: sub.rank,
        charges: sub.charges,
        variant: sub.variant,
        amber_stars: sub.amber_stars,
        cyan_stars: sub.cyan_stars,
    })
}

pub fn backend_sub_type(sub: Option<SubType>) -> Option<BackendSubType> {
    sub.map(|sub| BackendSubType {
        rank: sub.rank,
        charges: sub.charges,
        variant: sub.variant,
        amber_stars: sub.amber_stars,
        cyan_stars: sub.cyan_stars,
    })
}

pub fn status(status: BackendStatus) -> StockStatus {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(StockStatus::parse))
        .unwrap_or(StockStatus::Unknown)
}

fn price_history(history: Vec<BackendPrice>) -> Vec<PriceHistory> {
    history
        .into_iter()
        .map(|entry| PriceHistory {
            created_at: entry.created_at,
            price: entry.price,
        })
        .collect()
}

pub fn report(report: BackendReport) -> FinancialReport {
    FinancialReport {
        total_transactions: report.total_transactions as i64,
        average_transaction: report.average_transaction,
        total_profit: report.total_profit,
        average_profit: report.average_profit,
        profit_margin: report.profit_margin,
        roi: report.roi,
        sale_count: report.sale_count as i64,
        highest_revenue: report.highest_revenue.round() as i64,
        lowest_revenue: report.lowest_revenue.round() as i64,
        average_revenue: report.average_revenue,
        revenue: report.revenue.round() as i64,
        purchases_count: report.purchases_count as i64,
        highest_expense: report.highest_expense.round() as i64,
        lowest_expense: report.lowest_expense.round() as i64,
        average_expense: report.average_expense,
        expenses: report.expenses.round() as i64,
    }
}

fn cooldown(properties: &Properties) -> Option<CoolDownInfo> {
    let value: Option<Value> = properties.get_property_value("cooldown", None);
    let value = value?;
    let text = |key: &str| value[key].as_str().unwrap_or_default().to_owned();
    Some(CoolDownInfo {
        id: text("id"),
        cooldown_type: text("cooldown_type"),
        start_time: text("start_time"),
        end_time: text("end_time"),
    })
}

pub fn stock_item(model: stock_item::Model) -> StockItem {
    let properties = StockItemProperties {
        min_price: model.properties.get_property_value("min_price", None),
        min_sma: model.properties.get_property_value("min_sma", None),
        min_profit: model.properties.get_property_value("min_profit", None),
        is_bulk: model.properties.get_property_value("is_bulk", false),
        cooldown: cooldown(&model.properties),
    };
    StockItem {
        id: model.id,
        wfm_id: model.wfm_id,
        wfm_url: model.wfm_url,
        item_name: model.item_name,
        item_unique_name: model.item_unique_name,
        sub_type: sub_type(model.sub_type),
        bought: model.bought,
        list_price: model.list_price,
        owned: model.owned,
        is_hidden: model.is_hidden,
        status: status(model.status),
        price_history: price_history(model.price_history.0),
        updated_at: model.updated_at.to_rfc3339(),
        created_at: model.created_at.to_rfc3339(),
        properties,
    }
}

pub fn stock_riven(model: stock_riven::Model) -> StockRiven {
    StockRiven {
        id: model.id,
        uuid: model.uuid,
        wfm_weapon_id: model.wfm_weapon_id,
        wfm_weapon_url: model.wfm_weapon_url,
        weapon_name: model.weapon_name,
        weapon_type: model.weapon_type,
        weapon_unique_name: model.weapon_unique_name,
        mod_name: model.mod_name,
        rank: model.sub_type.and_then(|sub| sub.rank),
        mastery_rank: model.mastery_rank,
        re_rolls: model.re_rolls,
        polarity: model.polarity,
        attributes: model
            .attributes
            .0
            .into_iter()
            .map(|attribute| RivenAttribute {
                wfm_url: attribute.wfm_url,
                positive: attribute.positive,
                value: attribute.value,
            })
            .collect(),
        bought: model.bought,
        list_price: model.list_price,
        is_hidden: model.is_hidden,
        comment: model.comment,
        status: status(model.status),
        price_history: price_history(model.price_history.0),
        updated_at: model.updated_at.to_rfc3339(),
        created_at: model.created_at.to_rfc3339(),
    }
}

pub fn wish_list_item(model: wish_list::Model) -> WishListItem {
    WishListItem {
        id: model.id,
        wfm_id: model.wfm_id,
        wfm_url: model.wfm_url,
        item_name: model.item_name,
        item_unique_name: model.item_unique_name,
        sub_type: sub_type(model.sub_type),
        quantity: model.quantity,
        list_price: model.list_price,
        is_hidden: model.is_hidden,
        status: status(model.status),
        price_history: price_history(model.price_history.0),
        updated_at: model.updated_at.to_rfc3339(),
        created_at: model.created_at.to_rfc3339(),
    }
}

pub fn syndicate_item(model: syndicate_item::Model) -> SyndicateItem {
    SyndicateItem {
        id: model.id,
        wfm_id: model.wfm_id,
        wfm_url: model.wfm_url,
        item_name: model.item_name,
        item_unique_name: model.item_unique_name,
        sub_type: sub_type(model.sub_type),
        list_price: model.list_price,
        syndicate_name: model.syndicate_name,
        syndicate_unique_name: model.syndicate_unique_name,
        standing_cost: model.standing_cost,
        status: status(model.status),
        price_history: price_history(model.price_history.0),
        updated_at: model.updated_at.to_rfc3339(),
        created_at: model.created_at.to_rfc3339(),
    }
}

fn tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn trade_entry(model: trade_entry::Model) -> TradeEntry {
    TradeEntry {
        id: model.id,
        wfm_id: model.wfm_id,
        name: model.name,
        sub_type: sub_type(model.sub_type),
        price: model.price,
        group: model.group,
        tags: tags(&model.tags),
        updated_at: model.updated_at.to_rfc3339(),
        created_at: model.created_at.to_rfc3339(),
    }
}

pub fn transaction_type(kind: BackendTransactionType) -> TransactionType {
    match kind {
        BackendTransactionType::Sale => TransactionType::Sale,
        BackendTransactionType::Purchase => TransactionType::Purchase,
    }
}

pub fn backend_transaction_type(kind: TransactionType) -> BackendTransactionType {
    match kind {
        TransactionType::Sale => BackendTransactionType::Sale,
        TransactionType::Purchase => BackendTransactionType::Purchase,
    }
}

pub fn backend_item_type(kind: TransactionItemType) -> BackendItemType {
    match kind {
        TransactionItemType::Item => BackendItemType::Item,
        TransactionItemType::Riven => BackendItemType::Riven,
    }
}

pub fn transaction(model: transaction::Model) -> Transaction {
    Transaction {
        id: model.id,
        wfm_id: model.wfm_id,
        wfm_url: model.wfm_url,
        item_name: model.item_name,
        item_type: match model.item_type {
            BackendItemType::Riven => TransactionItemType::Riven,
            _ => TransactionItemType::Item,
        },
        item_unique_name: model.item_unique_name,
        sub_type: sub_type(model.sub_type),
        tags: tags(&model.tags),
        transaction_type: transaction_type(model.transaction_type),
        quantity: model.quantity,
        user_name: model.user_name,
        price: model.price,
        profit: model.profit,
        credits: model.credits,
        updated_at: model.updated_at.to_rfc3339(),
        created_at: model.created_at.to_rfc3339(),
    }
}

pub fn order_type(kind: wfm_enums::OrderType) -> OrderType {
    match kind {
        wfm_enums::OrderType::Buy => OrderType::Buy,
        wfm_enums::OrderType::Sell => OrderType::Sell,
    }
}

pub fn backend_order_type(kind: OrderType) -> wfm_enums::OrderType {
    match kind {
        OrderType::Buy => wfm_enums::OrderType::Buy,
        OrderType::Sell => wfm_enums::OrderType::Sell,
    }
}

pub fn order(order: wfm::Order) -> WfmOrder {
    WfmOrder {
        item_name: order
            .properties
            .get_property_value("name", order.item_id.clone()),
        item_url: order
            .properties
            .get_property_value("wfm_url", String::new()),
        sub_type: sub_type(order.subtype.to_entity()),
        id: order.id,
        order_type: order_type(order.order_type),
        item_id: order.item_id,
        platinum: i64::from(order.platinum),
        quantity: i64::from(order.quantity),
        per_trade: order.per_trade.map(i64::from),
        visible: order.visible,
        created_at: order.created_at,
        updated_at: order.updated_at,
    }
}

pub fn auction(auction: wfm::Auction) -> WfmAuction {
    let item = auction.item;
    WfmAuction {
        name: auction
            .properties
            .get_property_value("name", item.weapon_url_name.clone()),
        id: auction.id,
        uuid: auction.uuid,
        item_type: serde_json::to_value(item.item_type)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default(),
        weapon_url: item.weapon_url_name,
        mod_name: auction
            .properties
            .get_property_value::<Option<String>>("mod_name", None)
            .or(item.mod_name),
        mastery_level: item.mastery_level.map(i64::from),
        mod_rank: item.mod_rank.map(i64::from),
        re_rolls: item.re_rolls.map(i64::from),
        polarity: item.polarity,
        attributes: item
            .attributes
            .unwrap_or_default()
            .into_iter()
            .map(|attribute| RivenAttribute {
                wfm_url: attribute.url_name,
                positive: attribute.positive,
                value: attribute.value,
            })
            .collect(),
        starting_price: i64::from(auction.starting_price),
        buyout_price: auction.buyout_price.map(i64::from),
        top_bid: auction.top_bid.map(i64::from),
        is_direct_sell: auction.is_direct_sell,
        visible: auction.visible,
        closed: auction.closed,
        note: auction.note,
        created: auction.created,
        updated: auction.updated,
    }
}

pub fn chat_message(message: wfm::ChatMessage) -> ChatMessage {
    ChatMessage {
        id: message.id,
        chat_id: message.chat_id,
        message: message.message,
        send_date: message.send_date,
        message_from: message.message_from,
    }
}

pub fn chat(chat: wfm::Chat) -> Chat {
    Chat {
        id: chat.id,
        chat_name: chat.chat_name,
        chat_with: chat
            .chat_with
            .into_iter()
            .map(|user| ChatUser {
                id: user.id,
                name: user.name,
                reputation: user.reputation,
                status: serde_json::to_value(&user.status)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_default(),
            })
            .collect(),
        unread_count: i64::from(chat.unread_count),
        last_update: chat.last_update,
        last_message: chat.messages.into_iter().last().map(chat_message),
    }
}

pub fn item_price(price: qf::ItemPrice) -> ItemPrice {
    ItemPrice {
        name: price.name,
        wfm_id: price.wfm_id,
        wfm_url: price.wfm_url,
        sub_type: sub_type(price.sub_type),
        avg_price: price.avg_price,
        closed_price: price.closed_price,
        volume: price.volume,
        supply: price.supply,
        demand: price.demand,
    }
}

pub fn riven_price(price: qf::RivenPrice) -> RivenPrice {
    RivenPrice {
        wfm_url: price.wfm_url,
        name: price.name,
        volume: price.volume,
        min_price: price.min_price,
        max_price: price.max_price,
        avg_price: price.avg_price,
    }
}

pub fn syndicate_price(price: qf::SyndicateItemPrice) -> SyndicateItemPrice {
    SyndicateItemPrice {
        wfm_id: price.wfm_id,
        name: price.name,
        syndicate: price.syndicate,
        standing_cost: price.standing_cost,
        min_price: price.min_price,
        max_price: price.max_price,
        volume: price.volume,
    }
}

fn from_value<T: serde::de::DeserializeOwned + Default>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

fn chart(value: &Value) -> Chart {
    Chart {
        labels: from_value(&value["labels"]),
        values: from_value(&value["values"]),
    }
}

/// Reads a serialized `entity::dto::FinancialReport`, which has no `Deserialize`.
fn report_value(value: &Value) -> FinancialReport {
    let int = |key: &str| value[key].as_f64().unwrap_or(0.0).round() as i64;
    let float = |key: &str| value[key].as_f64().unwrap_or(0.0);
    FinancialReport {
        total_transactions: int("total_transactions"),
        average_transaction: float("average_transaction"),
        total_profit: int("total_profit"),
        average_profit: float("average_profit"),
        profit_margin: float("profit_margin"),
        roi: float("roi"),
        sale_count: int("sale_count"),
        highest_revenue: int("highest_revenue"),
        lowest_revenue: int("lowest_revenue"),
        average_revenue: float("average_revenue"),
        revenue: int("revenue"),
        purchases_count: int("purchases_count"),
        highest_expense: int("highest_expense"),
        lowest_expense: int("lowest_expense"),
        average_expense: float("average_expense"),
        expenses: int("expenses"),
    }
}

/// Reads a key that `FinancialReport::with_properties` stored.
fn text(value: &Value, key: &str) -> String {
    value["properties"][key]
        .as_str()
        .or_else(|| value[key].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn period(value: &Value) -> PeriodSummary {
    PeriodSummary {
        summary: report_value(&value["summary"]),
        chart: chart(&value["chart"]),
    }
}

pub fn dashboard(value: Value) -> Result<DashboardSummary, serde_json::Error> {
    let recent: Vec<transaction::Model> =
        serde_json::from_value(value["resent_transactions"].clone())?;
    let best = &value["best_seller"];
    Ok(DashboardSummary {
        total: report_value(&value["total"]),
        present_year: period(&value["total"]["present_year"]),
        last_year: period(&value["total"]["last_year"]),
        today: period(&value["today"]),
        recent_days: period(&value["recent_days"]),
        best_seller: BestSeller {
            wfm_id: text(best, "wfm_id"),
            item_name: text(best, "item_name"),
            report: report_value(best),
        },
        categories: value["categories"]
            .as_array()
            .map(|categories| {
                categories
                    .iter()
                    .map(|category| CategorySummary {
                        name: text(category, "name"),
                        icon: text(category, "icon"),
                        report: report_value(category),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        recent_transactions: recent.into_iter().map(transaction).collect(),
    })
}

pub fn wfm_debug_state(value: &Value) -> WfmDebugState {
    let orders = &value["user_orders"];
    let mut tracking: Vec<(String, u64)> = value["tracking"]
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(route, count)| (route.clone(), count.as_u64().unwrap_or(0)))
                .collect()
        })
        .unwrap_or_default();
    tracking.sort();
    let mut limiters: Vec<RouteLimiter> = value["limiters"]
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(route, limiter)| RouteLimiter {
                    route: route.clone(),
                    limit: limiter["limit"].as_i64().unwrap_or(0),
                    wait_time_sec: limiter["wait_time_sec"].as_f64().unwrap_or(0.0),
                    quota_type: limiter["quota_type"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                })
                .collect()
        })
        .unwrap_or_default();
    limiters.sort_by(|a, b| a.route.cmp(&b.route));
    let count = |value: &Value| value.as_array().map_or(0, Vec::len);
    WfmDebugState {
        sell_orders: count(&orders["sell_orders"]),
        buy_orders: count(&orders["buy_orders"]),
        auctions: count(&value["user_auctions"]["auctions"]),
        order_limit: value["order_limit"].as_i64().unwrap_or(0),
        tracking,
        limiters,
    }
}

pub fn inventory_rivens(value: &Value) -> Paginated<InventoryRiven> {
    paginated_value(value, |riven| {
        let base = &riven["base"];
        let text = |key: &str| base[key].as_str().unwrap_or_default().to_owned();
        InventoryRiven {
            uuid: text("uuid"),
            name: text("name"),
            wfm_url: text("wfm_url"),
            mod_name: text("mod_name"),
            mastery_rank: base["mastery_rank"].as_i64().unwrap_or(0),
            re_rolls: base["re_rolls"].as_i64().unwrap_or(0),
            rank: base["sub_type"]["rank"].as_i64().unwrap_or(0),
            polarity: text("polarity"),
            riven_type: riven["riven_type"].as_str().unwrap_or_default().to_owned(),
            is_in_stock: base["is_in_stock"].as_bool().unwrap_or(false),
        }
    })
}

pub fn inventory_syndicates(value: &Value) -> Paginated<InventorySyndicate> {
    paginated_value(value, |item| InventorySyndicate {
        name: item["name"].as_str().unwrap_or_default().to_owned(),
        unique_name: item["unique_name"].as_str().unwrap_or_default().to_owned(),
        standing: item["quantity"].as_i64().unwrap_or(0),
        total_standing: item["total"].as_i64().unwrap_or(0),
    })
}

fn paginated_value<T>(value: &Value, map: impl Fn(&Value) -> T) -> Paginated<T> {
    Paginated {
        total: value["total"].as_i64().unwrap_or(0),
        limit: value["limit"].as_i64().unwrap_or(0),
        page: value["page"].as_i64().unwrap_or(1),
        total_pages: value["total_pages"].as_i64().unwrap_or(0),
        results: value["results"]
            .as_array()
            .map(|results| results.iter().map(map).collect())
            .unwrap_or_default(),
    }
}
