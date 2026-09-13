use crate::i18n::tr;
use crate::{backend::Preference, ui::Ui};
use gtk::{gdk, glib, prelude::*};
use lyricglass::{
    config::{Config, Shortcuts},
    state::MediaCommand,
};
use std::{cell::Cell, rc::Rc};

pub fn dialog(ui: &Ui, title: &str, width: i32, height: i32) -> gtk::Window {
    let window = gtk::Window::builder()
        .application(
            &ui.window
                .application()
                .expect("Window belongs to application"),
        )
        .title(title)
        .default_width(width)
        .default_height(height)
        .decorated(false)
        .resizable(false)
        .build();
    window.add_css_class("preferences");
    ui.platform.setup(&window, true);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = gtk::HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some(title))));
    header.set_show_title_buttons(false);
    let close = crate::ui::button("window-close-symbolic", tr("Close"));
    let weak = window.downgrade();
    close.connect_clicked(move |_| {
        if let Some(window) = weak.upgrade() {
            window.close();
        }
    });
    header.pack_end(&close);
    root.append(&header);
    window.set_child(Some(&root));
    let keys = gtk::EventControllerKey::new();
    let weak = window.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(window) = weak.upgrade() {
                window.close();
            }
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    window.add_controller(keys);
    window
}

pub fn set_dialog_content(window: &gtk::Window, widget: &impl IsA<gtk::Widget>) {
    if let Some(root) = window.child().and_then(|w| w.downcast::<gtk::Box>().ok()) {
        root.append(widget);
    }
}

pub fn open(ui: &Rc<Ui>) {
    if let Some(window) = ui.settings.borrow().as_ref() {
        window.present();
        return;
    }
    let config = ui.config.borrow().clone();
    let window = dialog(ui, tr("LyricGlass · Settings"), 500, 680);
    let pages = gtk::Stack::new();
    pages.set_vexpand(true);
    pages.set_vhomogeneous(false);
    let navigation = gtk::StackSwitcher::builder()
        .stack(&pages)
        .halign(gtk::Align::Center)
        .build();
    navigation.set_margin_top(12);
    navigation.set_margin_bottom(8);
    let layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
    layout.append(&navigation);
    layout.append(&pages);
    let content = page(&pages, "appearance", tr("Appearance"));
    section(&content, tr("Appearance"));
    let style = gtk::DropDown::from_strings(&[
        tr("Glass"),
        tr("Dark"),
        tr("Light"),
        tr("High contrast"),
        tr("Minimal"),
    ]);
    let styles = ["glass", "dark", "light", "contrast", "minimal"];
    style.set_selected(styles.iter().position(|s| *s == config.theme).unwrap_or(0) as u32);
    let weak = Rc::downgrade(ui);
    style.connect_selected_notify(move |dropdown| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| {
                c.theme = styles
                    .get(dropdown.selected() as usize)
                    .unwrap_or(&"glass")
                    .to_string()
            });
        }
    });
    row(&content, tr("Style"), &style);
    let materials = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    materials.add_css_class("linked");
    let frosted = gtk::ToggleButton::with_label(tr("Frosted"));
    let liquid = gtk::ToggleButton::with_label(tr("Liquid Glass"));
    liquid.set_group(Some(&frosted));
    frosted.set_active(!config.liquid);
    liquid.set_active(config.liquid);
    let weak = Rc::downgrade(ui);
    liquid.connect_toggled(move |button| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| c.liquid = button.is_active());
        }
    });
    materials.append(&frosted);
    materials.append(&liquid);
    row(&content, tr("Material"), &materials);
    slider(
        ui,
        &content,
        tr("Reflections"),
        config.reflections * 100.0,
        0.0,
        100.0,
        1.0,
        |c, v| c.reflections = v / 100.0,
    );
    slider(
        ui,
        &content,
        tr("Glass curvature"),
        f64::from(config.liquid_radius),
        12.0,
        48.0,
        1.0,
        |c, v| c.liquid_radius = v as i32,
    );
    slider(
        ui,
        &content,
        tr("Refraction · Niri Liquid"),
        config.refraction,
        0.0,
        16.0,
        0.5,
        |c, v| c.refraction = v,
    );
    let modes = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    modes.add_css_class("linked");
    let mut first: Option<gtk::ToggleButton> = None;
    for (index, name) in [tr("Normal"), tr("Line"), tr("Square")]
        .into_iter()
        .enumerate()
    {
        let mode = gtk::ToggleButton::with_label(name);
        if let Some(first) = &first {
            mode.set_group(Some(first));
        } else {
            first = Some(mode.clone());
        }
        mode.set_active(
            index
                == if config.square {
                    2
                } else {
                    usize::from(config.compact)
                },
        );
        let weak = Rc::downgrade(ui);
        mode.connect_toggled(move |button| {
            if button.is_active()
                && let Some(ui) = weak.upgrade()
            {
                ui.change(|c| {
                    c.square = index == 2;
                    c.compact = index == 1;
                });
            }
        });
        modes.append(&mode);
    }
    row(&content, tr("Layout"), &modes);
    slider(
        ui,
        &content,
        tr("Square size"),
        f64::from(config.square_size),
        240.0,
        440.0,
        10.0,
        |c, v| c.square_size = v as i32,
    );
    slider(
        ui,
        &content,
        tr("Width"),
        f64::from(config.width),
        360.0,
        900.0,
        10.0,
        |c, v| c.width = v as i32,
    );
    slider(
        ui,
        &content,
        tr("Opacity"),
        config.opacity * 100.0,
        30.0,
        100.0,
        1.0,
        |c, v| c.opacity = v / 100.0,
    );
    slider(
        ui,
        &content,
        tr("Lyric size"),
        f64::from(config.font_size),
        14.0,
        28.0,
        1.0,
        |c, v| c.font_size = v as i32,
    );
    slider(
        ui,
        &content,
        tr("Edge distance"),
        f64::from(config.margin),
        0.0,
        240.0,
        2.0,
        |c, v| c.margin = v as i32,
    );
    slider(
        ui,
        &content,
        tr("Corners"),
        f64::from(config.corner_radius),
        0.0,
        24.0,
        1.0,
        |c, v| c.corner_radius = v as i32,
    );
    let colors = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mut first: Option<gtk::ToggleButton> = None;
    for (name, title) in [
        ("mint", tr("Mint")),
        ("blue", tr("Blue")),
        ("rose", tr("Rose")),
        ("gold", tr("Gold")),
    ] {
        let swatch = gtk::ToggleButton::new();
        swatch.set_tooltip_text(Some(title));
        swatch.add_css_class("swatch");
        swatch.add_css_class(name);
        if let Some(first) = &first {
            swatch.set_group(Some(first));
        } else {
            first = Some(swatch.clone());
        }
        swatch.set_active(config.accent == name);
        let weak = Rc::downgrade(ui);
        swatch.connect_toggled(move |b| {
            if b.is_active()
                && let Some(ui) = weak.upgrade()
            {
                ui.change(|c| c.accent = name.into());
            }
        });
        colors.append(&swatch);
    }
    row(&content, tr("Accent"), &colors);
    let custom = gtk::ColorDialogButton::new(Some(gtk::ColorDialog::new()));
    custom.set_rgba(&gdk::RGBA::parse(&config.custom_accent).expect("Normalized accent"));
    let weak = Rc::downgrade(ui);
    custom.connect_rgba_notify(move |button| {
        if let Some(ui) = weak.upgrade() {
            let rgba = button.rgba();
            ui.change(|c| {
                c.accent = "custom".into();
                c.custom_accent = format!(
                    "#{:02x}{:02x}{:02x}",
                    (rgba.red() * 255.0).round() as u8,
                    (rgba.green() * 255.0).round() as u8,
                    (rgba.blue() * 255.0).round() as u8
                );
            });
        }
    });
    row(&content, tr("Custom color"), &custom);
    let font = gtk::FontDialogButton::new(Some(gtk::FontDialog::new()));
    let mut description = gtk::pango::FontDescription::new();
    description.set_family(&config.font_family);
    font.set_font_desc(&description);
    font.set_size_request(210, -1);
    let weak = Rc::downgrade(ui);
    font.connect_font_desc_notify(move |button| {
        if let Some(ui) = weak.upgrade()
            && let Some(family) = button.font_desc().and_then(|d| d.family())
        {
            ui.change(|c| c.font_family = family.to_string());
        }
    });
    row(&content, tr("Font family"), &font);
    slider(
        ui,
        &content,
        tr("Line spacing"),
        f64::from(config.line_spacing),
        4.0,
        20.0,
        1.0,
        |c, v| c.line_spacing = v as i32,
    );
    slider(
        ui,
        &content,
        tr("Transition (ms)"),
        f64::from(config.transition_ms),
        100.0,
        400.0,
        10.0,
        |c, v| c.transition_ms = v as i32,
    );
    slider(
        ui,
        &content,
        tr("Secondary opacity"),
        config.secondary_opacity * 100.0,
        10.0,
        75.0,
        1.0,
        |c, v| c.secondary_opacity = v / 100.0,
    );
    toggle(
        ui,
        &content,
        tr("Lyrics only"),
        config.lyrics_only,
        |c, v| c.lyrics_only = v,
    );
    toggle(
        ui,
        &content,
        tr("Neighboring lines"),
        config.show_context,
        |c, v| c.show_context = v,
    );
    toggle(ui, &content, tr("Artwork"), config.show_artwork, |c, v| {
        c.show_artwork = v
    });
    toggle(
        ui,
        &content,
        tr("Playback controls"),
        config.show_controls,
        |c, v| c.show_controls = v,
    );
    toggle(
        ui,
        &content,
        tr("Progress bar"),
        config.show_progress,
        |c, v| c.show_progress = v,
    );
    toggle(ui, &content, tr("Animations"), config.animations, |c, v| {
        c.animations = v
    });
    let reset = gtk::Button::with_label(tr("Reset"));
    ui.on_button(&reset, |ui| {
        ui.change(|c| {
            let defaults = Config::default();
            c.theme = defaults.theme;
            c.font_family = defaults.font_family;
            c.opacity = defaults.opacity;
            c.font_size = defaults.font_size;
            c.line_spacing = defaults.line_spacing;
            c.transition_ms = defaults.transition_ms;
            c.secondary_opacity = defaults.secondary_opacity;
            c.liquid = defaults.liquid;
            c.liquid_radius = defaults.liquid_radius;
            c.reflections = defaults.reflections;
            c.refraction = defaults.refraction;
            c.corner_radius = defaults.corner_radius;
            c.accent = defaults.accent;
            c.custom_accent = defaults.custom_accent;
            c.show_artwork = true;
            c.show_controls = true;
            c.show_progress = true;
            c.lyrics_only = false;
            c.show_context = true;
        });
        reopen(ui);
    });
    row(&content, tr("Reset appearance"), &reset);
    let content = page(&pages, "usage", tr("Behavior"));
    section(&content, tr("Startup"));
    let backup = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (save, icon, title) in [
        (true, "document-save-symbolic", "Export settings"),
        (false, "document-open-symbolic", "Import settings"),
    ] {
        let button = crate::ui::button(icon, tr(title));
        ui.on_button(&button, move |ui| {
            let chooser = gtk::FileDialog::builder()
                .title(tr(title))
                .modal(true)
                .build();
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("JSON"));
            filter.add_pattern("*.json");
            let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            chooser.set_filters(Some(&filters));
            if save {
                chooser.set_initial_name(Some("lyricglass-settings.json"));
            }
            let weak = Rc::downgrade(ui);
            let parent = ui.window.clone();
            glib::MainContext::default().spawn_local(async move {
                let result = if save {
                    chooser.save_future(Some(&parent)).await
                } else {
                    chooser.open_future(Some(&parent)).await
                };
                if let Ok(file) = result
                    && let Some(path) = file.path()
                    && let Some(ui) = weak.upgrade()
                {
                    let preference = if save {
                        Preference::Export(path, ui.config.borrow().clone())
                    } else {
                        Preference::Import(path)
                    };
                    let _ = ui.backend.preferences.send(preference);
                }
            });
        });
        backup.append(&button);
    }
    row(&content, tr("Settings file"), &backup);
    let language = gtk::DropDown::from_strings(&["English", "Español", "简体中文"]);
    language.set_selected(match config.language.as_str() {
        "es" => 1,
        "zh" => 2,
        _ => 0,
    });
    let weak = Rc::downgrade(ui);
    language.connect_selected_notify(move |dropdown| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| {
                c.language = match dropdown.selected() {
                    1 => "es",
                    2 => "zh",
                    _ => "en",
                }
                .into()
            });
            let weak = Rc::downgrade(&ui);
            glib::idle_add_local_once(move || {
                if let Some(ui) = weak.upgrade() {
                    let window = ui.settings.borrow_mut().take();
                    if let Some(window) = window {
                        window.close();
                    }
                    open(&ui);
                }
            });
        }
    });
    row(&content, tr("Language"), &language);
    let startup = gtk::Switch::builder().active(config.autostart).build();
    let weak = Rc::downgrade(ui);
    startup.connect_active_notify(move |switch| {
        if let Some(ui) = weak.upgrade() {
            let mut config = ui.config.borrow().clone();
            config.autostart = switch.is_active();
            let _ = ui.backend.preferences.send(Preference::Startup(config));
        }
    });
    row(&content, tr("Start at login"), &startup);
    toggle(
        ui,
        &content,
        tr("Start hidden"),
        config.start_hidden,
        |c, v| c.start_hidden = v,
    );
    row(
        &content,
        tr("Window backend"),
        &gtk::Label::new(Some(ui.platform.name())),
    );
    section(&content, tr("Position"));
    let position = gtk::DropDown::from_strings(&[tr("Top"), tr("Bottom"), tr("Free")]);
    position.set_selected(if config.free_position {
        2
    } else {
        u32::from(config.bottom)
    });
    let weak = Rc::downgrade(ui);
    position.connect_selected_notify(move |d| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| {
                c.bottom = d.selected() == 1;
                c.free_position = d.selected() == 2;
            });
        }
    });
    row(&content, tr("Position"), &position);
    let place = gtk::Button::with_label(tr("Place freely"));
    ui.on_button(&place, |ui| ui.place());
    row(&content, tr("Drag"), &place);
    let reset = gtk::Button::with_label(tr("Center at top"));
    ui.on_button(&reset, |ui| {
        ui.change(|c| {
            c.free_position = false;
            c.bottom = false;
        })
    });
    row(&content, tr("Reset position"), &reset);
    let monitors = WidgetExt::display(&ui.window).monitors();
    let mut names = vec![tr("Automatic").to_string()];
    for i in 0..monitors.n_items() {
        if let Some(monitor) = monitors
            .item(i)
            .and_then(|m| m.downcast::<gdk::Monitor>().ok())
            && let Some(name) = monitor.connector()
        {
            names.push(name.to_string());
        }
    }
    let display =
        gtk::DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
    display.set_selected(names.iter().position(|n| *n == config.monitor).unwrap_or(0) as u32);
    let weak = Rc::downgrade(ui);
    display.connect_selected_notify(move |d| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| {
                c.monitor = if d.selected() == 0 {
                    String::new()
                } else {
                    names
                        .get(d.selected() as usize)
                        .cloned()
                        .unwrap_or_default()
                }
            });
        }
    });
    row(&content, tr("Monitor"), &display);
    section(&content, tr("While gaming"));
    toggle(
        ui,
        &content,
        tr("Lock position"),
        config.lock_position,
        |c, v| c.lock_position = v,
    );
    toggle(
        ui,
        &content,
        tr("Hide when paused"),
        config.hide_paused,
        |c, v| c.hide_paused = v,
    );
    toggle(
        ui,
        &content,
        tr("Click through"),
        config.click_through,
        |c, v| c.click_through = v,
    );
    toggle(
        ui,
        &content,
        tr("Hide when idle"),
        config.hide_idle,
        |c, v| c.hide_idle = v,
    );
    let visibility = gtk::Button::with_label(tr("Show / hide"));
    ui.on_button(&visibility, |ui| ui.toggle());
    row(&content, tr("Overlay"), &visibility);
    section(&content, tr("Playback"));
    let media = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (icon, title, command) in [
        (
            "media-skip-backward-symbolic",
            tr("Previous"),
            MediaCommand::Previous,
        ),
        (
            "media-seek-backward-symbolic",
            tr("Back 10 seconds"),
            MediaCommand::Seek(-10.0),
        ),
        (
            "media-playback-start-symbolic",
            tr("Play / pause"),
            MediaCommand::PlayPause,
        ),
        (
            "media-seek-forward-symbolic",
            tr("Forward 10 seconds"),
            MediaCommand::Seek(10.0),
        ),
        (
            "media-skip-forward-symbolic",
            tr("Next"),
            MediaCommand::Next,
        ),
    ] {
        let button = crate::ui::button(icon, title);
        ui.on_button(&button, move |ui| ui.media(command.clone()));
        media.append(&button);
    }
    row(&content, "Spotify", &media);
    let volume = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    volume.set_size_request(200, -1);
    volume.set_draw_value(true);
    volume.set_digits(0);
    volume.set_value(ui.volume().unwrap_or(0.5) * 100.0);
    volume.set_sensitive(ui.volume().is_some());
    let serial = Rc::new(Cell::new(0u64));
    let weak = Rc::downgrade(ui);
    volume.connect_value_changed(move |scale| {
        let serial = serial.clone();
        serial.set(serial.get() + 1);
        let current = serial.get();
        let value = scale.value() / 100.0;
        let weak = weak.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(180), move || {
            if serial.get() == current
                && let Some(ui) = weak.upgrade()
            {
                ui.media(MediaCommand::Volume(value));
            }
        });
    });
    row(&content, tr("Spotify volume"), &volume);
    let offset = gtk::SpinButton::with_range(-10000.0, 10000.0, 100.0);
    offset.set_value(f64::from(config.lyric_offset_ms));
    offset.set_tooltip_text(Some(tr("Milliseconds; positive advances lyrics")));
    let weak = Rc::downgrade(ui);
    offset.connect_value_changed(move |spin| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| c.lyric_offset_ms = spin.value_as_int());
        }
    });
    row(&content, tr("Lyric offset (ms)"), &offset);
    let retry = gtk::Button::with_label(tr("Search again"));
    ui.on_button(&retry, |ui| ui.retry());
    row(&content, tr("Lyrics"), &retry);
    let content = page(&pages, "shortcuts", tr("Shortcuts"));
    section(&content, tr("Global shortcuts"));
    let entries: Vec<gtk::Entry> = config
        .shortcuts
        .pairs()
        .iter()
        .zip([
            tr("Show / hide"),
            tr("Open settings"),
            tr("Game mode"),
            tr("Play / pause"),
            tr("Previous"),
            tr("Next"),
        ])
        .map(|((key, _), title)| {
            let entry = gtk::Entry::builder()
                .text(*key)
                .width_chars(16)
                .max_width_chars(20)
                .build();
            row(&content, title, &entry);
            entry
        })
        .collect();
    let niri = std::env::var_os("NIRI_SOCKET").is_some();
    let install = gtk::Button::with_label(tr(if niri {
        "Apply shortcuts in Niri"
    } else {
        "Save shortcuts"
    }));
    install.add_css_class("suggested-action");
    ui.on_button(&install, move |ui| {
        let values: Vec<String> = entries
            .iter()
            .map(|e| e.text().trim().to_string())
            .collect();
        let mut config = ui.config.borrow().clone();
        config.shortcuts = Shortcuts {
            toggle: values[0].clone(),
            settings: values[1].clone(),
            game: values[2].clone(),
            play_pause: values[3].clone(),
            previous: values[4].clone(),
            next: values[5].clone(),
        };
        if let Err(error) = config.shortcuts.render(std::path::Path::new("lyricglass")) {
            ui.notice.set_text(&error.to_string());
            return;
        }
        if niri {
            ui.notice.set_text(tr("Validating shortcuts with Niri..."));
            let _ = ui.backend.preferences.send(Preference::Install(config));
        } else {
            if let Err(error) = ui.platform.shortcuts(&config.shortcuts) {
                ui.notice.set_text(&error.to_string());
                return;
            }
            ui.change(|c| c.shortcuts = config.shortcuts);
        }
    });
    content.append(&install);
    if ui.notice.parent().is_some() {
        ui.notice.unparent();
    }
    ui.notice.set_margin_top(12);
    content.append(&ui.notice);
    let quit = gtk::Button::with_label(tr("Quit LyricGlass"));
    quit.set_margin_top(16);
    ui.on_button(&quit, |ui| {
        if let Some(app) = ui.window.application() {
            app.quit();
        }
    });
    content.append(&quit);
    set_dialog_content(&window, &layout);
    let weak = Rc::downgrade(ui);
    window.connect_close_request(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.settings.borrow_mut().take();
        }
        glib::Propagation::Proceed
    });
    *ui.settings.borrow_mut() = Some(window.clone());
    window.present();
}

fn reopen(ui: &Rc<Ui>) {
    let weak = Rc::downgrade(ui);
    glib::idle_add_local_once(move || {
        if let Some(ui) = weak.upgrade() {
            let window = ui.settings.borrow_mut().take();
            if let Some(window) = window {
                window.close();
            }
            open(&ui);
        }
    });
}

fn page(stack: &gtk::Stack, name: &str, title: &str) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_bottom(20);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&content)
        .build();
    stack.add_titled(&scroll, Some(name), title);
    content
}
fn section(content: &gtk::Box, title: &str) {
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("section-title");
    content.append(&label);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
}
fn row(content: &gtk::Box, title: &str, control: &impl IsA<gtk::Widget>) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.add_css_class("setting-row");
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_wrap(true);
    control.set_valign(gtk::Align::Center);
    row.append(&label);
    row.append(control);
    content.append(&row);
}
#[allow(clippy::too_many_arguments)]
fn slider(
    ui: &Rc<Ui>,
    content: &gtk::Box,
    title: &str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    update: fn(&mut Config, f64),
) {
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, step);
    scale.set_size_request(210, -1);
    scale.set_draw_value(true);
    scale.set_digits(0);
    scale.set_value(value);
    let weak = Rc::downgrade(ui);
    scale.connect_value_changed(move |s| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| update(c, s.value()));
        }
    });
    row(content, title, &scale);
}
fn toggle(
    ui: &Rc<Ui>,
    content: &gtk::Box,
    title: &str,
    value: bool,
    update: fn(&mut Config, bool),
) {
    let switch = gtk::Switch::builder().active(value).build();
    let weak = Rc::downgrade(ui);
    switch.connect_active_notify(move |s| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| update(c, s.is_active()));
        }
    });
    row(content, title, &switch);
}
