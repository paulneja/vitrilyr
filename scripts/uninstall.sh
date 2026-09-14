#!/usr/bin/env bash
set -euo pipefail
config="${XDG_CONFIG_HOME:-$HOME/.config}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
if [[ -x "$HOME/.local/bin/lyricglass" ]]; then
    "$HOME/.local/bin/lyricglass" autostart off
    "$HOME/.local/bin/lyricglass" quit
fi
if command -v systemctl >/dev/null; then
    systemctl --user disable --now lyricglass.service 2>/dev/null || true
fi
rm -f -- "$HOME/.local/bin/lyricglass" "$config/systemd/user/lyricglass.service" "$data/applications/io.github.lyricglass.LyricGlass.desktop" "$config/autostart/io.github.lyricglass.LyricGlass.desktop"
if command -v systemctl >/dev/null; then systemctl --user daemon-reload 2>/dev/null || true; fi
echo 'Removed the application. Preferences, cache and optional compositor were kept.'
echo 'Remove the LyricGlass shortcuts include from Niri before deleting its configuration directory.'
