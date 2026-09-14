# Verification

Last local verification: 2026-09-13. Arch Linux x86_64, Rust 1.95.0,
GTK 4.22.4, gtk4-layer-shell C library 1.3.0 and Niri 26.04.

## Automated coverage

- Formatting and strict Clippy with all features.
- Formatting and strict Clippy without the optional layer-shell feature.
- 22 unit tests plus one private-D-Bus MPRIS integration test.
- One additional native X11 integration test, explicitly enabled with
  `cargo test --test x11 -- --ignored`.
- Native X11 test also passes with `--no-default-features`.
- Release build, desktop-entry validation and user-service syntax validation.

The unit suite covers LRC timestamps, repeated tags, simultaneous lines, empty
sections, offsets, seeking, malformed metadata, bounded cache reads, offline
fallback, negative caching, settings migration/ranges, import validation,
executable-path quoting, shortcut parsing and localization completeness.

The MPRIS integration test uses its own bus and covers Spotify absent, starting,
play/pause, next/previous, relative and absolute seek, volume, closing and
reacquiring its bus name. It never sends commands to the user's Spotify.

The X11 integration test launches actual GTK windows inside a private Xvfb and
D-Bus session. It verifies:

- All five styles produce nonblank native snapshots.
- Normal, Line and Square layouts render; the minimum square is exactly 240 x 240.
- English, Spanish and Simplified Chinese preferences render and switch live.
- Coordinate commands reach the real X11 window.
- Injected mouse motion drags the grip to the expected position and saves it.
- The EWMH above hint is present.
- Injected F7 presses hide and show the window.
- Click-through creates an empty native input region.
- Settings import/export updates a running instance and preserves startup choice.
- Immediate exports use live preferences rather than an outdated file.
- Immediate Quit and SIGTERM flush the latest settings and bounded coordinates.
- Recovery restores access without resetting appearance, language or startup.
- Switching language retains the selected preferences tab.
- Oversized startup configuration files fall back to defaults.

All five generated stock-Niri rule files and all five optional Liquid Glass
rule files passed their respective compositor validators. The X11 harness waits
for D-Bus ownership before sending commands after a restart and bounds command
execution, preventing a status request from accidentally becoming the primary
test instance.

Screenshots in `screenshots/` are native widget captures with original sample
lyrics, not browser mockups. They exclude background blur and refraction.
The test uses temporary settings and cannot enable the real login service.

## Real session checks

Actual Spotify discovery, artwork and synchronized LRCLIB lyrics were verified
in the normal local session. Unsigned Spotify duration and string track IDs are
covered by a regression test. Starting, pausing and track changes were observed.

Niri reported a real overlay layer with namespace `lyricglass` and keyboard
interactivity `None`. CLI hide/show, single-instance delivery, normal/line/square
layouts, minimum square size and free-position persistence were checked.
The shortcut installer validates the complete Niri configuration before adding
the include and preserves an original backup.

The user installer was run end to end. The permanent `lyricglass.service` is
enabled and running, replacing the temporary launch unit. The installed desktop
and autostart entries and systemd unit passed their validators; `niri validate`
also passed. Disabling and reenabling login startup left the same application
process running. Existing appearance, placement and shortcut settings survived
the update. English defaults were verified separately from the user's explicit
language preference. The installed app reports the native Wayland layer backend,
real Spotify playback and synchronized lyrics. An actual reboot/login cycle has
not been performed.

## Optional refraction

- The pinned Niri-glass integration and replacement optical shader built in release.
- `niri-lyricglass` was installed separately; stock Niri was not replaced.
- A nested Wayland session rendered the real GTK overlay over an optical grid.
- Comparing refraction 0/6 changed pixels in an interior bevel strip; a checked
  900 x 300 region outside the panel had exactly zero pixel difference.
- `liquid-optics.png` includes the actual compositor effect.
- Free placement was exercised and persisted in the nested preview.
- The preview uses a private bus with no external service activation.

## Remaining coverage

No claim is made of full desktop testing on GNOME, Plasma, Sway, Hyprland,
Windows, macOS or every Linux distribution. The managed Wayland fallback compiles
but has not been exercised in a complete GNOME session.

Xvfb has no window manager, so the test does not prove that every desktop honors
always-on-top or that shortcuts work under exclusive game input. Multiple
monitors, unusual keyboard maps, fractional scaling and fullscreen games remain
manual-test gaps.

The optional compositor was not tested as the primary DRM session, in games or
on multiple monitors. Full refraction requires that separate session. Live
Spotify was verified in the normal session, not inside the isolated optical
preview. Hosted CI has not been run locally.
