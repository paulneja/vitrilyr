#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for tool in cargo pkg-config; do
    command -v "$tool" >/dev/null || { echo "Missing $tool. See docs/INSTALLATION.md." >&2; exit 1; }
done
pkg-config --atleast-version=4.12 gtk4 || { echo 'GTK 4.12+ development files are required.' >&2; exit 1; }
pkg-config --atleast-version=2.80 gio-2.0 || { echo 'GLib 2.80+ development files are required.' >&2; exit 1; }
args=(--release --locked --manifest-path "$root/Cargo.toml" --target-dir "${CARGO_TARGET_DIR:-$root/target}")
case "${1:-auto}" in
    auto)
        if ! pkg-config --exists gtk4-layer-shell-0; then
            args+=(--no-default-features)
            echo 'Layer-shell unavailable: building X11 and managed Wayland window support.'
        fi ;;
    --x11) args+=(--no-default-features) ;;
    --wayland) pkg-config --exists gtk4-layer-shell-0 || { echo 'Install gtk4-layer-shell development files.' >&2; exit 1; } ;;
    *) echo "Usage: $0 [--x11|--wayland]" >&2; exit 2 ;;
esac
cargo build "${args[@]}"
printf 'Built %s/release/vitrilyr\n' "${CARGO_TARGET_DIR:-$root/target}"
