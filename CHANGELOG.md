# Changelog

## Unreleased

### Fixed

- Flush pending preference writes on Quit, SIGINT and SIGTERM.
- Export live preferences before the disk-write debounce completes.
- Save clamped coordinates instead of the original out-of-bounds request.
- Refresh open settings after external changes and keep the selected tab.
- Preserve song and artist tooltips when changing the interface language.
- Match compositor corners, blur and reflections to the selected style.
- Keep neighboring lyrics readable in High Contrast.
- Remove a duplicate X11 window-state request.
- Avoid querying Wayland layer-shell APIs on X11 displays.
- Use English defaults for remaining Spotify and lyric-provider errors.
- Enforce read limits while reading cache and startup configuration files.

### Added

- Restore access action and `lyricglass recover` command.
- Native regression coverage for immediate export, graceful exit, recovery,
  bounded configuration loading and language changes within settings.
