use std::time::{Duration, Instant};

use quantframe_gpui::notify::{desktop, sound};

#[test]
fn sounds_and_notifications_return_without_an_audio_device_or_a_notification_daemon() {
    let dir = std::env::temp_dir().join(format!("qf-no-devices-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let alsa_config = dir.join("asound.conf");
    std::fs::write(&alsa_config, "").unwrap();
    unsafe {
        std::env::set_var("XDG_DATA_HOME", &dir);
        std::env::set_var("ALSA_CONFIG_PATH", &alsa_config);
        std::env::set_var("PULSE_SERVER", dir.join("no-pulse"));
        std::env::set_var(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={}", dir.join("no-bus").display()),
        );
    }

    let started = Instant::now();
    let played = sound::play_now("cat_meow.mp3", 0.5);
    let custom = sound::play_now("custom:missing.mp3", 1.0);
    assert!(sound::play_now("none", 1.0).is_ok());
    let shown = desktop::show_now("Quantframe test", "No daemon is listening");
    assert!(
        shown.is_err(),
        "a notification reached a bus that does not exist"
    );

    sound::play("windows_xp_error.mp3", 1.0);
    desktop::show("Quantframe test", "Queued with no daemon");
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the calls took {:?}",
        started.elapsed()
    );
    eprintln!("bundled: {played:?}, custom: {custom:?}, notification: {shown:?}");
    std::thread::sleep(Duration::from_millis(500));
    let _ = std::fs::remove_dir_all(&dir);
}
