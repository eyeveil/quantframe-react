#!/usr/bin/env bash
# Renders every built screen under Xvfb and saves one PNG per shot.
# Run inside `nix develop` from the repo root, after `cargo build --release --locked`
# in app-gpui.
#
#   bash app-gpui/scripts/render-screens.sh [output dir] [shot name ...]
#
# With no shot names it renders them all. The app runs the backend on
# src-tauri/fixtures, which it copies to a fresh temporary directory, so a
# render makes no network request and changes no data. Xvfb has no window
# manager, so QF_WINDOW_CONTROLS makes the app draw its own window buttons.
# The pointer starts at the screen centre, over the table, so it steps down
# to the status bar before the capture and no row shows its hover colour. One
# long jump leaves the hover colour in place.
# The app reads its settings from a temporary XDG_CONFIG_HOME, so a render
# never changes the user's saved appearance or item panel setting.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
binary="$root/app-gpui/target/release/quantframe"
out="${1:-$root/docs/screenshots}"
shift || true
mkdir -p "$out"

# Each shot is a file name, then the environment the app starts with.
shots=(
  "listings-items-dark|QF_SCREEN=listings QF_TAB=items QF_THEME=dark"
  "listings-items-light|QF_SCREEN=listings QF_TAB=items QF_THEME=light"
  "listings-items-panel|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_PANEL=open QF_ROW=0"
  "listings-items-panel-light|QF_SCREEN=listings QF_TAB=items QF_THEME=light QF_PANEL=open QF_ROW=0"
  "listings-items-sell-dialog|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_ROW=7 QF_DIALOG=Sell"
  "listings-items-edit-dialog|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_ROW=0 QF_DIALOG=Edit"
  "listings-items-loading|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_STATE=loading"
  "listings-items-error|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_STATE=error"
  "listings-items-empty|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_STATE=empty"
  "listings-rivens|QF_SCREEN=listings QF_TAB=rivens QF_THEME=dark"
  "listings-rivens-add-dialog|QF_SCREEN=listings QF_TAB=rivens QF_THEME=dark QF_DIALOG='Add riven'"
  "listings-wish-list|QF_SCREEN=listings QF_TAB=wish-list QF_THEME=dark"
  "listings-syndicates|QF_SCREEN=listings QF_TAB=syndicates QF_THEME=dark"
  "orders-orders|QF_SCREEN=orders QF_TAB=orders QF_THEME=dark"
  "orders-auctions|QF_SCREEN=orders QF_TAB=auctions QF_THEME=dark"
  "orders-auctions-panel|QF_SCREEN=orders QF_TAB=auctions QF_THEME=dark QF_PANEL=open QF_ROW=1"
  "trade-messages-items|QF_SCREEN=trade-messages QF_TAB=item QF_THEME=dark"
  "trade-messages-custom|QF_SCREEN=trade-messages QF_TAB=custom QF_THEME=dark"
  "trade-messages-rivens|QF_SCREEN=trade-messages QF_TAB=riven QF_THEME=dark"
  "trade-messages-message-dialog|QF_SCREEN=trade-messages QF_TAB=item QF_THEME=dark QF_DIALOG='Write a trade chat message'"
)

config="$(mktemp -d)"
trap 'rm -rf "$config"' EXIT

for shot in "${shots[@]}"; do
  name="${shot%%|*}"
  settings="${shot#*|}"
  if (($# > 0)) && [[ " $* " != *" $name "* ]]; then
    continue
  fi
  xvfb-run -a -s '-screen 0 1440x900x24' bash -c "
    env QF_FIXTURES='$root/src-tauri/fixtures' QF_PANEL=closed $settings QF_WINDOW_CONTROLS=1 \\
      XDG_CONFIG_HOME='$config' '$binary' &
    app=\$!
    sleep 5
    for step in 1 2 3 4 5 6; do
      xdotool mousemove_relative -- 30 72
      sleep 0.15
    done
    sleep 1
    import -window root '$out/$name.png'
    kill \$app
  "
  echo "$out/$name.png"
done
