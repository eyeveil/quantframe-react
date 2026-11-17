use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use utils::{get_location, info, Error, LoggerOptions};

use crate::{
    host::{PACKAGE_AUTHORS, PACKAGE_NAME, PACKAGE_VERSION},
    HTTP_CLIENT,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscordNotify {
    pub enabled: bool,
    pub content: String,
    pub webhook: String,
    pub user_ids: Vec<String>,
}
impl DiscordNotify {
    pub fn new(
        content: impl Into<String>,
        webhook: impl Into<String>,
        user_ids: Vec<String>,
    ) -> Self {
        Self {
            enabled: true,
            content: content.into(),
            webhook: webhook.into(),
            user_ids,
        }
    }
    pub fn send(&self, variables: &HashMap<String, String>) {
        let variables = &mut variables.clone();
        if self.webhook.is_empty() {
            return;
        }
        let mut tags = Vec::new();
        for user_id in &self.user_ids {
            if !user_id.is_empty() {
                tags.push(format!("<@{}>", user_id));
            }
        }
        if !tags.is_empty() {
            variables.insert("<MENTION>".to_string(), tags.join(" ").to_string());
        } else {
            variables.insert("<MENTION>".to_string(), "".to_string());
        }
        let mut content = self.content.clone();
        for (k, v) in variables.iter() {
            content = content.replace(&format!("{}", k), v);
        }
        let webhook = self.webhook.clone();
        crate::host::spawn(async move {
            let client = HTTP_CLIENT.get_or_init(reqwest::Client::new);
            let timestamp = chrono::Local::now()
                .to_utc()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string();
            let body = json!({
                "content": content,
                "embeds": [
                    {
                        "description": "",
                        "color": 5814783,
                        "footer": {
                            "text": format!("{} v{} ({})", PACKAGE_NAME, PACKAGE_VERSION, PACKAGE_AUTHORS),
                            "icon_url": "https://raw.githubusercontent.com/Kenya-DK/quantframe-react/refs/heads/main/app-icon.png"
                        },
                        "timestamp": timestamp
                    }
                ]
            });
            let res = client.post(webhook).json(&body).send().await;
            match res {
                Ok(_) => {
                    info(
                        "Helper",
                        "Message sent to discord",
                        &LoggerOptions::default(),
                    );
                }
                Err(e) => {
                    let err = Error::new(
                        "DiscordNotificationError",
                        &format!("{:?}", e),
                        get_location!(),
                    );
                    err.log("discord_notification.log");
                }
            }
        });
    }
}
