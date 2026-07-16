#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
EVIDENCE="$ROOT/docs/verification/control-surface-audit"
MODERN="$EVIDENCE/modern"
LOGS="$EVIDENCE/logs"
FAILURES="$EVIDENCE/failures"
mkdir -p "$MODERN" "$LOGS" "$FAILURES"
export DISPLAY="${DISPLAY:-:99}"
if ! xdpyinfo -display "$DISPLAY" >/dev/null 2>&1; then
  Xvfb "$DISPLAY" -screen 0 "${SCREEN:-1920x1080x24}" -ac -nolisten tcp >"$LOGS/xvfb.log" 2>&1 &
  XVFB_PID=$!
  sleep 1
else XVFB_PID=""; fi
openbox-session >"$LOGS/openbox.log" 2>&1 &
OPENBOX_PID=$!
cleanup(){ [ -n "${APP_PID:-}" ] && kill "$APP_PID" 2>/dev/null || true; kill "$OPENBOX_PID" 2>/dev/null || true; [ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null || true; }
trap cleanup EXIT INT TERM
cd "$ROOT"
dotnet build modern/desktop/src/Darwinbots.Desktop/Darwinbots.Desktop.csproj >"$LOGS/dotnet-build-from-audit.log" 2>&1
dotnet modern/desktop/src/Darwinbots.Desktop/bin/Debug/net10.0/Darwinbots.Desktop.dll >"$LOGS/modern-app.log" 2>&1 &
APP_PID=$!
for i in {1..60}; do wmctrl -l | tee "$LOGS/windows-$i.log" | grep -q "Darwinbots Modern" && break; sleep 1; done
WIN=$(wmctrl -l | awk '/Darwinbots Modern/{print $1; exit}')
wmctrl -ia "$WIN"; sleep 1
import -window root "$MODERN/setup-initial-modern-001.png"
# Open menus and dialogs via keyboard/mouse visible interaction.
xdotool key alt+f; sleep .5; import -window root "$MODERN/setup-menu-file-open-modern-002.png"
xdotool key Escape alt+v; sleep .5; import -window root "$MODERN/setup-menu-view-open-modern-003.png"
xdotool key Escape alt+h; sleep .5; import -window root "$MODERN/setup-menu-help-open-modern-004.png"
xdotool key Escape ctrl+comma; sleep 1; import -window root "$MODERN/setup-advanced-open-modern-005.png"; xdotool key Escape || true; sleep .5
# Exercise setup fields and controls by coordinate-safe clicks based on rendered layout.
xdotool click 1; sleep .2
xdotool mousemove 640 190 click 1; sleep .3; import -window root "$MODERN/setup-mode-zerobots-modern-006.png"
xdotool mousemove 920 190 click 1; sleep .3; import -window root "$MODERN/setup-mode-zerobots-vegetables-modern-007.png"
xdotool mousemove 365 190 click 1; sleep .3
# mutation slider approximate min/mid/max
xdotool mousemove 760 356 mousedown 1 mousemove 650 356 mouseup 1; sleep .2; import -window root "$MODERN/setup-mutation-min-modern-008.png"
xdotool mousemove 650 356 mousedown 1 mousemove 735 356 mouseup 1; sleep .2; import -window root "$MODERN/setup-mutation-mid-modern-009.png"
xdotool mousemove 735 356 mousedown 1 mousemove 815 356 mouseup 1; sleep .2; import -window root "$MODERN/setup-mutation-max-modern-010.png"
# Change combos and numeric fields.
xdotool mousemove 1130 126 click 1; sleep .2; xdotool key Down Return; sleep .2; import -window root "$MODERN/setup-backend-combo-modern-011.png"
xdotool mousemove 1170 500 click 1; sleep .2; import -window root "$MODERN/setup-create-world-before-modern-012.png"
xdotool mousemove 1170 660 click 1; sleep 3; import -window root "$MODERN/live-initial-modern-013.png"
# Live toolbar and panels.
xdotool key alt+space Escape || true
xdotool mousemove 280 30 click 1; sleep .5; import -window root "$MODERN/live-run-modern-014.png"
xdotool mousemove 380 30 click 1; sleep .5; import -window root "$MODERN/live-pause-modern-015.png"
xdotool mousemove 470 30 click 1; sleep .5; import -window root "$MODERN/live-step-modern-016.png"
xdotool mousemove 555 30 click 1; sleep .5; import -window root "$MODERN/live-turbo-modern-017.png"
xdotool mousemove 89 258 click 1; sleep .3; import -window root "$MODERN/live-toroidal-toggle-modern-018.png"
xdotool mousemove 91 282 click 1; sleep .3; import -window root "$MODERN/live-render-waste-toggle-modern-019.png"
xdotool mousemove 87 316 click 1; sleep .3; import -window root "$MODERN/live-add-obstacle-modern-020.png"
xdotool mousemove 90 348 click 1; sleep .3; import -window root "$MODERN/live-add-teleporter-modern-021.png"
xdotool mousemove 91 380 click 1; sleep .3; import -window root "$MODERN/live-remove-feature-modern-022.png"
xdotool mousemove 111 409 click 1; sleep .3; import -window root "$MODERN/live-open-physics-modern-023.png"; xdotool key Escape || true
xdotool mousemove 112 498 click 1; sleep .3; import -window root "$MODERN/live-apply-energy-modern-024.png"
xdotool mousemove 505 290 click 1; sleep .3; import -window root "$MODERN/live-viewport-select-modern-025.png"
xdotool mousemove 505 290 mousedown 1 mousemove 650 360 mouseup 1; sleep .5; import -window root "$MODERN/live-viewport-drag-modern-026.png"
xdotool mousemove 830 255 click 1; sleep .5; import -window root "$MODERN/live-dna-editor-modern-027.png"; xdotool key Escape || true
printf 'Rendered GUI audit completed at %s\n' "$(date -u +%FT%TZ)" > "$LOGS/rendered-modern-audit-complete.log"
