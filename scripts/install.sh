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
args=(--release --locked --manifest-path "$root/Cargo.toml" --target-dir "$root/target")
if ! pkg-config --exists gtk4-layer-shell-0; then
    args+=(--no-default-features)
    echo 'Building without layer-shell; X11 and desktop-window mode remain available.'
fi
cargo build "${args[@]}"
install -Dm755 "$root/target/release/lyricglass" "$HOME/.local/bin/lyricglass.new"
mv -- "$HOME/.local/bin/lyricglass.new" "$HOME/.local/bin/lyricglass"
"$HOME/.local/bin/lyricglass" install-desktop
"$HOME/.local/bin/lyricglass" autostart "$startup"
printf 'Installed %s/.local/bin/lyricglass. Login startup: %s\n' "$HOME" "$startup"
