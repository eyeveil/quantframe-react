#!/usr/bin/env bash
# Xvfb has no window manager, so QF_WINDOW_CONTROLS makes the app draw its own
# window buttons. One long pointer jump leaves a row's hover colour in place, so
# the pointer steps down to the status bar before the capture.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
binary="$root/app-gpui/target/release/quantframe"
out="${1:-$root/docs/screenshots}"
shift || true
mkdir -p "$out"

shots=(
  "listings-items-dark|QF_SCREEN=listings QF_TAB=items QF_THEME=dark"
  "listings-items-light|QF_SCREEN=listings QF_TAB=items QF_THEME=light"
  "listings-items-panel|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_PANEL=open QF_ROW=0"
  "listings-items-panel-light|QF_SCREEN=listings QF_TAB=items QF_THEME=light QF_PANEL=open QF_ROW=0"
  "listings-items-sell-dialog|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_ROW=7 QF_DIALOG=Sell"
  "listings-items-edit-dialog|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_ROW=1 QF_DIALOG=Edit"
  "listings-items-add-dialog|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_DIALOG='Add item'"
  "listings-items-loading|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_STATE=loading"
  "listings-items-error|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_STATE=error"
  "listings-items-empty|QF_SCREEN=listings QF_TAB=items QF_THEME=dark QF_STATE=empty"
  "listings-rivens|QF_SCREEN=listings QF_TAB=rivens QF_THEME=dark"
  "listings-rivens-panel|QF_SCREEN=listings QF_TAB=rivens QF_THEME=dark QF_PANEL=open QF_ROW=0"
  "listings-rivens-add-dialog|QF_SCREEN=listings QF_TAB=rivens QF_THEME=dark QF_DIALOG='Add riven'"
  "listings-rivens-filter-dialog|QF_SCREEN=listings QF_TAB=rivens QF_THEME=dark QF_ROW=0 QF_DIALOG='Edit the auction filter'"
  "listings-wish-list|QF_SCREEN=listings QF_TAB=wish-list QF_THEME=dark"
  "listings-wish-list-edit-dialog|QF_SCREEN=listings QF_TAB=wish-list QF_THEME=dark QF_ROW=0 QF_DIALOG=Edit"
  "listings-syndicates|QF_SCREEN=listings QF_TAB=syndicates QF_THEME=dark"
  "orders-orders|QF_SCREEN=orders QF_TAB=orders QF_THEME=dark"
  "orders-orders-panel|QF_SCREEN=orders QF_TAB=orders QF_THEME=dark QF_PANEL=open QF_ROW=0"
  "orders-auctions|QF_SCREEN=orders QF_TAB=auctions QF_THEME=dark"
  "orders-auctions-panel|QF_SCREEN=orders QF_TAB=auctions QF_THEME=dark QF_PANEL=open QF_ROW=1"
  "trade-messages-items|QF_SCREEN=trade-messages QF_TAB=item QF_THEME=dark"
  "trade-messages-custom|QF_SCREEN=trade-messages QF_TAB=custom QF_THEME=dark"
  "trade-messages-rivens|QF_SCREEN=trade-messages QF_TAB=riven QF_THEME=dark"
  "trade-messages-rivens-panel|QF_SCREEN=trade-messages QF_TAB=riven QF_THEME=dark QF_PANEL=open QF_ROW=0"
  "trade-messages-message-dialog|QF_SCREEN=trade-messages QF_TAB=item QF_THEME=dark QF_DIALOG='Write a trade chat message'"
  "home-dark|QF_SCREEN=home QF_THEME=dark"
  "home-light|QF_SCREEN=home QF_THEME=light"
  "chat|QF_SCREEN=chat QF_THEME=dark"
  "chat-send-error|QF_SCREEN=chat QF_THEME=dark QF_DIALOG=send"
  "analytics-transactions|QF_SCREEN=analytics QF_TAB=transactions QF_THEME=dark"
  "analytics-transactions-panel|QF_SCREEN=analytics QF_TAB=transactions QF_THEME=dark QF_PANEL=open QF_ROW=0"
  "analytics-transactions-chart|QF_SCREEN=analytics QF_TAB=transactions QF_VIEW=chart QF_THEME=dark"
  "analytics-items|QF_SCREEN=analytics QF_TAB=items QF_THEME=dark"
  "analytics-rivens|QF_SCREEN=analytics QF_TAB=rivens QF_THEME=dark"
  "analytics-syndicates|QF_SCREEN=analytics QF_TAB=syndicates QF_THEME=dark"
  "analytics-users|QF_SCREEN=analytics QF_TAB=users QF_THEME=dark"
  "inventory-rivens|QF_SCREEN=inventory QF_TAB=rivens QF_THEME=dark"
  "inventory-syndicates|QF_SCREEN=inventory QF_TAB=syndicates QF_THEME=dark"
  "settings-general|QF_SCREEN=settings QF_TAB=general QF_THEME=dark"
  "settings-account|QF_SCREEN=settings QF_TAB=account QF_THEME=dark"
  "settings-appearance|QF_SCREEN=settings QF_TAB=appearance QF_THEME=dark"
  "settings-trading|QF_SCREEN=settings QF_TAB=trading QF_THEME=dark"
  "settings-buying|QF_SCREEN=settings QF_TAB=buying QF_THEME=dark"
  "settings-selling|QF_SCREEN=settings QF_TAB=selling QF_THEME=dark"
  "settings-rivens|QF_SCREEN=settings QF_TAB=rivens QF_THEME=dark"
  "settings-syndicates|QF_SCREEN=settings QF_TAB=syndicates QF_THEME=dark"
  "settings-new-conversation|QF_SCREEN=settings QF_TAB=new-conversation QF_THEME=dark"
  "settings-chat-message|QF_SCREEN=settings QF_TAB=chat-message QF_THEME=dark"
  "settings-trade|QF_SCREEN=settings QF_TAB=trade QF_THEME=dark"
  "settings-inventory|QF_SCREEN=settings QF_TAB=inventory QF_THEME=dark"
  "settings-logs|QF_SCREEN=settings QF_TAB=logs QF_THEME=dark"
  "settings-http-server|QF_SCREEN=settings QF_TAB=http-server QF_THEME=dark"
  "settings-debugging|QF_SCREEN=settings QF_TAB=debugging QF_THEME=dark"
  "settings-trading-light|QF_SCREEN=settings QF_TAB=trading QF_THEME=light"
  "about|QF_SCREEN=about QF_THEME=dark"
  "debug-state|QF_SCREEN=debug QF_TAB=state QF_THEME=dark"
  "debug-game-log|QF_SCREEN=debug QF_TAB=game-log QF_THEME=dark"
  "login|QF_SCREEN=login QF_THEME=dark"
  "banned|QF_SCREEN=banned QF_THEME=dark"
  "error|QF_SCREEN=error QF_THEME=dark"
)

config="$(mktemp -d)"
trap 'rm -rf "$config"' EXIT

for shot in "${shots[@]}"; do
  name="${shot%%|*}"
  settings="${shot#*|}"
  if (($# > 0)) && [[ " $* " != *" $name "* ]]; then
    continue
  fi
  timeout -k 5 45 xvfb-run -a -s '-screen 0 1440x900x24' bash -c "
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
    kill \$app; sleep 1; kill -9 \$app 2>/dev/null || true
  " || echo "render timed out: $name" >&2
  echo "$out/$name.png"
done
