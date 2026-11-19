use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use utils::{info, LoggerOptions};
use wf_market::Client;

pub const RECOVER_AFTER: Duration = Duration::from_secs(5 * 60);

static LAST_LIMITED: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

pub fn note_rate_limited(route: impl Into<String>, at: Instant) {
    if let Ok(mut last) = LAST_LIMITED.lock() {
        last.get_or_insert_with(HashMap::new).insert(route.into(), at);
    }
}

pub fn last_rate_limited(route: &str) -> Option<Instant> {
    LAST_LIMITED
        .lock()
        .ok()
        .and_then(|last| last.as_ref().and_then(|map| map.get(route).copied()))
}

pub fn recover<S: Clone + 'static>(client: &Client<S>, now: Instant) -> Vec<String> {
    let limiters = client.get_per_route_limiter();
    let Ok(mut limiters) = limiters.lock() else {
        return Vec::new();
    };
    let Ok(mut last) = LAST_LIMITED.lock() else {
        return Vec::new();
    };
    let last = last.get_or_insert_with(HashMap::new);
    let recovered: Vec<String> = limiters
        .keys()
        .filter(|route| {
            last.get(*route)
                .is_none_or(|at| now.saturating_duration_since(*at) >= RECOVER_AFTER)
        })
        .cloned()
        .collect();
    for route in &recovered {
        limiters.remove(route);
        last.remove(route);
        info(
            "LiveScraper:RateLimit",
            &format!("No 429 on {route} for {} s, back to the default rate", RECOVER_AFTER.as_secs()),
            &LoggerOptions::default(),
        );
    }
    recovered
}
