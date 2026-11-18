use std::path::PathBuf;

use futures::{StreamExt, executor::block_on};
use quantframe_gpui::data::{BackendEvent, DataSource, StartOptions};

#[test]
fn fixture_mode_refuses_a_storage_dir_from_a_real_run() {
    let storage = std::env::temp_dir().join(format!("qf-guard-{}", std::process::id()));
    std::fs::create_dir_all(&storage).unwrap();
    std::fs::write(storage.join("auth.json"), "{\"real\":true}").unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/fixtures");
    let (_source, mut events) = DataSource::start(StartOptions {
        fixtures: Some(fixtures),
        storage_dir: Some(storage.clone()),
    })
    .expect("start");
    let failure = block_on(async {
        loop {
            match events.next().await.expect("backend event channel closed") {
                BackendEvent::StartupFailed(reason) => break reason,
                BackendEvent::Ready => panic!("fixture mode started on a real storage dir"),
                _ => {}
            }
        }
    });
    assert!(failure.contains("holds data from a real run"), "{failure}");
    assert_eq!(
        std::fs::read_to_string(storage.join("auth.json")).unwrap(),
        "{\"real\":true}"
    );
    assert!(!storage.join("cache").exists());
    std::fs::remove_dir_all(&storage).ok();
}
