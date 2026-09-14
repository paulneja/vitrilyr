#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
config="${XDG_CONFIG_HOME:-$HOME/.config}/vitrilyr"
binary="$HOME/.local/bin/niri-vitrilyr"
case "${1:-preview}" in
    preview)
        test -x "$binary" || { echo "Run compositor/build.sh first." >&2; exit 1; }
        "$root/target/release/vitrilyr" prepare-liquid
        export GTK_A11Y=none
        export GIO_USE_VFS=local
        exec dbus-run-session --config-file="$root/compositor/preview-bus.conf" -- "$binary" --config "$config/liquid-preview.kdl"
        ;;
    session)
        if [[ -n "${WAYLAND_DISPLAY:-}${DISPLAY:-}" ]]; then
            echo "Start from a TTY after logging out. Your current desktop was not changed." >&2
            exit 1
        fi
        "$root/target/release/vitrilyr" prepare-liquid
        exec dbus-run-session -- "$binary" --session --config "$config/liquid-session.kdl"
        ;;
    *) echo "Usage: $0 [preview|session]" >&2; exit 2 ;;
esac
