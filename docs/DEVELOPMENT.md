# Development

## Layout

| Module | Responsibility |
| --- | --- |
| `main.rs`, `app.rs` | CLI, single-instance actions and application lifetime |
| `player.rs`, `state.rs` | MPRIS discovery, signals, commands and monotonic playback clock |
| `lyrics.rs`, `lrc.rs`, `cache.rs` | Lookup, bounded downloads, parsing and atomic cache writes |
| `backend.rs` | Worker runtime, request cancellation and preference persistence |
| `ui.rs`, `lyric_view.rs`, `material.rs` | Native widgets, lyric animation and glass drawing |
| `settings.rs`, `i18n.rs` | Live preferences and English/Spanish/Chinese catalogs |
| `platform.rs` | Wayland layer-shell, X11 and managed-window behavior |
| `config.rs`, `startup.rs` | Configuration, Niri rules, desktop entry and login service |

GTK mutations remain on the main thread. HTTP, artwork decoding and asynchronous
file work run in a two-thread Tokio runtime. Slow startup/shortcut installation
uses blocking worker tasks. Lyric/artwork requests carry generations so stale
results cannot overwrite the current track. MPRIS signals update state promptly;
position resynchronization runs every two seconds, with a monotonic clock between
updates. Slider writes are coalesced without discarding queued import/install
operations.

The exported application action state holds the current normalized preferences,
so CLI backups do not depend on a delayed file write. A worker-queue barrier
finishes pending writes on normal shutdown, including SIGINT/SIGTERM service
stops, with a three-second maximum wait. Forced kills and power loss cannot use
that graceful shutdown path.

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo clippy --no-default-features --all-targets -- -D warnings
cargo test --no-default-features
cargo test --test x11 -- --ignored
cargo test --no-default-features --test x11 -- --ignored
cargo build --release --locked
bash -n scripts/install.sh scripts/uninstall.sh
desktop-file-validate data/io.github.lyricglass.LyricGlass.desktop
systemd-analyze --user verify data/lyricglass.service
```

The ignored X11 test needs Xvfb, D-Bus and a working GTK icon theme. Install a CJK
font such as Noto Sans CJK to inspect Chinese text. Tests start private displays
and buses with no external service activation, set temporary configuration
directories and clean up child processes. They do not control real Spotify,
change desktop settings or enable startup.

Keep the public MPRIS protocol test independent of live Spotify. Changes to
timing, cache lookup or metadata decoding should include regression coverage.
Platform changes also need native session testing; an Xvfb geometry assertion
cannot verify fullscreen stacking under a real window manager.

The Linux CI workflow builds without the optional C layer-shell dependency on
Ubuntu 24.04, runs strict Clippy and both ordinary and native X11 tests. Local
checks do not establish that a hosted CI run has passed.

## Visual checks

```sh
LYRICGLASS_TEST_ARTIFACTS=/tmp/lyricglass-captures cargo test --test x11 -- --ignored
```

This renders every style, all three layouts and preferences in three languages.
Inspect text wrapping, minimum square geometry, disabled controls and input
regions. For a normal session, `lyricglass demo` uses original sample lines when
no instance is already running. Quit the real instance first; demo never drives
Spotify. Hidden diagnostic commands `capture /absolute/path.png` and
`capture-settings /absolute/path.png` snapshot GTK widgets only. They do not
capture other applications, compositor blur or refraction.

The optional compositor has its own [build and preview workflow](../compositor/README.md).
Keep the stock compositor intact when testing it.

## Release notes

Use the locked dependency set for releases and build against the oldest runtime
you intend to support. Test both feature configurations. Before public
distribution, replace the unpublished local-build HTTP User-Agent in
`lyrics.rs` with a real project/contact URL. Do not invent a public repository
address. Keep third-party compositor notices and licenses with its sources and
any distributed binaries.
