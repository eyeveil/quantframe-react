#![allow(dead_code)]

use std::{
    future::Future,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, OnceLock},
};

use qf_app_lib::{
    app::{AppState, Settings},
    host::{self, Host},
    live_scraper::{ItemEntry, LiveScraperState},
    qf_utils::Properties,
    utils::modules::states,
    wf_market::types::{Order, OrderList, OrderWithUser},
    HeadlessOptions,
};
use serde_json::{json, Value};
use tokio::runtime::Runtime;

#[derive(Default)]
pub struct RecordingHost {
    pub events: Mutex<Vec<(String, Value)>>,
}

impl Host for RecordingHost {
    fn emit(&self, event: &str, payload: Value) {
        self.events
            .lock()
            .unwrap()
            .push((event.to_owned(), payload));
    }

    fn notify(&self, _title: &str, _body: &str) {}
}

impl RecordingHost {
    pub fn ui_events(&self, name: &str) -> Vec<Value> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, payload)| payload["event"] == name)
            .map(|(_, payload)| payload["data"].clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentRequest {
    pub method: String,
    pub url: String,
}

pub struct Harness {
    pub host: Arc<RecordingHost>,
    pub proxy_lines: Arc<Mutex<Vec<String>>>,
    pub sent: Arc<Mutex<Vec<SentRequest>>>,
}

impl Harness {
    pub fn sent(&self) -> Vec<SentRequest> {
        self.sent.lock().unwrap().clone()
    }

    pub fn writes(&self) -> Vec<SentRequest> {
        self.sent()
            .into_iter()
            .filter(|request| request.method != "GET")
            .collect()
    }

    pub fn proxy_lines(&self) -> Vec<String> {
        self.proxy_lines.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.sent.lock().unwrap().clear();
        self.proxy_lines.lock().unwrap().clear();
        self.host.events.lock().unwrap().clear();
    }
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures"))
}

fn interfaces() -> Vec<String> {
    std::fs::read_to_string("/proc/self/net/dev")
        .unwrap_or_default()
        .lines()
        .skip(2)
        .filter_map(|line| line.split(':').next())
        .map(|name| name.trim().to_owned())
        .collect()
}

pub fn assert_isolated() {
    let interfaces = interfaces();
    assert!(
        interfaces.iter().all(|name| name == "lo"),
        "Run this test with no network: `unshare -rn sh -c 'ip link set lo up && cargo test ...'`. Interfaces found: {interfaces:?}"
    );
}

fn spawn_refusing_proxy(lines: Arc<Mutex<Vec<String>>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("bind 127.0.0.1, run `ip link set lo up` inside the namespace first");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && head.len() < 8192 {
                match stream.read(&mut byte) {
                    Ok(1) => head.push(byte[0]),
                    _ => break,
                }
            }
            let first = String::from_utf8_lossy(&head)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();
            lines.lock().unwrap().push(first);
            let _ = stream
                .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    });
    port
}

pub async fn start() -> Harness {
    assert_isolated();
    let proxy_lines = Arc::new(Mutex::new(Vec::new()));
    let port = spawn_refusing_proxy(proxy_lines.clone());
    let proxy = format!("http://127.0.0.1:{port}");
    for key in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
        std::env::set_var(key, &proxy);
    }
    for key in ["NO_PROXY", "no_proxy"] {
        std::env::remove_var(key);
    }

    let recording = Arc::new(RecordingHost::default());
    let dyn_host: Arc<dyn Host> = recording.clone();
    qf_app_lib::init_headless(
        HeadlessOptions {
            fixtures: Some(fixtures_dir()),
            storage_dir: None,
            use_temp_db: false,
        },
        dyn_host,
    )
    .await
    .expect("init_headless");

    let sent = Arc::new(Mutex::new(Vec::new()));
    let recorder = sent.clone();
    states::app_state()
        .unwrap()
        .wfm_client
        .on("api:before", move |_, data| {
            recorder.lock().unwrap().push(SentRequest {
                method: data.get_property_value("method", String::new()),
                url: data.get_property_value("url", String::new()),
            });
        });

    Harness {
        host: recording,
        proxy_lines,
        sent,
    }
}

pub fn update_settings(change: impl FnOnce(&mut Settings)) {
    let state = host::managed::<Mutex<AppState>>().unwrap();
    let mut app = state.lock().unwrap();
    let mut settings = app.settings.clone();
    change(&mut settings);
    app.update_settings(settings).unwrap();
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static HARNESS: OnceLock<Harness> = OnceLock::new();
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn run<F, Fut>(test: F) -> Fut::Output
where
    F: FnOnce(&'static Harness) -> Fut,
    Fut: Future,
{
    let _guard = serial();
    let runtime = RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
    });
    runtime.block_on(async move {
        if HARNESS.get().is_none() {
            let harness = start().await;
            let _ = HARNESS.set(harness);
        }
        let harness = HARNESS.get().unwrap();
        reset_settings();
        harness.clear();
        test(harness).await
    })
}

pub fn reset_settings() {
    update_settings(|settings| {
        *settings = Settings::default();
        settings.live_scraper.general.auto_delete = false;
        settings.debugging.live_scraper.fake_orders = true;
    });
}

pub fn scraper() -> Arc<LiveScraperState> {
    let scraper = host::managed::<Arc<LiveScraperState>>().unwrap().clone();
    if !scraper.is_running() {
        scraper.start();
    }
    scraper.init_modules();
    scraper
}

pub fn entry(
    wfm_url: &str,
    wfm_id: &str,
    operations: &[&str],
    stock_id: Option<i64>,
    buy_quantity: i64,
    sell_quantity: i64,
) -> ItemEntry {
    ItemEntry::new(
        stock_id,
        None,
        None,
        wfm_url,
        wfm_id,
        None,
        0,
        buy_quantity,
        sell_quantity,
        operations.iter().map(|op| op.to_string()).collect(),
        "",
        Properties::default(),
    )
}

pub fn market_order(id: &str, order_type: &str, platinum: u32, user: &str) -> Value {
    json!({
        "id": id,
        "type": order_type,
        "platinum": platinum,
        "quantity": 1,
        "visible": true,
        "itemId": "unused",
        "createdAt": "2026-11-01T00:00:00.000Z",
        "updatedAt": "2026-11-01T00:00:00.000Z",
        "user": {
            "id": format!("user-{user}"),
            "ingame_name": user,
            "reputation": 10.0,
            "status": "ingame"
        }
    })
}

pub fn write_market(wfm_url: &str, sell: &[u32], buy: &[u32]) {
    let sell_orders: Vec<Value> = sell
        .iter()
        .enumerate()
        .map(|(index, price)| market_order(&format!("s{index}"), "sell", *price, &format!("seller{index}")))
        .collect();
    let buy_orders: Vec<Value> = buy
        .iter()
        .enumerate()
        .map(|(index, price)| market_order(&format!("b{index}"), "buy", *price, &format!("buyer{index}")))
        .collect();
    let list: OrderList<OrderWithUser> =
        serde_json::from_value(json!({ "sell_orders": sell_orders, "buy_orders": buy_orders }))
            .unwrap();
    let dir = PathBuf::from(qf_app_lib::qf_utils::get_base_path()).join("fake_orders");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("order_{wfm_url}.json")),
        serde_json::to_string(&list).unwrap(),
    )
    .unwrap();
}

pub fn own_order(id: &str, order_type: &str, wfm_id: &str, platinum: u32, quantity: u32) -> Order {
    serde_json::from_value(json!({
        "id": id,
        "type": order_type,
        "platinum": platinum,
        "quantity": quantity,
        "visible": true,
        "itemId": wfm_id,
        "createdAt": "2026-11-01T00:00:00.000Z",
        "updatedAt": "2026-11-01T00:00:00.000Z"
    }))
    .unwrap()
}

pub fn set_own_orders(orders: Vec<Order>) {
    let app = states::app_state().unwrap();
    let mut sell = Vec::new();
    let mut buy = Vec::new();
    for order in orders {
        match order.order_type {
            qf_app_lib::wf_market::enums::OrderType::Sell => sell.push(order),
            qf_app_lib::wf_market::enums::OrderType::Buy => buy.push(order),
        }
    }
    let list: OrderList<Order> =
        serde_json::from_value(json!({ "sell_orders": sell, "buy_orders": buy })).unwrap();
    app.wfm_client.order().set_orders(list);
}
