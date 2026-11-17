use serde::{Deserialize, Serialize};
use serde_json::Value;
use utils::{get_location, info, Error, LoggerOptions};

use crate::{
    host::{PACKAGE_AUTHORS, PACKAGE_NAME, PACKAGE_VERSION},
    HTTP_CLIENT,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebHookNotify {
    pub enabled: bool,
    pub url: String,
}
impl WebHookNotify {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            enabled: false,
            url: url.into(),
        }
    }
    pub fn send(&self, value: Value) {
        if self.url.is_empty() {
            return;
        }
        let url = self.url.clone();
        crate::host::spawn(async move {
            let client = HTTP_CLIENT.get_or_init(reqwest::Client::new);
            let res = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header(
                    "User-Agent",
                    format!(
                        "{} v{} ({})",
                        PACKAGE_NAME, PACKAGE_VERSION, PACKAGE_AUTHORS
                    ),
                )
                .json(&value)
                .send()
                .await;
            match res {
                Ok(_) => {
                    info(
                        "Helper",
                        &format!("Message sent to webhook: {}", url),
                        &LoggerOptions::default(),
                    );
                }
                Err(e) => {
                    let err = Error::new(
                        "WebhookNotificationError",
                        &format!("{:?}", e),
                        get_location!(),
                    );
                    err.log("webhook_notification.log");
                }
            }
        });
    }
}
