# Installation

## Requirements

- Linux with a graphical session and a user D-Bus session.
- Rust 1.92 or newer, a C compiler and pkg-config.
- GTK 4.12 or newer and GLib 2.80 or newer, including development files.
- The C library gtk4-layer-shell for the optional Wayland layer backend.
- Native or Flatpak Spotify exposing MPRIS on the same session bus.

Check versions before building:

```sh
rustc --version
pkg-config --modversion gtk4 glib-2.0
pkg-config --modversion gtk4-layer-shell-0
```

The last check is optional for X11 and ordinary desktop-window mode. The Rust
layer-shell crate does not replace the C library. Older distributions such as
Debian 12 need newer GTK/GLib packages; disabling layer-shell does not remove
the GTK version requirement. Install a current Rust toolchain using
[rustup](https://rustup.rs/) if your distribution's compiler is too old.

## Distribution packages

### Arch Linux

```sh
sudo pacman -S --needed base-devel rust gtk4 gtk4-layer-shell pkgconf dbus
```

### Debian 13

```sh
sudo apt update
sudo apt install build-essential pkg-config libgtk-4-dev libgtk4-layer-shell-dev dbus
```

Debian provides the [layer-shell development package](https://packages.debian.org/trixie/libgtk4-layer-shell-dev).
Use Rust 1.92+ independently of the system GTK packages.

### Ubuntu 24.04

```sh
sudo apt update
sudo apt install build-essential pkg-config libgtk-4-dev dbus
```

Ubuntu 24.04's [GTK development package](https://packages.ubuntu.com/en/noble-updates/libgtk-4-dev)
meets the minimum. Without the separate layer-shell library, the installer builds
X11 and ordinary desktop-window support. To use a Wayland layer overlay, install
the library following its [upstream instructions](https://github.com/wmww/gtk4-layer-shell#building-from-source),
then rebuild. Do not mix packages from a different distribution release.

### Fedora

```sh
sudo dnf install gcc pkgconf-pkg-config gtk4-devel gtk4-layer-shell-devel dbus-daemon
```

Fedora provides [gtk4-layer-shell-devel](https://packages.fedoraproject.org/pkgs/gtk4-layer-shell/gtk4-layer-shell-devel/).
Check GTK/GLib and Rust versions for your Fedora release.

## Build and install

From the source directory:

```sh
./scripts/install.sh
~/.local/bin/lyricglass
```

The script uses the locked dependency set and replaces the installed executable
atomically. It detects the C layer-shell library through pkg-config. It does not
install system packages, overwrite your compositor or require root.

For a manual build without layer-shell:

```sh
cargo build --release --locked --no-default-features
target/release/lyricglass
```

This is a source-build compatibility option, not a static universal binary.
Runtime GTK/GLib libraries are still required. A binary built against a newer
distribution may not run on an older one. No AppImage, Flatpak, Windows or macOS
package is currently provided.

Installed files:

| File | Purpose |
| --- | --- |
| `~/.local/bin/lyricglass` | Executable |
| `$XDG_DATA_HOME/applications/io.github.lyricglass.LyricGlass.desktop` | Application menu |
| `$XDG_CONFIG_HOME/systemd/user/lyricglass.service` | User service |
| `$XDG_CONFIG_HOME/autostart/io.github.lyricglass.LyricGlass.desktop` | Login launcher |

XDG defaults are `~/.local/share` and `~/.config`. Commands in this guide assume
`~/.local/bin` is on PATH; otherwise use the executable's full path.

## Automatic startup

Installation enables startup by default. The application starts at **graphical
login**, not before login or on the lock screen. It is a user service, never a
root service, and does not enable user lingering.

```sh
lyricglass autostart on
lyricglass autostart off
systemctl --user status lyricglass.service
journalctl --user -u lyricglass.service -b
```

The same setting is available in Preferences under Behavior. Turning it off
disables future login startup without closing the current overlay. Start hidden
is a separate preference. Close the running app with `lyricglass quit`.

On systemd desktops, the service attaches to `graphical-session.target` and
restarts after failures. The XDG launcher imports the current display environment
and starts that same service; it does not create a second overlay. On non-systemd
desktops, the XDG launcher starts the application directly. Desktops must support
either the graphical-session target or XDG autostart; minimal window-manager
sessions that provide neither need their own startup command:

```sh
/home/YOUR_USER/.local/bin/lyricglass autostart-run
```

Do not add a second direct app launch if your desktop already runs the launcher.
This follows the [XDG autostart specification](https://specifications.freedesktop.org/autostart/0.5/).
Simultaneous graphical sessions for the same user are not supported: one user bus
and application ID share one instance.

To start the service immediately from your graphical session:

```sh
systemctl --user import-environment DISPLAY WAYLAND_DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
lyricglass quit
systemctl --user start lyricglass.service
```

Only import variables actually set in your session; systemctl may warn about
missing ones. The generated unit records the executable's absolute path. Run
`autostart on` again if you move it. Existing differing units receive a one-time
`.service.backup` before replacement.

## Updates and removal

Run the installer again after updating the source. Preferences and cache are
retained. Restart the service to use the new binary:

```sh
systemctl --user restart lyricglass.service
```

If you launched manually, quit and reopen instead. To remove the app:

```sh
./scripts/uninstall.sh
```

Removal preserves preferences, cache and the optional compositor. Remove the
LyricGlass include from Niri's configuration before deleting its configuration
directory. Keep your existing Niri backup until you have validated the result.
