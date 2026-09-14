#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
if [[ $(id -u) == 0 ]]; then
    echo 'Run this installer as your desktop user, not root.' >&2
    exit 1
fi
startup=on
case "${1:-}" in
    '') ;;
    --no-autostart) startup=off ;;
    *) echo "Usage: $0 [--no-autostart]" >&2; exit 2 ;;
esac
if [[ -n ${VITRILYR_BINARY:-} ]]; then
    binary=$VITRILYR_BINARY
else
    "$root/scripts/build.sh"
    binary="${CARGO_TARGET_DIR:-$root/target}/release/vitrilyr"
fi
"$binary" --version
config=${XDG_CONFIG_HOME:-$HOME/.config}
data=${XDG_DATA_HOME:-$HOME/.local/share}
if [[ -x "$HOME/.local/bin/lyricglass" ]]; then
    "$HOME/.local/bin/lyricglass" quit
fi
if command -v systemctl >/dev/null && systemctl --user show-environment >/dev/null 2>&1; then
    if [[ -f "$config/systemd/user/lyricglass.service" ]]; then
        systemctl --user disable --now lyricglass.service
    fi
    if [[ -f "$config/systemd/user/vitrilyr.service" ]]; then
        systemctl --user stop vitrilyr.service
    fi
fi
if [[ -x "$HOME/.local/bin/vitrilyr" ]]; then
    "$HOME/.local/bin/vitrilyr" quit
fi
# Keep the legacy configuration and cache as a rollback copy.
if [[ ! -e "$config/vitrilyr/config.json" && -f "$config/lyricglass/config.json" ]]; then
    install -Dm600 "$config/lyricglass/config.json" "$config/vitrilyr/config.json"
fi
install -Dm755 "$binary" "$HOME/.local/bin/vitrilyr.new"
mv -- "$HOME/.local/bin/vitrilyr.new" "$HOME/.local/bin/vitrilyr"
if [[ ! -e "$HOME/.local/bin/niri-vitrilyr" && -x "$HOME/.local/bin/niri-lyricglass" ]]; then
    ln -s niri-lyricglass "$HOME/.local/bin/niri-vitrilyr"
fi
"$HOME/.local/bin/vitrilyr" install-desktop
"$HOME/.local/bin/vitrilyr" autostart "$startup"
if [[ -f "$config/niri/config.kdl" ]] && grep -q 'lyricglass/shortcuts.kdl' "$config/niri/config.kdl"; then
    "$HOME/.local/bin/vitrilyr" install-shortcuts
fi
rm -f -- "$config/autostart/io.github.lyricglass.LyricGlass.desktop" "$data/applications/io.github.lyricglass.LyricGlass.desktop"
if [[ $startup == on && ( -n ${WAYLAND_DISPLAY:-} || -n ${DISPLAY:-} ) ]]; then
    "$HOME/.local/bin/vitrilyr" autostart-run
fi
printf 'Installed %s/.local/bin/vitrilyr. Login startup: %s\n' "$HOME" "$startup"
