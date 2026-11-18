use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use futures::channel::mpsc::UnboundedSender;
use qf_app_lib::Host;
use serde_json::Value;

use super::events::BackendEvent;

pub struct GpuiHost {
    events: UnboundedSender<BackendEvent>,
    pub(super) export_path: Mutex<Option<PathBuf>>,
}

impl GpuiHost {
    pub fn new(events: UnboundedSender<BackendEvent>) -> Self {
        Self {
            events,
            export_path: Mutex::new(None),
        }
    }

    pub fn send(&self, event: BackendEvent) {
        let _ = self.events.unbounded_send(event);
    }
}

impl Host for GpuiHost {
    fn emit(&self, event: &str, payload: Value) {
        self.send(BackendEvent::parse(event, &payload));
    }

    fn notify(&self, title: &str, body: &str) {
        self.send(BackendEvent::SystemNotification {
            title: title.to_owned(),
            body: body.to_owned(),
        });
    }

    fn play_sound(&self, file: &str, volume: f32) {
        self.send(BackendEvent::PlaySound {
            file: file.to_owned(),
            volume,
        });
    }

    fn save_file(&self, _: &str, _: &[&str], _: Option<&Path>) -> Option<PathBuf> {
        self.export_path.lock().ok()?.take()
    }

    fn show_trade(&self, payload: Value) {
        self.emit("add_trade", payload);
    }
}
