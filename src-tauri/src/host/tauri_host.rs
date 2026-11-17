use std::path::{Path, PathBuf};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_notification::NotificationExt;
use utils::{error, LoggerOptions};

use super::Host;

pub struct TauriHost(pub AppHandle);

impl Host for TauriHost {
    fn emit(&self, event: &str, payload: Value) {
        if let Err(e) = self.0.emit(event, payload) {
            error(
                "Host:Emit",
                &format!("Event {event} error: {e}"),
                &LoggerOptions::default(),
            );
        }
    }

    fn notify(&self, title: &str, body: &str) {
        self.0
            .notification()
            .builder()
            .title(title)
            .body(body)
            .icon("assets/icons/icon.png")
            .sound("Ping")
            .show()
            .expect("Failed to show notification");
    }

    fn save_file(
        &self,
        filter_name: &str,
        extensions: &[&str],
        suggested: Option<&Path>,
    ) -> Option<PathBuf> {
        let mut dialog = self.0.dialog().file();
        if let Some(path) = suggested {
            if let Some(name) = path.file_name() {
                dialog = dialog.set_file_name(name.to_string_lossy());
            }
            if let Some(dir) = path.parent() {
                dialog = dialog.set_directory(dir);
            }
        }
        dialog
            .add_filter(filter_name, extensions)
            .blocking_save_file()
            .and_then(|path| path.into_path().ok())
    }

    fn pick_file(&self, title: &str, filter_name: &str, extensions: &[&str]) -> Option<PathBuf> {
        self.0
            .dialog()
            .file()
            .set_title(title)
            .add_filter(filter_name, extensions)
            .blocking_pick_file()
            .and_then(|path| path.into_path().ok())
    }

    fn show_trade(&self, payload: Value) {
        let label = "processing-trades";
        let (is_open, window) = match self.0.get_webview_window(label) {
            Some(window) => (true, window),
            None => {
                let built = WebviewWindowBuilder::new(
                    &self.0,
                    label,
                    WebviewUrl::App("clean?type=process_trade".into()),
                )
                .title("Processing Trades")
                .resizable(true)
                .inner_size(800.0, 600.0)
                .build();
                match built {
                    Ok(window) => (false, window),
                    Err(e) => {
                        error(
                            "Host:ShowTrade",
                            &format!("Failed to build window: {e}"),
                            &LoggerOptions::default(),
                        );
                        return;
                    }
                }
            }
        };

        let window_clone = window.clone();
        let payload_clone = payload.clone();
        window.once("initialize", move |_| {
            let _ = window_clone.emit("add_trade", payload_clone);
        });
        if is_open {
            let _ = window.emit("add_trade", payload);
        }
    }

    fn exit(&self, code: i32) {
        self.0.exit(code);
    }
}
