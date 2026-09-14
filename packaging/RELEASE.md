# Vitrilyr for Linux x86_64

`vitrilyr-linux-x86_64` is the actual ELF executable, not a launcher or installer.
Built on Ubuntu 24.04 with Rust 1.95.0. Requires x86_64 Linux, glibc 2.39+,
GTK 4.12+, GLib 2.80+, a graphical session and a session D-Bus daemon.
The gtk4-layer-shell 1.3.0 library is included in the executable.
GTK and its usual runtime libraries remain dynamically linked.

On Ubuntu 24.04:

```sh
sudo apt-get install libgtk-4-1 dbus ca-certificates
sha256sum -c SHA256SUMS
chmod +x vitrilyr-linux-x86_64
./vitrilyr-linux-x86_64 --version
./vitrilyr-linux-x86_64
```

For menu integration and automatic startup, put the executable in a permanent
location before enabling the user service:

```sh
install -Dm755 vitrilyr-linux-x86_64 "$HOME/.local/bin/vitrilyr"
~/.local/bin/vitrilyr install-desktop
~/.local/bin/vitrilyr autostart on
~/.local/bin/vitrilyr autostart-run
```

Startup happens at graphical login, not before the desktop exists. On Niri, run
`~/.local/bin/vitrilyr install-shortcuts` once. X11 registers its shortcuts directly.
Other Wayland desktops need their own shortcut bindings. GNOME uses a regular
window fallback. Full optical refraction requires the optional Niri compositor,
which is not included here. Spotify must expose MPRIS; lyrics come from LRCLIB.

For upgrades from LyricGlass, use the source checkout's installer with
`VITRILYR_BINARY=/absolute/path/vitrilyr-linux-x86_64 ./scripts/install.sh`.
It preserves settings, disables the old service and migrates the Niri include.
Existing cache files remain untouched; new downloads use the Vitrilyr cache.

The native integration test runs this release executable on a private X11 display
before packaging. See the repository documentation for other desktop limitations.
An ARM, Windows or macOS binary is not provided by this release.

Application license: MIT. Bundled gtk4-layer-shell: MIT, upstream source pinned to
https://github.com/wmww/gtk4-layer-shell/tree/1c963c51514581c41b9bdae08cdf69171265cdda
Keep both license files when redistributing this executable.
