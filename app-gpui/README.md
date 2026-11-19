# Quantframe GPUI app

The Rust and GPUI front end that replaces the Tauri and React app in `src/`. This crate is its own cargo workspace with its own `Cargo.lock`.

The Rust backend in `src-tauri` (`qf_app_lib`) feeds every screen:

- Listings (Items, Rivens, Wish list and Syndicates tabs), Orders (Orders and Auctions tabs) and Trade messages (Items, Custom and Rivens tabs).
- Home, the dashboard of profit totals, profit charts, the latest transactions and totals per category.
- Chat, the warframe.market conversations.
- Analytics (Transactions with a list and a chart, Items, Rivens, Syndicates, Users and GDPR export tabs). The Items, Rivens, Syndicates and Users tabs read Quantframe's server, so in fixture mode they show the backend's refusal.
- The GDPR export tab of Analytics, which imports the text file of a Warframe GDPR data request and shows an overview, the trades, the market purchases and the real-money payments of each account in it. Each list has a report with totals, the most traded items and a chart per month. Fixture mode loads `src-tauri/fixtures/warframe_gdpr.txt`.
- The detected-trade sheet, which opens when the backend reads a finished trade from Warframe's log that has several items, or one item whose stars or roll the log leaves out. It shows what each side gave, what Quantframe will record and the stock change, and its buttons record the trade, edit the prices or ignore it.
- Inventory (Rivens and Syndicates tabs), read from the game through the source picked in Settings. Fixture mode has no inventory.
- Settings, Debug (State and Warframe log tabs) and About.
- Sign-in, and the states of a suspended account and of an error the app can't work past.

Not built: the update check, the theme editor and custom notification sounds.

## Build and run

Run these inside `nix develop` at the repo root.

```sh
cd app-gpui
CARGO_BUILD_JOBS=5 cargo build --release --locked
target/release/quantframe --fixtures ../src-tauri/fixtures
```

The app starts the backend and shows a loading screen until the backend sends `app:ready`.

- Fixture mode reads `src-tauri/fixtures` into a fresh temp directory and makes no network request. Pick it with `--fixtures <dir>` or `QF_FIXTURES=<dir>`. Debug builds use `src-tauri/fixtures` when neither is set.
- Live mode uses the real app data directory, warframe.market and the Quantframe API. Release builds run in live mode when no fixture directory is named, and debug builds run in live mode with `--live`.
- `QF_STORAGE_DIR=<dir>` replaces the data directory in either mode. In fixture mode the fixtures fill its database only while it has no stock items, so edits made on one run are still there on the next.

Run the data layer test without network:

```sh
CARGO_BUILD_JOBS=5 unshare -rn cargo test --locked --offline
```

## Appearance and the item panel

The app follows the system appearance. The button left of the live scraper button sets it to light or dark, and the choice is saved. The panel button in each toolbar opens the item panel beside the table, and that state is saved too. Both settings live in `interface.conf` under `quantframe` in the user's config directory.

## Keyboard

| Key | Action |
| --- | --- |
| Up, Down, Home, End, Page Up, Page Down | Move the selected row |
| Enter | Open the item panel on the selected row |
| Space | Check or clear the selected row's selection box |
| `/` from the table, or Ctrl+F (Command+F on macOS) | Focus the search box |
| Enter in the search box | Go back to the table |
| Ctrl+I (Command+I on macOS) | Show or hide the item panel |
| Tab, Shift+Tab | Move between controls |

A right-click on a row opens the same menu as the row's overflow button.

## Opening a screen from the environment

These variables open the app in a given state, so screenshots can be scripted.

| Variable | Values |
| --- | --- |
| `QF_SCREEN` | `listings`, `orders`, `trade-messages`, `home`, `chat`, `analytics`, `inventory`, `settings`, `debug`, `about`. `login`, `banned` and `error` show those states with sample text, whatever the account's state is |
| `QF_TAB` | Listings: `items`, `rivens`, `wish-list`, `syndicates`. Orders: `orders`, `auctions`. Trade messages: `item`, `custom`, `riven`. Analytics: `transactions`, `items`, `rivens`, `syndicates`, `users`, `gdpr`. Inventory: `rivens`, `syndicates`. Debug: `state`, `game-log`. Settings: a section, such as `trading`, `buying`, `account` or `logs` (the `key` of each entry in `src/settings/schema.rs`) |
| `QF_VIEW` | `chart` opens the Transactions tab of Analytics on its chart. On the GDPR export tab, `overview`, `trades`, `purchases` and `payments` open that section, `trades-report`, `purchases-report` and `payments-report` open its report, and `import` shows the tab before any file is imported. With `QF_TRADE`, `edit` opens the sheet with its prices editable |
| `QF_TRADE` | `sale`, `purchase`, or both with a comma. It sends a sample trade through the backend's `add_trade` event, so the detected-trade sheet opens without Warframe |
| `QF_THEME` | `system`, `light`, `dark`. It takes precedence over the saved setting and doesn't change it |
| `QF_PANEL` | `open`, `closed`. It takes precedence over the saved setting |
| `QF_ROW` | The index of a row to select once the rows have loaded. On Chat, the conversation to open |
| `QF_DIALOG` | The label of the toolbar button or row menu entry whose prompt to open, such as `Sell` or `Add riven`. On Chat, `send` writes a sample message and sends it |
| `QF_STATE` | `loading`, `error`, `empty`. It shows that table state instead of the loaded rows |
| `QF_WINDOW_CONTROLS` | Any value makes the app draw its own window buttons on Linux when no window manager draws them |

## Screenshots

```sh
bash app-gpui/scripts/render-screens.sh
```

The script runs the app on `src-tauri/fixtures` under Xvfb with Mesa lavapipe and writes one PNG per screen, tab and state to `docs/screenshots/`. Pass an output directory and shot names to render only some of them. `docs/screenshots/variants/` holds the three design variants of the first round.

```sh
bash app-gpui/scripts/check-settings-persist.sh
```

This script checks that a settings change survives a restart. It runs the app twice on one storage directory, switches one setting on with `xdotool`, clicks "Save changes", and fails unless the backend's `settings.json` holds the new value after both runs.

## Settings

The Settings screen edits a draft of the backend's settings and saves the whole draft with `app_update_settings` when you click "Save changes". "Revert" drops the draft. The appearance is the exception: the app saves it at once in `interface.conf`.

`src/settings/schema.rs` is the list of sections and fields. A field names the place of its value in the settings JSON, its label, its hint and its kind, so a new backend setting needs one entry there and no view code.

## Layout

- `src/theme.rs` holds the `Theme` struct with a dark and a light palette. Every colour and size a view uses comes from it, and `Theme::install` maps the same tokens onto the gpui-component theme.
- `src/prefs.rs` reads and writes the saved appearance and item panel settings.
- `src/model/` holds view models whose fields mirror the backend types in `src-tauri/entity` and `src/types/tauri.type.ts`.
- `src/data/` holds `DataSource`, whose methods are named after the Tauri commands, and `Backend`, the gpui entity that re-emits backend events. `src/lib.rs` exports `data` and `model` so `tests/` can use them.
- `src/shell/` holds the sidebar, title bar, live scraper control and status bar.
- `src/ui/` holds what every list screen shares. `grid.rs` has the `Catalog` trait and the table. `page.rs` loads a catalog through the backend and draws its toolbar, totals, filter chips, table and item panel. `screen.rs` puts pages behind tabs. `form.rs` builds the dialogs from a list of fields.
- `src/listings/`, `src/orders/`, `src/trade_messages/`, `src/inventory/` and the price and transaction tabs of `src/analytics/` hold one `Catalog` per tab: its columns, row actions, dialogs and backend calls.
- `src/gdpr/` holds the GDPR export tab: the view with its sections and file import, one `Catalog` per list, the overview and the report. `src/detected_trade/` holds the trade sheet and its sample trades.
- `src/home/`, `src/chat/`, `src/settings/`, `src/about/` and `src/auth/` hold the screens that are not tables. `src/debug/` holds a state view and the Warframe log table.
- `src/kit.rs` holds what those screens share: cards, facts, link rows and the loading and error states. `src/chart.rs` draws grouped bars. The line charts are gpui-component's `LineChart`.

## Dependencies

| Crate | Version | Note |
| --- | --- | --- |
| `gpui-pre` (imported as `gpui`) | `=0.3.12` | A crates.io snapshot of Zed's `gpui` that the gpui-component maintainer publishes. gpui-component pins this exact version. |
| `gpui-pre-platform` (imported as `gpui_platform`) | `=0.3.12` | Provides `application()` for X11, Wayland, macOS and Windows. |
| `gpui-component` | `=0.10.0` | Table, input, select, switch, button, checkbox, menu, dialog, notification, tooltip, title bar, line chart. |
| `gpui-kit-assets` | `=0.10.0` | Lucide icons. `src/assets.rs` embeds only the icons it lists. |

Upgrade the four together, because gpui-component pins `gpui-pre` to one exact version.

## Bundled files

- `assets/fonts/Geist*.ttf`, SIL Open Font License 1.1, see `assets/fonts/OFL-Geist.txt`.
- Lucide icons come from `gpui-kit-assets` under the ISC license.
- The app icon is `src-tauri/icons/128x128.png`, embedded at build time.
