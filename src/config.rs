use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, process::Command};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub autostart: bool,
    pub language: String,
    pub theme: String,
    pub font_family: String,
    pub custom_accent: String,
    pub line_spacing: i32,
    pub transition_ms: i32,
    pub secondary_opacity: f64,
    pub lyrics_only: bool,
    pub show_context: bool,
    pub hide_paused: bool,
    pub lock_position: bool,
    pub start_hidden: bool,
    pub width: i32,
    pub opacity: f64,
    pub font_size: i32,
    pub margin: i32,
    pub bottom: bool,
    pub monitor: String,
    pub click_through: bool,
    pub compact: bool,
    pub square: bool,
    pub square_size: i32,
    pub show_artwork: bool,
    pub show_controls: bool,
    pub show_progress: bool,
    pub corner_radius: i32,
    pub liquid: bool,
    pub liquid_radius: i32,
    pub reflections: f64,
    pub refraction: f64,
    pub accent: String,
    pub free_position: bool,
    pub x: i32,
    pub y: i32,
    pub hide_idle: bool,
    pub animations: bool,
    pub lyric_offset_ms: i32,
    pub shortcuts: Shortcuts,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Shortcuts {
    pub toggle: String,
    pub settings: String,
    pub game: String,
    pub play_pause: String,
    pub previous: String,
    pub next: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            toggle: "F7".into(),
            settings: "Shift+F7".into(),
            game: "Ctrl+F7".into(),
            play_pause: "Ctrl+F8".into(),
            previous: "Ctrl+F9".into(),
            next: "Ctrl+F10".into(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 580,
            autostart: true,
            language: "en".into(),
            theme: "glass".into(),
            font_family: "Sans".into(),
            custom_accent: "#91d5b2".into(),
            line_spacing: 9,
            transition_ms: 200,
            secondary_opacity: 0.35,
            lyrics_only: false,
            show_context: true,
            hide_paused: false,
            lock_position: false,
            start_hidden: false,
            opacity: 0.86,
            font_size: 19,
            margin: 16,
            bottom: false,
            monitor: String::new(),
            click_through: false,
            compact: false,
            square: false,
            square_size: 280,
            show_artwork: true,
            show_controls: true,
            show_progress: true,
            corner_radius: 10,
            liquid: true,
            liquid_radius: 32,
            reflections: 0.65,
            refraction: 6.0,
            accent: "mint".into(),
            free_position: false,
            x: 100,
            y: 100,
            hide_idle: false,
            animations: true,
            lyric_offset_ms: 0,
            shortcuts: Shortcuts::default(),
        }
    }
}

pub fn dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.config_dir().join("lyricglass"))
        .unwrap_or_else(|| PathBuf::from(".lyricglass"))
}

impl Config {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        anyhow::ensure!(bytes.len() <= 128 * 1024, "Settings file exceeds 128 KiB");
        let mut config: Self = serde_json::from_slice(bytes).context("Invalid settings JSON")?;
        config.normalize();
        Ok(config)
    }
    pub async fn read_from(path: &std::path::Path) -> Result<Self> {
        let bytes = crate::cache::read(path, 128 * 1024)
            .await
            .context("Cannot read settings file (maximum 128 KiB)")?;
        Self::decode(&bytes)
    }
    pub async fn export(&self, path: &std::path::Path) -> Result<()> {
        crate::cache::write(path, &serde_json::to_vec_pretty(self)?).await
    }
    pub fn clamp_position(&mut self, monitor: (i32, i32), panel: (i32, i32)) {
        self.x = self.x.clamp(0, monitor.0.saturating_sub(panel.0).max(0));
        self.y = self.y.clamp(0, monitor.1.saturating_sub(panel.1).max(0));
    }
    pub fn load() -> Self {
        match std::fs::read(dir().join("config.json")) {
            Ok(bytes) => match serde_json::from_slice::<Self>(&bytes) {
                Ok(mut config) => {
                    config.normalize();
                    config
                }
                Err(error) => {
                    tracing::warn!(%error, "Ignoring malformed settings");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
    pub fn normalize(&mut self) {
        if !["glass", "dark", "light", "contrast", "minimal"].contains(&self.theme.as_str()) {
            self.theme = "glass".into();
        }
        if self.font_family.is_empty()
            || self.font_family.len() > 120
            || self.font_family.chars().any(char::is_control)
        {
            self.font_family = "Sans".into();
        }
        if self.custom_accent.len() != 7
            || !self.custom_accent.starts_with('#')
            || !self.custom_accent[1..]
                .bytes()
                .all(|c| c.is_ascii_hexdigit())
        {
            self.custom_accent = "#91d5b2".into();
        }
        self.line_spacing = self.line_spacing.clamp(4, 20);
        self.transition_ms = self.transition_ms.clamp(100, 400);
        self.secondary_opacity = if self.secondary_opacity.is_finite() {
            self.secondary_opacity.clamp(0.1, 0.75)
        } else {
            0.35
        };
        if !["en", "es", "zh"].contains(&self.language.as_str()) {
            self.language = "en".into();
        }
        self.width = self.width.clamp(360, 900);
        self.square_size = self.square_size.clamp(240, 440);
        self.corner_radius = self.corner_radius.clamp(0, 24);
        self.liquid_radius = self.liquid_radius.clamp(12, 48);
        self.reflections = if self.reflections.is_finite() {
            self.reflections.clamp(0.0, 1.0)
        } else {
            0.65
        };
        self.refraction = if self.refraction.is_finite() {
            self.refraction.clamp(0.0, 16.0)
        } else {
            6.0
        };
        self.x = self.x.max(0);
        self.y = self.y.max(0);
        if self.square {
            self.compact = false;
        }
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(0.3, 1.0)
        } else {
            0.86
        };
        self.font_size = self.font_size.clamp(14, 28);
        self.margin = self.margin.clamp(0, 240);
        self.lyric_offset_ms = self.lyric_offset_ms.clamp(-10_000, 10_000);
    }
    pub async fn save(&self) -> Result<()> {
        crate::cache::write(
            &dir().join("config.json"),
            &serde_json::to_vec_pretty(self)?,
        )
        .await?;
        crate::cache::write(
            &dir().join("surface-effects.kdl"),
            self.surface_effects().as_bytes(),
        )
        .await?;
        crate::cache::write(
            &dir().join("liquid-effects.kdl"),
            self.liquid_effects().as_bytes(),
        )
        .await
    }
    pub fn liquid_effects(&self) -> String {
        let radius = if self.liquid {
            self.liquid_radius
        } else {
            self.corner_radius
        };
        let strength = if self.liquid && self.theme == "glass" {
            self.refraction
        } else {
            0.0
        };
        format!(
            r##"// Only include from the optional niri-lyricglass compositor.
layer-rule {{
    match namespace="^lyricglass$"
    geometry-corner-radius {radius}
    background-effect {{
        blur true
        xray false
        noise 0
        saturation 1
        liquid-glass {{
            refraction-strength {strength:.3}
            refraction-power 0.8
            edge-thickness 0.22
            lens-distortion 0.12
            fringing 0.12
            edge-lighting {reflection:.3}
            brightness 1.0
            contrast 1.03
            saturation 1.08
            edge-padding 0
        }}
    }}
    shadow {{
        on
        softness 24
        spread 0
        offset x=0 y=6
        color "#00000045"
    }}
}}
"##,
            reflection = self.reflections
        )
    }
    pub fn surface_effects(&self) -> String {
        let radius = if self.liquid {
            self.liquid_radius
        } else {
            self.corner_radius
        };
        format!(
            r##"// Stock Niri-compatible geometry, updated by LyricGlass.
layer-rule {{
    match namespace="^lyricglass$"
    geometry-corner-radius {radius}
    background-effect {{ blur true; xray false; }}
    shadow {{ on; softness 24; spread 0; offset x=0 y=6; color "#00000045"; }}
}}
"##
        )
    }
}

pub fn prepare_liquid() -> Result<()> {
    let base = directories::BaseDirs::new().context("No home directory")?;
    let directory = dir();
    let preview = directory.join("preview");
    let preview_app = preview.join("lyricglass");
    let executable = std::env::current_exe()?;
    let scene = executable
        .parent()
        .context("No executable directory")?
        .join("examples/glass_scene");
    let quote = |path: &std::path::Path| serde_json::to_string(&path.to_string_lossy());
    let settings = Config::load();
    std::fs::create_dir_all(&preview_app)?;
    std::fs::write(
        directory.join("liquid-effects.kdl"),
        settings.liquid_effects(),
    )?;
    let preview_settings = Config {
        width: 580,
        square: false,
        compact: false,
        free_position: true,
        x: 320,
        y: 280,
        opacity: 0.3,
        font_size: 19,
        liquid: true,
        ..settings
    };
    std::fs::write(
        preview_app.join("config.json"),
        serde_json::to_vec_pretty(&preview_settings)?,
    )?;
    std::fs::write(
        preview_app.join("liquid-effects.kdl"),
        preview_settings.liquid_effects(),
    )?;
    let session = format!(
        "include {}\ninclude {}\n",
        quote(&base.config_dir().join("niri/config.kdl"))?,
        quote(&directory.join("liquid-effects.kdl"))?
    );
    let preview_config = format!(
        r##"layout {{ background-color "#141519"; }}
blur {{ passes 1; offset 1.0; noise 0; saturation 1.0; }}
hotkey-overlay {{ skip-at-startup; }}
include {effects}
spawn-at-startup {scene}
spawn-at-startup "env" {environment} {app} "demo"
binds {{
    Escape {{ quit skip-confirmation=true; }}
    F7 {{ spawn "env" {environment} {app} "toggle"; }}
    Shift+F7 {{ spawn "env" {environment} {app} "settings"; }}
}}
"##,
        effects = quote(&preview_app.join("liquid-effects.kdl"))?,
        scene = quote(&scene)?,
        environment = serde_json::to_string(&format!("XDG_CONFIG_HOME={}", preview.display()))?,
        app = quote(&executable)?
    );
    let binary = base.home_dir().join(".local/bin/niri-lyricglass");
    for (name, text) in [
        ("liquid-session.kdl", session),
        ("liquid-preview.kdl", preview_config),
    ] {
        let path = directory.join(name);
        std::fs::write(&path, text)?;
        let output = Command::new(&binary)
            .args(["validate", "--config"])
            .arg(&path)
            .output()
            .context("Missing niri-lyricglass: run compositor/build.sh")?;
        anyhow::ensure!(
            output.status.success(),
            "Configuration rejected: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

impl Shortcuts {
    pub fn pairs(&self) -> [(&str, &str); 6] {
        [
            (&self.toggle, "toggle"),
            (&self.settings, "settings"),
            (&self.game, "game-mode"),
            (&self.play_pause, "play-pause"),
            (&self.previous, "previous"),
            (&self.next, "next"),
        ]
    }
    pub fn render(&self, executable: &std::path::Path) -> Result<String> {
        let exe = serde_json::to_string(&executable.to_string_lossy())?;
        let mut text =
            String::from("// Managed by LyricGlass. Change keys in its settings.\nbinds {\n");
        let mut seen = std::collections::HashSet::new();
        for (key, action) in self.pairs() {
            if key.is_empty() {
                continue;
            }
            if !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'_')
            {
                bail!("Invalid shortcut: {key}");
            }
            if !seen.insert(key.to_ascii_lowercase()) {
                bail!("Duplicate shortcut: {key}");
            }
            text.push_str(&format!(
                "    {key} allow-inhibiting=false {{ spawn {exe} \"{action}\"; }}\n"
            ));
        }
        text.push_str("}\n");
        text.push_str(&format!(
            "include {}\n",
            serde_json::to_string(&dir().join("surface-effects.kdl").to_string_lossy())?
        ));
        Ok(text)
    }
}

/// Validate the entire user's configuration before committing an include.
pub fn install_shortcuts(shortcuts: &Shortcuts) -> Result<()> {
    let base = directories::BaseDirs::new().context("No home directory")?;
    let niri_dir = base.config_dir().join("niri");
    let config_path = niri_dir.join("config.kdl");
    let original =
        std::fs::read_to_string(&config_path).context("Niri configuration was not found")?;
    let target = dir().join("shortcuts.kdl");
    let include = format!(
        "include {}",
        serde_json::to_string(&target.to_string_lossy())?
    );
    let candidate = if original.lines().any(|line| line.trim() == include) {
        original.clone()
    } else {
        format!("{original}\n// LyricGlass global shortcuts\n{include}\n")
    };
    let snippet = shortcuts.render(&std::env::current_exe()?)?;
    std::fs::create_dir_all(dir())?;
    let effect_target = dir().join("surface-effects.kdl");
    let old_effects = std::fs::read(&effect_target).ok();
    std::fs::write(&effect_target, Config::load().surface_effects())?;
    let old_snippet = std::fs::read(&target).ok();
    std::fs::write(&target, snippet)?;
    let check_path = niri_dir.join(format!(".lyricglass-check-{}.kdl", std::process::id()));
    let result = (|| -> Result<()> {
        std::fs::write(&check_path, &candidate)?;
        let output = Command::new("niri")
            .arg("validate")
            .arg("--config")
            .arg(&check_path)
            .output()?;
        if !output.status.success() {
            bail!(
                "Niri rejected the shortcuts: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if candidate != original {
            std::fs::write(niri_dir.join("config.kdl.before-lyricglass"), &original)?;
            std::fs::rename(&check_path, &config_path)?;
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(check_path);
    if result.is_err() {
        if let Some(bytes) = old_effects {
            std::fs::write(&effect_target, bytes)?;
        } else {
            let _ = std::fs::remove_file(&effect_target);
        }
        if let Some(bytes) = old_snippet {
            std::fs::write(&target, bytes)?;
        } else {
            let _ = std::fs::remove_file(&target);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn liquid_settings_migrate_without_changing_layout() {
        let mut config: Config =
            serde_json::from_str(r#"{"width":360,"free_position":true,"x":1560,"opacity":0.3}"#)
                .unwrap();
        config.normalize();
        assert!(config.liquid);
        assert_eq!(config.width, 360);
        assert_eq!(config.x, 1560);
        assert_eq!(config.opacity, 0.3);
        assert!(
            config
                .liquid_effects()
                .contains("refraction-strength 6.000")
        );
        config.liquid = false;
        assert!(
            config
                .liquid_effects()
                .contains("refraction-strength 0.000")
        );
        config.reflections = f64::NAN;
        config.refraction = f64::INFINITY;
        config.liquid_radius = 999;
        config.normalize();
        assert!(config.reflections.is_finite());
        assert!(config.refraction.is_finite());
        assert_eq!(config.liquid_radius, 48);
    }
    #[test]
    fn settings_defaults_and_limits() {
        let mut config: Config = serde_json::from_str("{\"width\":1,\"opacity\":9}").unwrap();
        config.normalize();
        assert_eq!(config.width, 360);
        assert_eq!(config.opacity, 1.0);
        assert_eq!(config.shortcuts.toggle, "F7");
    }
    #[test]
    fn square_and_free_position_survive_restart_and_stay_on_screen() {
        let config = Config {
            square: true,
            square_size: 280,
            free_position: true,
            x: 1700,
            y: 1000,
            ..Config::default()
        };
        let mut restored: Config =
            serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        restored.clamp_position((1920, 1080), (280, 280));
        assert!(restored.square && restored.free_position);
        assert_eq!((restored.x, restored.y), (1640, 800));
        restored.clamp_position((200, 200), (280, 280));
        assert_eq!((restored.x, restored.y), (0, 0));
    }
    #[test]
    fn reject_duplicate_and_injected_keys() {
        let mut shortcuts = Shortcuts {
            toggle: "F7\"; quit;".into(),
            ..Shortcuts::default()
        };
        assert!(shortcuts.render(std::path::Path::new("/tmp/app")).is_err());
        shortcuts.toggle = shortcuts.next.clone();
        assert!(shortcuts.render(std::path::Path::new("/tmp/app")).is_err());
    }
}
