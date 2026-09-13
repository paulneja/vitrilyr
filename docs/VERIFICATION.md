# Verification

Verified on Arch Linux with Rust 1.95.0, GTK 4.22.4,
gtk4-layer-shell C library 1.3.0 and Niri 26.04.

- `cargo fmt --check`: passed.
- `cargo check`: passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- `cargo test`: 14 unit tests and 1 private-D-Bus integration test passed.
- `cargo build --release`: passed.
- Actual Spotify detected dynamically; artwork and synchronized LRCLIB lyrics
  loaded, playback state and track changes observed.
- Real Spotify's unsigned duration and string track ID verified and covered by
  a regression test.
- Real layer-shell surface verified with `niri msg --json layers`: overlay layer,
  namespace `lyricglass`, keyboard interactivity `None`.
- Shortcut/effects include installed only after full `niri validate` succeeded;
  original config backed up. CLI hide/show and single-instance delivery checked.
- Normal, narrow line and 280 x 280 square layouts rendered and inspected.
- Square layout and free coordinates survived a process restart.
- Settings tabs rendered and inspected using GTK's own snapshot API.

The integration test uses a separate D-Bus daemon and covers Spotify absent,
starting, play/pause, next/previous, relative and absolute seeking, volume,
closing and reacquiring its bus name. It never controls the user's Spotify.
Unit tests also cover malformed metadata/LRC/cache, offline stale-cache fallback,
negative caching, missing artwork, plain lyrics and instrumental states.

`square.png`, `line.png`, and `settings.png` are native widget renders, not web
mockups. Widget-only captures do not include Niri's live background blur.

The user's session locked during final visual checks. Free-position commands and
persistence were verified; the physical pointer drag still needs an unlocked
session for end-to-end verification. Multi-monitor dragging was not tested;
placement is intentionally constrained to the monitor selected in preferences.

## Liquid Glass, 2026-09-13

- Pinned Niri-glass integration and replacement optical shader compiled in release.
- Separate `niri-lyricglass` installed; stock Niri binary was not replaced.
- Nested Wayland session rendered the real GTK overlay over the optical grid.
  No shader compilation errors were logged. The preview exited normally on request.
- Comparing refraction 0/6 changed pixels in an interior bevel strip; the checked
  900 x 300 region outside the panel had exactly zero pixel difference.
- `liquid-optics.png` includes the actual compositor effect, not a mockup.
- Minimum square reports exactly 240 x 240 after the layout fix.
  `liquid-square.png` and `liquid-settings.png` are native GTK snapshots.
- Free positioning was exercised during the nested preview and persisted.
- App release/check/format/strict Clippy passed; all 15 tests passed.
- Updated app installed and restarted without stopping Spotify. Existing position,
  appearance and shortcut preferences were retained. Stock-compatible corner/shadow
  rules were installed only after full Niri validation.

The optional compositor was not tested as the main DRM session, in games, on
multiple monitors or at fractional scale. Full refraction needs that separate
session. The optical preview uses a private D-Bus and original demo lyrics.
It does not demonstrate live Spotify inside the optional compositor; actual
Spotify integration remains verified in the ordinary running app.
