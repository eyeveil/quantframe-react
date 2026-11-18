//! The prompts several screens open: sold, bought, a price limit, delete and
//! the trade chat message.

use super::form::{Field, FormSpec, FormValues, Preview};

/// The longest message Warframe's trade chat accepts.
pub const CHAT_LIMIT: usize = 180;

/// Asks how many units sold and for how much. `max_quantity` is `None` for
/// things that sell one at a time, such as a riven.
pub fn sold(name: &str, max_quantity: Option<i64>, total: Option<i64>) -> FormSpec {
    traded("Sell", "Record sale", name, max_quantity, total)
}

/// Asks how many units were bought and for how much.
pub fn bought(name: &str, max_quantity: Option<i64>, total: Option<i64>) -> FormSpec {
    traded("Buy", "Record purchase", name, max_quantity, total)
}

fn traded(
    verb: &str,
    confirm: &str,
    name: &str,
    max_quantity: Option<i64>,
    total: Option<i64>,
) -> FormSpec {
    let spec = FormSpec::new(format!("{verb} {name}"), confirm);
    match max_quantity {
        Some(quantity) => spec
            .field(
                Field::number("quantity", "Quantity", Some(quantity))
                    .min(1)
                    .half(),
            )
            .field(
                Field::platinum("price", "Total price", total)
                    .hint("For all units together.")
                    .half(),
            ),
        None => spec.field(Field::platinum("price", "Price", total)),
    }
}

/// Asks for a price limit. An empty field removes the limit.
pub fn price_limit(title: &str, label: &str, hint: &str, value: Option<i64>) -> FormSpec {
    FormSpec::new(title, "Save").field(
        Field::platinum("price", label, value)
            .optional()
            .hint(hint),
    )
}

/// Asks before deleting `count` rows. `singular` and `plural` name the rows.
pub fn delete(count: usize, singular: &str, plural: &str) -> FormSpec {
    let (title, text) = if count == 1 {
        (
            format!("Delete this {singular}?"),
            "You can't undo this.".to_owned(),
        )
    } else {
        (
            format!("Delete {count} {plural}?"),
            format!("This deletes all {count} {plural}. You can't undo this."),
        )
    };
    FormSpec::confirm_danger(title, text, "Delete")
}

/// One item of a trade chat message.
#[derive(Clone, Debug)]
pub struct MessageItem {
    pub name: String,
    pub rank: Option<String>,
    pub price: Option<i64>,
}

const DEFAULT_TEMPLATE: &str = "[<name>] <rank> <price>p";

/// Asks for the wording of a trade chat message and previews it. The
/// submitted values go to [`compose_message`].
pub fn message(prefix: &str, items: Vec<MessageItem>) -> FormSpec {
    let count = items.len();
    FormSpec::new(
        if count == 1 {
            "Trade chat message for 1 item".to_owned()
        } else {
            format!("Trade chat message for {count} items")
        },
        "Copy message",
    )
    .description(format!(
        "Warframe's trade chat takes {CHAT_LIMIT} characters. Items that don't fit are left out."
    ))
    .width(560.)
    .field(Field::text("prefix", "Prefix", prefix, "WTS").half())
    .field(
        Field::text("suffix", "Suffix", ":heart:", "")
            .optional()
            .half(),
    )
    .field(
        Field::text("template", "Item template", DEFAULT_TEMPLATE, DEFAULT_TEMPLATE)
            .hint("Use <name>, <rank> and <price>."),
    )
    .preview(move |values| Preview {
        label: "Message",
        text: compose_message(values, &items),
        limit: Some(CHAT_LIMIT),
    })
}

/// Builds the message from the prompt's values, adding items while they fit.
pub fn compose_message(values: &FormValues, items: &[MessageItem]) -> String {
    let prefix = values.text("prefix");
    let suffix = values.text("suffix");
    let template = values.text("template");
    let room = CHAT_LIMIT.saturating_sub(suffix.chars().count() + 1);
    let mut message = prefix;
    for item in items {
        let entry = template
            .replace("<name>", &item.name)
            .replace("<rank>", item.rank.as_deref().unwrap_or(""))
            .replace(
                "<price>",
                &item.price.map(|price| price.to_string()).unwrap_or_default(),
            );
        let entry = entry.split_whitespace().collect::<Vec<_>>().join(" ");
        if message.chars().count() + 1 + entry.chars().count() > room {
            break;
        }
        if !message.is_empty() {
            message.push(' ');
        }
        message.push_str(&entry);
    }
    if !suffix.is_empty() {
        message.push(' ');
        message.push_str(&suffix);
    }
    message
}
