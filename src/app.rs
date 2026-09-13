use crate::i18n::tr;
use crate::{Args, Command, ui::Ui};
use clap::Parser;
use gtk::{gio, glib, prelude::*};
use lyricglass::state::MediaCommand;
use std::{cell::RefCell, rc::Rc};

pub fn run() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id("io.github.lyricglass.LyricGlass")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let current: Rc<RefCell<Option<Rc<Ui>>>> = Rc::new(RefCell::new(None));
    let config_action = gio::SimpleAction::new("apply-config", Some(glib::VariantTy::STRING));
    let target = current.clone();
    config_action.connect_activate(move |_, parameter| {
        if let Some(json) = parameter.and_then(|v| v.str())
            && let Ok(config) = lyricglass::config::Config::decode(json.as_bytes())
            && let Some(ui) = target.borrow().as_ref()
        {
            ui.change(|c| *c = config);
            let settings = ui.settings.borrow_mut().take();
            if let Some(window) = settings {
                window.close();
                crate::settings::open(ui);
            }
        }
    });
    app.add_action(&config_action);
    app.connect_command_line(move |app, cli| {
        let args = match Args::try_parse_from(cli.arguments()) {
            Ok(args) => args,
            Err(error) => {
                cli.printerr_literal(&error.to_string());
                return glib::ExitCode::FAILURE;
            }
        };
        if matches!(args.command, Some(Command::Quit)) {
            app.quit();
            return glib::ExitCode::SUCCESS;
        }
        let starting = current.borrow().is_none();
        if starting {
            match Ui::new(app, matches!(args.command, Some(Command::Demo))) {
                Ok(ui) => *current.borrow_mut() = Some(ui),
                Err(error) => {
                    cli.printerr_literal(&format!("{error:#}\n"));
                    app.quit();
                    return glib::ExitCode::FAILURE;
                }
            }
        }
        let ui = current
            .borrow()
            .as_ref()
            .cloned()
            .expect("UI initialized above");
        match args.command {
            Some(Command::Toggle) if starting => ui.set_visible(true),
            Some(Command::Toggle) => ui.toggle(),
            Some(Command::Hide) => ui.set_visible(false),
            None if starting && ui.config.borrow().start_hidden => ui.set_visible(false),
            Some(Command::Show) | None => ui.set_visible(true),
            Some(Command::Settings) => crate::settings::open(&ui),
            Some(Command::Language { language }) => {
                ui.change(|c| c.language = language);
                let settings = ui.settings.borrow_mut().take();
                if let Some(window) = settings {
                    window.close();
                    crate::settings::open(&ui);
                }
            }
            Some(Command::Style { style }) => ui.change(|c| c.theme = style),
            Some(Command::GameMode) => ui.game_mode(),
            Some(Command::Layout { mode }) => ui.change(|c| {
                c.square = matches!(mode, crate::Layout::Square);
                c.compact = matches!(mode, crate::Layout::Line);
            }),
            Some(Command::Position { x, y }) => ui.change(|c| {
                c.free_position = true;
                c.x = x;
                c.y = y;
            }),
            Some(Command::Move) => ui.place(),
            Some(Command::Material { mode, refraction }) => ui.change(|c| {
                c.liquid = matches!(mode, crate::MaterialMode::Liquid);
                if let Some(value) = refraction {
                    c.refraction = value;
                }
            }),
            Some(Command::ResetPosition) => ui.change(|c| {
                c.free_position = false;
                c.bottom = false;
            }),
            Some(Command::PlayPause) => ui.media(MediaCommand::PlayPause),
            Some(Command::Next) => ui.media(MediaCommand::Next),
            Some(Command::Previous) => ui.media(MediaCommand::Previous),
            Some(Command::Seek { seconds }) if seconds.is_finite() => {
                ui.media(MediaCommand::Seek(seconds))
            }
            Some(Command::Volume { percent })
                if percent.is_finite() && (0.0..=100.0).contains(&percent) =>
            {
                ui.media(MediaCommand::Volume(percent / 100.0))
            }
            Some(Command::Status) => cli.print_literal(&format!("{}\n", ui.status())),
            Some(Command::Capture { path }) => {
                if let Err(error) = ui.capture(&path, false) {
                    cli.printerr_literal(&format!("{error:#}\n"));
                    return glib::ExitCode::FAILURE;
                }
            }
            Some(Command::CaptureSettings { path }) => {
                if let Err(error) = ui.capture(&path, true) {
                    cli.printerr_literal(&format!("{error:#}\n"));
                    return glib::ExitCode::FAILURE;
                }
            }
            Some(Command::Demo) => {}
            _ => {
                cli.printerr_literal(tr("Invalid argument\n"));
                return glib::ExitCode::FAILURE;
            }
        }
        glib::ExitCode::SUCCESS
    });
    app.run()
}
