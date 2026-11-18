#!/usr/bin/env bash
# Renders each design variant under Xvfb and saves a screenshot of the screen.
# Run inside `nix develop` from the repo root, after `cargo build --release --locked`
# in app-gpui. The app reads src-tauri/fixtures. Xvfb has no window manager, so QF_WINDOW_CONTROLS makes the app
# draw its own window buttons. The pointer starts over the table, so it steps
# down to the status bar before the capture and no row shows its hover colour.
# One long jump leaves the hover colour in place.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
out="${1:-$root/docs/screenshots/variants}"
mkdir -p "$out"

for variant in a b c; do
  xvfb-run -a -s '-screen 0 1440x900x24' bash -c "
    QF_FIXTURES='$root/src-tauri/fixtures' QF_VARIANT=$variant QF_WINDOW_CONTROLS=1 '$root/app-gpui/target/release/quantframe' &
    app=\$!
    sleep 4
    for step in 1 2 3 4 5 6; do
      xdotool mousemove_relative -- 30 72
      sleep 0.15
    done
    sleep 1
    import -window root '$out/listings-$variant.png'
    kill \$app
  "
  echo "$out/listings-$variant.png"
done
