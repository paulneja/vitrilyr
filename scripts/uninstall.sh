#!/usr/bin/env bash
set -euo pipefail
config="${XDG_CONFIG_HOME:-$HOME/.config}"
data="${XDG_DATA_HOME:-$HOME/.local/share}"
if [[ -x "$HOME/.local/bin/vitrilyr" ]]; then
    "$HOME/.local/bin/vitrilyr" autostart off
    "$HOME/.local/bin/vitrilyr" quit
fi
if command -v systemctl >/dev/null; then
    systemctl --user disable --now vitrilyr.service 2>/dev/null || true
fi
rm -f -- "$HOME/.local/bin/vitrilyr" "$config/systemd/user/vitrilyr.service" "$data/applications/io.github.paulneja.Vitrilyr.desktop" "$config/autostart/io.github.paulneja.Vitrilyr.desktop"
if command -v systemctl >/dev/null; then systemctl --user daemon-reload 2>/dev/null || true; fi
echo 'Removed the application. Preferences, cache and optional compositor were kept.'
echo 'Remove the Vitrilyr shortcuts include from Niri before deleting its configuration directory.'
