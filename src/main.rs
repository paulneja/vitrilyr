mod app;
mod backend;
mod i18n;
mod lyric_view;
mod material;
mod platform;
mod settings;
mod ui;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Copy, ValueEnum)]
pub enum Layout {
    Normal,
    Line,
    Square,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum MaterialMode {
    Liquid,
    Frosted,
}

#[derive(Parser)]
#[command(version, about = "Spotify lyrics on your Linux desktop")]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show or hide the overlay.
    Toggle,
    Show,
    Hide,
    /// Open live settings.
    Settings,
    /// Toggle mouse click-through.
    GameMode,
    /// Choose a layout.
    Layout {
        mode: Layout,
    },
    /// Change the glass material and refraction.
    Material {
        mode: MaterialMode,
        #[arg(long)]
        refraction: Option<f64>,
    },
    /// Position the overlay on the selected monitor.
    Position {
        x: i32,
        y: i32,
    },
    /// Enable free placement and dragging.
    Move,
    /// Restore a visible, clickable overlay without resetting its appearance.
    Recover,
    ResetPosition,
    PlayPause,
    Next,
    Previous,
    /// Seek by a number of seconds.
    Seek {
        #[arg(allow_hyphen_values = true)]
        seconds: f64,
    },
    /// Set Spotify volume from 0 to 100.
    Volume {
        percent: f64,
    },
    /// Print current state as JSON.
    Status,
    #[command(hide = true)]
    Capture {
        path: std::path::PathBuf,
    },
    #[command(hide = true)]
    CaptureSettings {
        path: std::path::PathBuf,
    },
    /// Install global shortcuts after validating Niri configuration.
    InstallShortcuts,
    /// Prepare the optional Niri Liquid Glass session and preview.
    PrepareLiquid,
    /// Install an application-menu entry for this executable.
    InstallDesktop,
    /// Choose the interface language (English is the default).
    Language {
        #[arg(value_parser = ["en", "es", "zh"])]
        language: String,
    },
    /// Choose an appearance style.
    Style {
        #[arg(value_parser = ["glass", "dark", "light", "contrast", "minimal"])]
        style: String,
    },
    /// Export preferences to a JSON file.
    ExportConfig {
        path: std::path::PathBuf,
    },
    /// Import preferences; does not change login startup.
    ImportConfig {
        path: std::path::PathBuf,
    },
    /// Enable or disable automatic startup at graphical login.
    Autostart {
        #[arg(value_parser = ["on", "off"])]
        state: String,
    },
    #[command(hide = true)]
    AutostartRun,
    /// Preview original sample lyrics without controlling Spotify.
    Demo,
    Quit,
}

fn main() -> gtk::glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vitrilyr=info".into()),
        )
        .init();
    let args = Args::parse();
    if matches!(args.command, Some(Command::InstallDesktop)) {
        return cli_result(vitrilyr::startup::install_desktop());
    }
    if let Some(Command::Autostart { state }) = &args.command {
        let result = (|| -> anyhow::Result<()> {
            let enabled = state == "on";
            vitrilyr::startup::set_enabled(enabled)?;
            let mut config = vitrilyr::startup::current_config()?;
            config.autostart = enabled;
            tokio::runtime::Runtime::new()?.block_on(config.save())?;
            vitrilyr::startup::notify_config(&config)?;
            Ok(())
        })();
        return cli_result(result);
    }
    if matches!(args.command, Some(Command::AutostartRun)) {
        return cli_result(vitrilyr::startup::run());
    }
    if let Some(Command::ExportConfig { path }) = &args.command {
        return cli_result((|| {
            tokio::runtime::Runtime::new()?
                .block_on(vitrilyr::startup::current_config()?.export(path))
        })());
    }
    if let Some(Command::ImportConfig { path }) = &args.command {
        return cli_result((|| -> anyhow::Result<()> {
            let runtime = tokio::runtime::Runtime::new()?;
            let mut config = runtime.block_on(vitrilyr::config::Config::read_from(path))?;
            config.autostart = vitrilyr::startup::current_config()?.autostart;
            runtime.block_on(config.save())?;
            vitrilyr::startup::notify_config(&config)?;
            Ok(())
        })());
    }
    if matches!(args.command, Some(Command::PrepareLiquid)) {
        return match vitrilyr::config::prepare_liquid() {
            Ok(()) => gtk::glib::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error:#}");
                gtk::glib::ExitCode::FAILURE
            }
        };
    }
    if matches!(args.command, Some(Command::InstallShortcuts)) {
        return match vitrilyr::config::install_shortcuts(
            &vitrilyr::config::Config::load().shortcuts,
        ) {
            Ok(()) => {
                println!("Shortcuts enabled in Niri");
                gtk::glib::ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error:#}");
                gtk::glib::ExitCode::FAILURE
            }
        };
    }
    app::run()
}

fn cli_result(result: anyhow::Result<()>) -> gtk::glib::ExitCode {
    match result {
        Ok(()) => gtk::glib::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            gtk::glib::ExitCode::FAILURE
        }
    }
}
