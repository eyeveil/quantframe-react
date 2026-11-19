use std::{
    io::Cursor,
    path::{Component, Path, PathBuf},
};

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

use super::Worker;

pub const CUSTOM_PREFIX: &str = "custom:";
pub const FALLBACK: &str = "cat_meow.mp3";

const BUNDLED: &[(&str, &[u8])] = &[
    (
        "cat_meow.mp3",
        include_bytes!("../../../src-tauri/resources/sounds/cat_meow.mp3"),
    ),
    (
        "iphone_notification.mp3",
        include_bytes!("../../../src-tauri/resources/sounds/iphone_notification.mp3"),
    ),
    (
        "windows_notification.mp3",
        include_bytes!("../../../src-tauri/resources/sounds/windows_notification.mp3"),
    ),
    (
        "windows_xp_error.mp3",
        include_bytes!("../../../src-tauri/resources/sounds/windows_xp_error.mp3"),
    ),
    (
        "windows_xp_startup.mp3",
        include_bytes!("../../../src-tauri/resources/sounds/windows_xp_startup.mp3"),
    ),
];

struct Request {
    file: String,
    volume: f32,
}

static WORKER: Worker<Request, Option<MixerDeviceSink>> = Worker::new("sound", deliver);

pub fn is_silent(file: &str) -> bool {
    file.is_empty() || file == "none"
}

pub fn play(file: &str, volume: f32) {
    if is_silent(file) {
        return;
    }
    WORKER.submit(Request {
        file: file.to_owned(),
        volume,
    });
}

pub fn play_now(file: &str, volume: f32) -> Result<(), String> {
    deliver(
        &mut None,
        Request {
            file: file.to_owned(),
            volume,
        },
    )
}

fn deliver(sink: &mut Option<MixerDeviceSink>, request: Request) -> Result<(), String> {
    if is_silent(&request.file) {
        return Ok(());
    }
    let bytes = match load(&request.file) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("sound: {error}, playing {FALLBACK} instead");
            load(FALLBACK)?
        }
    };
    let decoder = match Decoder::new(Cursor::new(bytes)) {
        Ok(decoder) => decoder,
        Err(error) => {
            eprintln!(
                "sound: could not decode {}: {error}, playing {FALLBACK} instead",
                request.file
            );
            Decoder::new(Cursor::new(load(FALLBACK)?)).map_err(|error| error.to_string())?
        }
    };
    if sink.is_none() {
        let mut opened = DeviceSinkBuilder::open_default_sink()
            .map_err(|error| format!("no audio output, {} not played: {error}", request.file))?;
        opened.log_on_drop(false);
        *sink = Some(opened);
    }
    let Some(sink) = sink.as_ref() else {
        return Ok(());
    };
    let player = Player::connect_new(sink.mixer());
    player.set_volume(request.volume.clamp(0.0, 1.0));
    player.append(decoder);
    player.detach();
    Ok(())
}

pub fn custom_sounds_dir() -> Option<PathBuf> {
    std::panic::catch_unwind(qf_app_lib::helper::get_sounds_path).ok()
}

fn load(file: &str) -> Result<Vec<u8>, String> {
    if let Some(name) = file.strip_prefix(CUSTOM_PREFIX) {
        let mut parts = Path::new(name).components();
        if !matches!(
            (parts.next(), parts.next()),
            (Some(Component::Normal(_)), None)
        ) {
            return Err(format!("{file} is not a file name"));
        }
        let dir = custom_sounds_dir().ok_or("the custom sounds folder is unavailable")?;
        let path = dir.join(name);
        return std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()));
    }
    BUNDLED
        .iter()
        .find(|(name, _)| *name == file)
        .map(|(_, bytes)| bytes.to_vec())
        .ok_or_else(|| format!("{file} is not a bundled sound"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_sound_decodes() {
        for (name, bytes) in BUNDLED {
            assert!(
                Decoder::new(Cursor::new(bytes.to_vec())).is_ok(),
                "{name} does not decode"
            );
        }
    }

    #[test]
    fn custom_names_cannot_leave_the_sounds_folder() {
        assert!(load("custom:../settings.json").is_err());
        assert!(load("custom:/etc/passwd").is_err());
    }
}
