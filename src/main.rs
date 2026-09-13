mod app;
mod backend;
mod lyric_view;
mod material;
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
#[command(version, about = "Letras de Spotify sobre tu escritorio Wayland")]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Mostrar u ocultar el overlay.
    Toggle,
    Show,
    Hide,
    /// Abrir los ajustes en vivo.
    Settings,
    /// Alternar el paso de clics al juego.
    GameMode,
    /// Elegir normal, line o square.
    Layout {
        mode: Layout,
    },
    /// Cambiar el material y su refraccion sin abrir ajustes.
    Material {
        mode: MaterialMode,
        #[arg(long)]
        refraction: Option<f64>,
    },
    /// Colocar el overlay en coordenadas del monitor seleccionado.
    Position {
        x: i32,
        y: i32,
    },
    /// Activar la colocacion libre y permitir arrastrar.
    Move,
    ResetPosition,
    PlayPause,
    Next,
    Previous,
    /// Desplazar la reproduccion un numero de segundos.
    Seek {
        #[arg(allow_hyphen_values = true)]
        seconds: f64,
    },
    /// Establecer el volumen de Spotify de 0 a 100.
    Volume {
        percent: f64,
    },
    /// Mostrar el estado actual como JSON.
    Status,
    #[command(hide = true)]
    Capture {
        path: std::path::PathBuf,
    },
    #[command(hide = true)]
    CaptureSettings {
        path: std::path::PathBuf,
    },
    /// Instalar los atajos configurados, validando primero Niri.
    InstallShortcuts,
    /// Preparar una sesion optativa y una vista previa de Niri Liquid Glass.
    PrepareLiquid,
    /// Vista de prueba con letras originales; no controla Spotify.
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
                println!("Atajos activados en Niri");
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
