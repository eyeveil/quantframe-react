# Quantframe GPUI app

The Rust and GPUI front end that replaces the Tauri and React app in `src/`. This crate is its own cargo workspace with its own `Cargo.lock`.

Built so far: the app shell and the listings screen (live scraper Item tab), fed by fixture data. The other screens show a "not built yet" line.

## Build and run

Run these inside `nix develop` at the repo root.

```sh
cd app-gpui
CARGO_BUILD_JOBS=5 cargo build --release --locked
QF_VARIANT=a target/release/quantframe
```

`QF_VARIANT` selects the design variant: `a`, `b`, or `c`. Any other value selects `a`.

## Screenshots

```sh
bash app-gpui/scripts/render-variants.sh
```

The script runs each variant under Xvfb with Mesa lavapipe and writes `docs/screenshots/variants/listings-{a,b,c}.png`. Xvfb has no window manager, so the script sets `QF_WINDOW_CONTROLS=1`, which makes the app draw its own minimise, maximise and close buttons.

## Layout

- `src/theme.rs` holds the `Theme` struct. Every colour, size and layout choice a view uses comes from it, and `Theme::install` maps the same tokens onto the gpui-component theme.
- `src/model/` holds view models whose fields mirror the backend types in `src-tauri/entity` and `src/types/tauri.type.ts`.
- `src/data/` holds the `DataSource` trait, named after the Tauri commands, and `FixtureSource`.
- `src/shell/` holds the navigation, title bar, live scraper control and status bar.
- `src/listings/` holds the listings screen, its table delegate and the item panel of variant B.

## Dependencies

| Crate | Version | Note |
| --- | --- | --- |
| `gpui-pre` (imported as `gpui`) | `=0.3.12` | A crates.io snapshot of Zed's `gpui` that the gpui-component maintainer publishes. gpui-component pins this exact version. |
| `gpui-pre-platform` (imported as `gpui_platform`) | `=0.3.12` | Provides `application()` for X11, Wayland, macOS and Windows. |
| `gpui-component` | `=0.10.0` | Table, input, button, checkbox, tooltip, title bar. |
| `gpui-kit-assets` | `=0.10.0` | Lucide icons. `src/assets.rs` embeds only the icons it lists. |

Upgrade the four together, because gpui-component pins `gpui-pre` to one exact version.

## Bundled files

- `assets/fonts/Geist*.ttf`, SIL Open Font License 1.1, see `assets/fonts/OFL-Geist.txt`.
- `assets/fonts/IBMPlex*.ttf`, SIL Open Font License 1.1, see `assets/fonts/OFL-IBM-Plex.txt`.
- Lucide icons come from `gpui-kit-assets` under the ISC license.
- The app icon is `src-tauri/icons/128x128.png`, embedded at build time.
