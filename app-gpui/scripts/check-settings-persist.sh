#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
binary="$root/app-gpui/target/release/quantframe"
out="${1:-$root/docs/screenshots}"
mkdir -p "$out"

storage="$(mktemp -d)"
config="$(mktemp -d)"
trap 'rm -rf "$storage" "$config"' EXIT

switch="1200 496"
save="1375 846"

run() {
  local shot="$1" clicks="$2"
  xvfb-run -a -s '-screen 0 1440x900x24' bash -c "
    env QF_FIXTURES='$root/src-tauri/fixtures' QF_STORAGE_DIR='$storage' \\
      QF_SCREEN=settings QF_TAB=trading QF_THEME=dark QF_WINDOW_CONTROLS=1 \\
      XDG_CONFIG_HOME='$config' '$binary' >/dev/null 2>&1 &
    app=\$!
    sleep 5
    $clicks
    xdotool mousemove 700 860
    sleep 1
    import -window root '$out/$shot.png'
    kill \$app
  "
}

stored() {
  local file
  file="$(find "$storage" -name settings.json | head -n 1)"
  [[ -n "$file" ]] || { echo "no settings.json under $storage" >&2; return 1; }
  python3 -c '
import json, sys
print(json.load(open(sys.argv[1]))["live_scraper"]["general"]["delete_conflicting_orders"])
' "$file"
}

run settings-persist-1-saved "
    xdotool mousemove $switch click 1
    sleep 0.5
    xdotool mousemove $save click 1
    sleep 1.5
"
first="$(stored)"
run settings-persist-2-restarted ""
second="$(stored)"

echo "after saving: delete_conflicting_orders=$first"
echo "after restart: delete_conflicting_orders=$second"
[[ "$first" == True && "$second" == True ]] || { echo "FAIL: the change did not persist" >&2; exit 1; }
echo "PASS: $out/settings-persist-1-saved.png $out/settings-persist-2-restarted.png"
