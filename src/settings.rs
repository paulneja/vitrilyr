use crate::{backend::Preference, ui::Ui};
use gtk::{gdk, glib, prelude::*};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
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
    window.init_layer_shell();
    window.set_namespace(Some("lyricglass-settings"));
    window.set_layer(Layer::Overlay);
    window.set_exclusive_zone(0);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = gtk::HeaderBar::new();
    header.set_title_widget(Some(&gtk::Label::new(Some(title))));
    header.set_show_title_buttons(false);
    let close = crate::ui::button("window-close-symbolic", "Cerrar");
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
    let window = dialog(ui, "LyricGlass · Ajustes", 500, 680);
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
    let content = page(&pages, "appearance", "Aspecto");
    section(&content, "Apariencia");
    let materials = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    materials.add_css_class("linked");
    let frosted = gtk::ToggleButton::with_label("Esmerilado");
    let liquid = gtk::ToggleButton::with_label("Liquid Glass");
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
    row(&content, "Material", &materials);
    slider(
        ui,
        &content,
        "Reflejos",
        config.reflections * 100.0,
        0.0,
        100.0,
        1.0,
        |c, v| c.reflections = v / 100.0,
    );
    slider(
        ui,
        &content,
        "Curvatura del vidrio",
        f64::from(config.liquid_radius),
        12.0,
        48.0,
        1.0,
        |c, v| c.liquid_radius = v as i32,
    );
    slider(
        ui,
        &content,
        "Refracción · Niri Liquid",
        config.refraction,
        0.0,
        16.0,
        0.5,
        |c, v| c.refraction = v,
    );
    let modes = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    modes.add_css_class("linked");
    let mut first: Option<gtk::ToggleButton> = None;
    for (index, name) in ["Normal", "Línea", "Cuadrado"].into_iter().enumerate() {
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
    row(&content, "Diseño", &modes);
    slider(
        ui,
        &content,
        "Lado del cuadrado",
        f64::from(config.square_size),
        240.0,
        440.0,
        10.0,
        |c, v| c.square_size = v as i32,
    );
    slider(
        ui,
        &content,
        "Ancho",
        f64::from(config.width),
        360.0,
        900.0,
        10.0,
        |c, v| c.width = v as i32,
    );
    slider(
        ui,
        &content,
        "Opacidad",
        config.opacity * 100.0,
        30.0,
        100.0,
        1.0,
        |c, v| c.opacity = v / 100.0,
    );
    slider(
        ui,
        &content,
        "Tamaño de letra",
        f64::from(config.font_size),
        14.0,
        28.0,
        1.0,
        |c, v| c.font_size = v as i32,
    );
    slider(
        ui,
        &content,
        "Distancia al borde",
        f64::from(config.margin),
        0.0,
        240.0,
        2.0,
        |c, v| c.margin = v as i32,
    );
    slider(
        ui,
        &content,
        "Esquinas",
        f64::from(config.corner_radius),
        0.0,
        24.0,
        1.0,
        |c, v| c.corner_radius = v as i32,
    );
    let colors = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mut first: Option<gtk::ToggleButton> = None;
    for (name, title) in [
        ("mint", "Menta"),
        ("blue", "Azul"),
        ("rose", "Rosa"),
        ("gold", "Dorado"),
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
    row(&content, "Acento", &colors);
    toggle(ui, &content, "Carátula", config.show_artwork, |c, v| {
        c.show_artwork = v
    });
    toggle(
        ui,
        &content,
        "Controles de música",
        config.show_controls,
        |c, v| c.show_controls = v,
    );
    toggle(
        ui,
        &content,
        "Barra de progreso",
        config.show_progress,
        |c, v| c.show_progress = v,
    );
    toggle(ui, &content, "Animaciones", config.animations, |c, v| {
        c.animations = v
    });
    let content = page(&pages, "usage", "Uso");
    section(&content, "Posición");
    let position = gtk::DropDown::from_strings(&["Arriba", "Abajo", "Libre"]);
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
    row(&content, "Posición", &position);
    let place = gtk::Button::with_label("Colocar libremente");
    ui.on_button(&place, |ui| ui.place());
    row(&content, "Arrastrar", &place);
    let reset = gtk::Button::with_label("Centrar arriba");
    ui.on_button(&reset, |ui| {
        ui.change(|c| {
            c.free_position = false;
            c.bottom = false;
        })
    });
    row(&content, "Restablecer posición", &reset);
    let monitors = WidgetExt::display(&ui.window).monitors();
    let mut names = vec!["Automático".to_string()];
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
    row(&content, "Pantalla", &display);
    section(&content, "Durante el juego");
    toggle(
        ui,
        &content,
        "Dejar pasar los clics",
        config.click_through,
        |c, v| c.click_through = v,
    );
    toggle(
        ui,
        &content,
        "Ocultar cuando no hay musica",
        config.hide_idle,
        |c, v| c.hide_idle = v,
    );
    let visibility = gtk::Button::with_label("Mostrar / ocultar");
    ui.on_button(&visibility, |ui| ui.toggle());
    row(&content, "Overlay", &visibility);
    section(&content, "Reproduccion");
    let media = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (icon, title, command) in [
        (
            "media-skip-backward-symbolic",
            "Anterior",
            MediaCommand::Previous,
        ),
        (
            "media-seek-backward-symbolic",
            "Retroceder 10 segundos",
            MediaCommand::Seek(-10.0),
        ),
        (
            "media-playback-start-symbolic",
            "Reproducir / pausar",
            MediaCommand::PlayPause,
        ),
        (
            "media-seek-forward-symbolic",
            "Adelantar 10 segundos",
            MediaCommand::Seek(10.0),
        ),
        (
            "media-skip-forward-symbolic",
            "Siguiente",
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
    row(&content, "Volumen de Spotify", &volume);
    let offset = gtk::SpinButton::with_range(-10000.0, 10000.0, 100.0);
    offset.set_value(f64::from(config.lyric_offset_ms));
    offset.set_tooltip_text(Some("Milisegundos; positivo adelanta las letras"));
    let weak = Rc::downgrade(ui);
    offset.connect_value_changed(move |spin| {
        if let Some(ui) = weak.upgrade() {
            ui.change(|c| c.lyric_offset_ms = spin.value_as_int());
        }
    });
    row(&content, "Desfase de letra (ms)", &offset);
    let retry = gtk::Button::with_label("Volver a buscar");
    ui.on_button(&retry, |ui| ui.retry());
    row(&content, "Letras", &retry);
    let content = page(&pages, "shortcuts", "Atajos");
    section(&content, "Atajos globales");
    let entries: Vec<gtk::Entry> = config
        .shortcuts
        .pairs()
        .iter()
        .zip([
            "Mostrar / ocultar",
            "Abrir ajustes",
            "Modo de juego",
            "Reproducir / pausar",
            "Anterior",
            "Siguiente",
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
    let install = gtk::Button::with_label("Aplicar atajos en Niri");
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
        ui.notice.set_text("Validando atajos con Niri...");
        let _ = ui.backend.preferences.send(Preference::Install(config));
    });
    content.append(&install);
    if ui.notice.parent().is_some() {
        ui.notice.unparent();
    }
    ui.notice.set_margin_top(12);
    content.append(&ui.notice);
    let quit = gtk::Button::with_label("Salir de LyricGlass");
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
