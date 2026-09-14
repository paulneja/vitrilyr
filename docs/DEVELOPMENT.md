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
desktop-file-validate data/io.github.paulneja.Vitrilyr.desktop
systemd-analyze --user verify data/vitrilyr.service
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
VITRILYR_TEST_ARTIFACTS=/tmp/vitrilyr-captures cargo test --test x11 -- --ignored
```

This renders every style, all three layouts and preferences in three languages.
Inspect text wrapping, minimum square geometry, disabled controls and input
regions. For a normal session, `vitrilyr demo` uses original sample lines when
no instance is already running. Quit the real instance first; demo never drives
Spotify. Hidden diagnostic commands `capture /absolute/path.png` and
`capture-settings /absolute/path.png` snapshot GTK widgets only. They do not
capture other applications, compositor blur or refraction.

The optional compositor has its own [build and preview workflow](../compositor/README.md).
Keep the stock compositor intact when testing it.

## Release notes

Run `./scripts/release.sh` on an x86_64 Linux host with Docker. It builds on
Ubuntu 24.04 using the locked Rust dependencies and a pinned gtk4-layer-shell
revision. The C layer library is linked into the ELF; GTK remains dynamic.
The native X11 suite exercises that exact release executable before files and
SHA-256 checksums are exported to `dist/`. `VITRILYR_TEST_BINARY` can point the
same test at another executable. Do not enable that override for untrusted files.

Push a tag matching the Cargo version, such as `v0.1.0`, to run the release
workflow. It verifies the repository is private before building and again before
uploading the ELF and notices to GitHub Releases. It never changes visibility.
The ordinary CI job separately tests the build without layer-shell.

Keep third-party notices and licenses with sources and distributed binaries.
The optional compositor is not part of the application ELF release.

## Rename migration

The application ID is `io.github.paulneja.Vitrilyr`; config and cache directories
are named `vitrilyr`. If the new settings file is absent, the app reads the old
`lyricglass/config.json` without modifying it. New preferences take precedence.
The installer disables the old service, copies existing preferences only when
the new file is absent, removes old menu/login launchers and replaces an exact
legacy Niri shortcut include after validating the entire configuration.
Old cache files and the optional compositor executable remain untouched.
