//! Interface settings that outlive a session: the appearance setting and
//! whether the item panel is open. They live in a `key=value` file in the
//! user's config directory until the backend's settings store takes them.

use std::{fs, path::PathBuf};

use crate::theme::AppearanceChoice;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Prefs {
    pub appearance: AppearanceChoice,
    pub item_panel_open: bool,
}

const FILE_NAME: &str = "interface.conf";

fn path() -> Option<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(var("HOME")?).join("Library/Application Support")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(var("APPDATA")?)
    } else {
        match var("XDG_CONFIG_HOME") {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from(var("HOME")?).join(".config"),
        }
    };
    Some(base.join("quantframe").join(FILE_NAME))
}

pub fn load() -> Prefs {
    let mut prefs = Prefs::default();
    let Some(text) = path().and_then(|path| fs::read_to_string(path).ok()) else {
        return prefs;
    };
    for line in text.lines() {
        match line.split_once('=') {
            Some(("appearance", value)) => {
                prefs.appearance = AppearanceChoice::parse(value).unwrap_or_default();
            }
            Some(("item_panel", value)) => prefs.item_panel_open = value.trim() == "open",
            _ => {}
        }
    }
    prefs
}

/// Changes the saved settings. A failed write keeps the change for this
/// session only, so the error goes to stderr and the app carries on.
pub fn update(change: impl FnOnce(&mut Prefs)) {
    let mut prefs = load();
    change(&mut prefs);
    let Some(path) = path() else {
        return;
    };
    let text = format!(
        "appearance={}\nitem_panel={}\n",
        prefs.appearance.as_str(),
        if prefs.item_panel_open {
            "open"
        } else {
            "closed"
        },
    );
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&path, text));
    if let Err(error) = written {
        eprintln!("Could not save {}: {error}", path.display());
    }
}
