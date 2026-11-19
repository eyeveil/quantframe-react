use std::{future::Future, time::Duration};

use utils::{warning, LoggerOptions};
use wf_market::errors::ApiError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub attempts: u32,
    pub base: Duration,
    pub max: Duration,
}

pub const SERVER_ERRORS: RetryPolicy = RetryPolicy {
    attempts: 4,
    base: Duration::from_millis(500),
    max: Duration::from_secs(4),
};

impl RetryPolicy {
    pub fn delay(&self, retry: u32) -> Duration {
        self.base
            .saturating_mul(2u32.saturating_pow(retry))
            .min(self.max)
    }
}

pub async fn retry_server_errors<T, F, Fut>(
    label: &str,
    policy: &RetryPolicy,
    mut call: F,
) -> Result<T, ApiError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ApiError>>,
{
    let mut retry = 0;
    loop {
        match call().await {
            Err(ApiError::InternalServerError(error)) if retry + 1 < policy.attempts => {
                let delay = policy.delay(retry);
                warning(
                    "LiveScraper:Retry",
                    &format!(
                        "{label} got status {:?}, retry {} of {} in {} ms",
                        error.status_code,
                        retry + 1,
                        policy.attempts - 1,
                        delay.as_millis()
                    ),
                    &LoggerOptions::default(),
                );
                tokio::time::sleep(delay).await;
                retry += 1;
            }
            other => return other,
        }
    }
}
