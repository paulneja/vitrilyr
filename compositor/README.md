# Native Liquid Glass

An optional, experimental compositor installed **alongside** Niri. This is not
Apple code or a pixel-identical iOS implementation. Stock Niri provides blur;
this variant also bends the actual live scene behind the Vitrilyr layer.
No screenshots, screen-recording portal or browser produce the effect.

## Sources and Licenses

- Niri: https://github.com/niri-wm/niri at
  `49fc6117fd6c043adaa2ead316b82db5ed735d36`, GPL-3.0-or-later.
- Integration: https://github.com/zaroutt/Niri-glass at
  `147c35b0c328220fb60e5fd184a2a8bdf17bebeb`. Its added integration code is
  offered under MIT; inherited Niri code remains GPL.
- The original overlay shader references https://github.com/4v3ngR/kwin-effects-glass.
  We replace it with the smaller `clipped_surface.frag` here, GPL-3.0-or-later.
- License texts: `LICENSE-NIRI` and `LICENSE-NIRI-GLASS`.
  The GTK application remains a separate MIT-licensed program.

`niri-glass.patch` contains the integration and corrections against the pinned
base. The build script also copies `clipped_surface.frag`, the authoritative
shader source. Offsets use local geometry and the inverse framebuffer transform.
Geometry is in logical pixels. Snell refraction (IOR 1.46), a curved bevel,
restrained center lens, RGB dispersion and directional edge reflections act on
the live background, not the text. Per-frame warning spam was removed.
Non-xray glass uses a light one-pass blur locally; other surfaces keep their blur.

## Build

Additional Arch dependencies:

```sh
sudo pacman -S --needed clang libinput libxkbcommon libseat libdisplay-info mesa pipewire
cargo build --release --examples --bin vitrilyr
./compositor/build.sh
```

The pinned build uses Cargo.lock and installs `~/.local/bin/niri-vitrilyr`.
It never replaces `/usr/bin/niri`, your active configuration or login manager.
Treat it as experimental; test before using it as your main session.

## Preview

```sh
./compositor/run.sh preview
```

Opens a nested desktop with an optical grid and the real GTK app using original
demo lyrics. Escape closes it, F7 toggles the panel and Shift+F7 opens settings.
It has its own D-Bus and preferences under `~/.config/vitrilyr/preview/` and
does not control Spotify. Starting it resets only those preview preferences.
The preview bus cannot activate desktop daemons. Accessibility is disabled only
inside that diagnostic preview, preventing collisions with session services.

## Real Desktop

`vitrilyr prepare-liquid` generates and validates a separate
`~/.config/vitrilyr/liquid-session.kdl`, including your existing Niri config and
the optical rules. Stock Niri must never include `liquid-effects.kdl`. The regular
shortcut include uses only stock-compatible `surface-effects.kdl`. Appearance
sliders update both rule files atomically off the GTK thread.

For the full effect over games and applications, log out, enter a TTY, log in,
and run the absolute path to `compositor/run.sh session`. The script refuses to
start a real desktop over an existing graphical session. Launch Vitrilyr in
the new session. Shell services and portals still need their normal session
setup; this is not a replacement for distribution display-manager integration.
No display-manager entry is installed. To revert, log out and select your usual
Niri session. Its binary and configuration are intact.

Stock Niri supports the glossy material and native blur, but not true refraction.
The refraction slider needs the optional compositor. Surface reflections and
curvature work in either session. This alternative session has been tested
nested, not as the primary DRM desktop or with games/multiple monitors.

## Recheck

Build the `glass_scene` example. Compare nested screenshots with
`vitrilyr material liquid --refraction 0` and `--refraction 6`, targeting its
private bus. The grid must bend inside the panel near its edges; outside pixels
must not change. Check normal, line, minimum square, dragging, settings,
reduced motion, hide/show and frosted fallback. `vitrilyr capture` verifies
widget layout only; `grim` on the nested socket verifies the compositor effect.
