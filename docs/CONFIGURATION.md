# Configuration

Open Preferences with `lyricglass settings` or Shift+F7 where global shortcuts
are configured. Changes apply live and save after a short debounce. English is
the default; Behavior offers Spanish and Simplified Chinese. Song text and
artist names are never translated.

## Appearance

| Setting | Default | Range / choices |
| --- | --- | --- |
| Style | Glass | Glass, Dark, Light, High contrast, Minimal |
| Material | Liquid Glass | Liquid Glass, Frosted; affects Glass style |
| Layout | Normal | Normal, Line, Square |
| Width | 580 px | 360-900 px, non-square layouts |
| Square size | 280 px | 240-440 px |
| Opacity | 86% | 30-100%, Glass material |
| Lyric font | Sans, 19 px | Native font family chooser, 14-28 px |
| Line spacing | 9 px | 4-20 px |
| Neighboring-line opacity | 35% | 10-75% |
| Transition | 200 ms | 100-400 ms |
| Frosted corners | 10 px | 0-24 px |
| Glass curvature | 32 px | 12-48 px |
| Reflections | 65% | 0-100% |
| Refraction | 6 | 0-16, optional patched Niri only |
| Accent | Mint | Mint, blue, rose, gold, custom color |

The font chooser changes the lyric family; the separate size control remains
authoritative. Small square layouts cap the lyric size to keep content inside
the panel. Long lines wrap or ellipsize; full text remains available in the
complete lyrics view. Artwork, playback buttons, progress, neighboring lines
and metadata can be hidden independently or through lyrics-only mode.

Dark and Light use solid surfaces; their opacity does not follow the Glass
slider. Minimal removes the drawn panel background, though compositor blur
rules may still affect the surface behind it. System reduced-motion settings
are honored in addition to the app's animation toggle.

Reset appearance restores visual defaults without changing language, startup,
position or shortcuts.

## Behavior and placement

Free placement uses the grip and persists on release. The Move command unlocks
position, disables click-through and shows the panel. Lock position hides the
grip. Select the monitor in Preferences before positioning; coordinates are
logical pixels relative to that output. The panel is constrained to its bounds.
Reset position returns to the top-center anchor.

Managed Wayland windows use the desktop's native move operation instead; saved
coordinates cannot override compositor policy. See [compatibility](COMPATIBILITY.md).

- **Hide when idle:** hide when Spotify is absent or stopped.
- **Hide when paused:** hide while playback is paused.
- **Start hidden:** launch without showing the overlay.
- **Click-through:** pointer input reaches applications below the overlay.
- **Start at login:** manage the user service and XDG login launcher.
- **Lyric offset:** -10,000 to +10,000 ms; positive advances the lyric display.

Hide/show does not quit the application. The settings command still works while
the overlay is hidden, paused or click-through. Turning off login startup does
not terminate a running instance.

## Playback

Previous, next, play/pause, ten-second seeks, volume and the seekable progress bar
use Spotify's reported MPRIS capabilities. Disabled controls mean Spotify is
not available or has not advertised that operation. Playlist editing, library
search and liking tracks are not provided by MPRIS; use Spotify for those.

## Keyboard commands

```sh
lyricglass show
lyricglass hide
lyricglass toggle
lyricglass settings
lyricglass game-mode
lyricglass play-pause
lyricglass previous
lyricglass next
lyricglass seek -10
lyricglass volume 50
lyricglass layout normal
lyricglass layout line
lyricglass layout square
lyricglass style contrast
lyricglass language en
lyricglass language es
lyricglass language zh
lyricglass move
lyricglass position 120 180
lyricglass reset-position
lyricglass material liquid --refraction 6
lyricglass material frosted
lyricglass autostart on
lyricglass autostart off
lyricglass status
lyricglass quit
```

Commands address the same application instance, including while hidden. Use
`lyricglass --help` for available subcommands. `status` prints JSON including
backend, language, style, visibility, geometry and playback state. Demo state
is reported separately from an actual Spotify connection.

Niri bindings are installed only after validating the complete candidate
configuration, with a one-time `config.kdl.before-lyricglass` backup. On X11,
keys are registered directly; failed grabs keep the previous set. Other Wayland
desktops need manual command bindings.

## Backups and files

```sh
lyricglass export-config ~/lyricglass-settings.json
lyricglass import-config ~/lyricglass-settings.json
```

Preferences also has native import/export file dialogs. Imports apply live,
normalize ranges and preserve the current login-startup choice. Files must be
valid JSON and at most 128 KiB. Backups can include display connector names and
shortcut choices from another machine; review these after importing.

Settings live in `$XDG_CONFIG_HOME/lyricglass/config.json`, normally
`~/.config/lyricglass/config.json`. Missing fields receive defaults. Invalid JSON
falls back to defaults with a warning. Writes are atomic. If editing JSON by
hand, stop the app first so a pending live change cannot overwrite your edit.
Use import for changes while running; there is no file watcher.

Generated `surface-effects.kdl` and `liquid-effects.kdl` track appearance for
stock and optional Niri respectively. Do not include the patched-compositor
rules in stock Niri. `shortcuts.kdl` is generated by the Niri shortcut installer.

Cache is under `$XDG_CACHE_HOME/lyricglass/`, normally `~/.cache/lyricglass/`.
It contains `lyrics/` JSON and `artwork/` images and can be removed while the
app is stopped. It contains no Spotify credentials.
