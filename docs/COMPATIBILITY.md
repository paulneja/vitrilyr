# Desktop compatibility

Backend selection happens at launch: supported Wayland layer-shell first,
native X11 next, otherwise a regular GTK window. Inspect it with
`vitrilyr status`. No XWayland is needed on a supported Wayland compositor.

| Environment | Window behavior | Global shortcuts | Verification |
| --- | --- | --- | --- |
| Niri, Wayland | Real overlay layer, no keyboard focus, saved placement | Built-in Niri installer | Real local session |
| Sway, Hyprland, compatible layer-shell compositors | Real overlay layer | Bind CLI commands in compositor config | Protocol support; not individually tested |
| KDE Plasma, Wayland | Layer-shell when advertised by the compositor | Configure desktop shortcuts | Not tested locally |
| GNOME Wayland or no layer-shell | Normal decorated GTK window | Configure desktop shortcuts | Fallback compiles; full session not tested |
| X11 desktops | Native utility window, above request, saved placement | App registers keys directly | Isolated Xvfb integration tests |

The upstream [supported desktop list](https://github.com/wmww/gtk4-layer-shell#supported-desktops)
documents the layer-shell library's scope. Backend availability does not prove
identical behavior across window managers, drivers or games.

## Wayland limits

The layer surface reserves no workspace space. Its namespace is `vitrilyr`;
preferences and full lyrics use `vitrilyr-settings`. Normal anchored placement
uses exclusive zone zero. Free placement uses zone -1 to measure from output
edges. Dragging and saved coordinates stay within the selected output.

Without layer-shell, ordinary Wayland clients cannot freely set global position
or force always-on-top. Use the window manager's move/keep-above actions. The grip
requests a native interactive move. Coordinate and monitor preferences cannot
override compositor policy in this mode. Its keyboard focus follows ordinary
window behavior, unlike the layer overlay.

Only Niri shortcut installation is automated on Wayland. On GNOME, Plasma, Sway
or Hyprland, bind `vitrilyr toggle`, `vitrilyr settings` and
`vitrilyr game-mode` in the desktop's shortcut settings. There is no global
shortcuts portal integration yet. Niri bindings use `allow-inhibiting=false`;
other desktops and exclusive-input games may handle shortcuts differently.

## X11 limits

The app requests `_NET_WM_STATE_ABOVE`, a utility role and exclusion from the
taskbar. The window manager decides whether to honor those hints. Click-through
uses an empty input region. Shortcut conflicts preserve the previous working
bindings and report an error. Caps Lock and the conventional Mod2 Num Lock
modifier are ignored; unusual keyboard maps may need different bindings.

Xvfb tests exercise real X11 windows, key injection, geometry and input regions,
but do not run a complete window manager. Fullscreen games, multiple monitors,
fractional scaling and individual X11 desktops still need manual coverage.

## Material effects

| Style / effect | Requirement |
| --- | --- |
| Dark, Light, High Contrast, Minimal | GTK rendering |
| Glass highlights and transparency | GTK; X11 needs a compositing manager for transparency |
| Background blur | Compositor support and matching rules |
| Background refraction | Optional patched Niri session |

Stock Niri blur rules are installed with the shortcut configuration. They require
a Niri version supporting background effects; the installer validates syntax
before changing the main configuration. On other compositors configure blur
using their own rules. Vitrilyr cannot create true desktop-background blur
inside a normal transparent GTK window.

The optional [Liquid Glass compositor](../compositor/README.md) is experimental,
installed separately and never selected automatically. It was tested in a nested
session, not as a primary DRM session or with a fullscreen game.

## Operating systems and architectures

Linux is the supported operating system. Builds and native tests were performed
on x86_64 Arch Linux. Debian/Ubuntu/Fedora installation paths are documented, not
claims of completed hardware testing on those distributions. Other architectures
may build from source if dependencies are available; they are not verified.

Windows and macOS ports are not implemented. They would require different media
discovery, global shortcuts, startup and window-management backends. The Linux
MPRIS and X11 code is not a cross-platform implementation of those services.
