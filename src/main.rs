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
                .unwrap_or_else(|_| "lyricglass=info".into()),
        )
        .init();
    let args = Args::parse();
    if let Some(Command::Autostart { state }) = &args.command {
        let result = (|| -> anyhow::Result<()> {
            let enabled = state == "on";
            lyricglass::startup::set_enabled(enabled)?;
            let mut config = lyricglass::config::Config::load();
            config.autostart = enabled;
            tokio::runtime::Runtime::new()?.block_on(config.save())?;
            Ok(())
        })();
        return cli_result(result);
    }
    if matches!(args.command, Some(Command::AutostartRun)) {
        return cli_result(lyricglass::startup::run());
    }
    if matches!(args.command, Some(Command::PrepareLiquid)) {
        return match lyricglass::config::prepare_liquid() {
            Ok(()) => gtk::glib::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error:#}");
                gtk::glib::ExitCode::FAILURE
            }
        };
    }
    if matches!(args.command, Some(Command::InstallShortcuts)) {
        return match lyricglass::config::install_shortcuts(
            &lyricglass::config::Config::load().shortcuts,
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
