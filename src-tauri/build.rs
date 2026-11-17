fn main() {
    #[cfg(feature = "tauri")]
    tauri_build::build();
    #[cfg(not(feature = "tauri"))]
    println!("cargo::rustc-check-cfg=cfg(dev)");
}
