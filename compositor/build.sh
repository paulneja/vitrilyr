#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
revision=49fc6117fd6c043adaa2ead316b82db5ed735d36
work=$(mktemp -d "${TMPDIR:-/tmp}/vitrilyr-build.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
git clone --filter=blob:none --no-checkout https://github.com/niri-wm/niri.git "$work/niri"
git -C "$work/niri" checkout --detach "$revision"
git -C "$work/niri" apply --check "$root/compositor/niri-glass.patch"
git -C "$work/niri" apply "$root/compositor/niri-glass.patch"
cp "$root/compositor/clipped_surface.frag" "$work/niri/src/render_helpers/shaders/clipped_surface.frag"
export CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/vitrilyr/compositor-target"
cargo build --release --locked --manifest-path "$work/niri/Cargo.toml"
install -Dm755 "$CARGO_TARGET_DIR/release/niri" "$HOME/.local/bin/niri-vitrilyr.new"
mv "$HOME/.local/bin/niri-vitrilyr.new" "$HOME/.local/bin/niri-vitrilyr"
printf 'Installed separately: %s/.local/bin/niri-vitrilyr\n' "$HOME"
