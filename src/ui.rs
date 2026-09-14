use crate::i18n::tr;
use crate::{
    backend::{Backend, Event, Load, Preference},
    lyric_view::LyricView,
};
use gtk::{gdk, gio, glib, prelude::*};
#[cfg(feature = "wayland")]
use gtk4_layer_shell::LayerShell;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};
use vitrilyr::{
    config::Config,
    lyrics::Lyrics,
    player::PlayerEvent,
    state::{MediaCommand, Playback, Snapshot, Track},
};

pub struct Ui {
    pub platform: crate::platform::Platform,
    pub window: gtk::ApplicationWindow,
    pub config: RefCell<Config>,
    pub backend: Backend,
    pub settings: RefCell<Option<gtk::Window>>,
    pub settings_page: RefCell<String>,
    pub notice: gtk::Label,
    snapshot: RefCell<Option<Snapshot>>,
    lyrics: RefCell<Option<Lyrics>>,
    generation: Cell<u64>,
    visible: Cell<bool>,
    demo: bool,
    title: gtk::Label,
    artist: gtk::Label,
    source: gtk::Label,
    art: gtk::Picture,
    placeholder: gtk::Image,
    header: gtk::Box,
    metadata: gtk::Box,
    controls_box: gtk::Box,
    toolbar_slot: gtk::Box,
    cover: gtk::Overlay,
    move_handle: gtk::Image,
    transport: Vec<gtk::Button>,
    dragging: Cell<bool>,
    play: gtk::Button,
    next: gtk::Button,
    previous: gtk::Button,
    progress: gtk::Scale,
    seek_pending: RefCell<Option<(f64, Instant)>>,
    view: LyricView,
    material: crate::material::Material,
    css: gtk::CssProvider,
    _hold: gio::ApplicationHoldGuard,
}

pub fn button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .focusable(false)
        .build()
}

impl Ui {
    pub fn new(app: &gtk::Application, demo: bool) -> anyhow::Result<Rc<Self>> {
        let config = Config::load();
        crate::i18n::set_language(&config.language);
        let platform = crate::platform::Platform::detect();
        let (backend, events) = Backend::start()?;
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Vitrilyr")
            .decorated(false)
            .resizable(false)
            .build();
        window.add_css_class("vitrilyr");
        platform.setup(&window, false);
        let glass = gtk::Box::new(gtk::Orientation::Vertical, 4);
        glass.add_css_class("glass");
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let cover = gtk::Overlay::new();
        cover.set_size_request(56, 56);
        cover.set_halign(gtk::Align::Start);
        cover.set_valign(gtk::Align::Center);
        cover.set_overflow(gtk::Overflow::Hidden);
        cover.add_css_class("cover");
        let art = gtk::Picture::new();
        art.set_content_fit(gtk::ContentFit::Cover);
        art.set_can_shrink(true);
        art.set_size_request(56, 56);
        cover.set_child(Some(&art));
        let placeholder = gtk::Image::from_icon_name("audio-x-generic-symbolic");
        placeholder.set_pixel_size(24);
        cover.add_overlay(&placeholder);
        header.append(&cover);
        let metadata = gtk::Box::new(gtk::Orientation::Vertical, 2);
        metadata.set_hexpand(true);
        metadata.set_valign(gtk::Align::Center);
        let title = label("Vitrilyr", "track-title");
        let artist = label("Spotify", "artist");
        let source = label(tr("Disconnected"), "source");
        for item in [&title, &artist, &source] {
            metadata.append(item);
        }
        header.append(&metadata);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        controls.set_valign(gtk::Align::Center);
        let previous = button("media-skip-backward-symbolic", tr("Previous"));
        let play = button("media-playback-start-symbolic", tr("Play / pause"));
        play.add_css_class("play");
        let next = button("media-skip-forward-symbolic", tr("Next"));
        let full_lyrics = button("view-list-symbolic", tr("Full lyrics"));
        let settings = button("emblem-system-symbolic", tr("Settings"));
        let hide = button("window-minimize-symbolic", tr("Hide"));
        for item in [&previous, &play, &next, &full_lyrics, &settings, &hide] {
            controls.append(item);
        }
        let move_handle = gtk::Image::from_icon_name("list-drag-handle-symbolic");
        move_handle.add_css_class("drag-handle");
        move_handle.set_pixel_size(16);
        move_handle.set_size_request(28, 28);
        move_handle.set_cursor_from_name(Some("grab"));
        move_handle.set_tooltip_text(Some(tr("Drag to position")));
        controls.append(&move_handle);
        header.append(&controls);
        glass.append(&header);
        let toolbar_slot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        toolbar_slot.set_halign(gtk::Align::Center);
        toolbar_slot.set_visible(false);
        glass.append(&toolbar_slot);
        let view = LyricView::new();
        glass.append(&view.widget);
        let progress = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.1);
        progress.set_draw_value(false);
        progress.set_focusable(false);
        progress.set_sensitive(false);
        progress.set_tooltip_text(Some(tr("Playback position")));
        glass.append(&progress);
        let surface = gtk::Overlay::new();
        let material = crate::material::Material::new(&surface);
        surface.set_child(Some(&material.widget));
        surface.add_overlay(&glass);
        surface.set_measure_overlay(&glass, true);
        window.set_child(Some(&surface));
        let css = gtk::CssProvider::new();
        let display = WidgetExt::display(&window);
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let notice = gtk::Label::new(None);
        notice.set_wrap(true);
        notice.set_xalign(0.0);
        notice.add_css_class("status");
        let ui = Rc::new(Self {
            platform,
            window,
            backend,
            config: RefCell::new(config),
            settings: RefCell::new(None),
            settings_page: RefCell::new("appearance".into()),
            notice,
            snapshot: RefCell::new(None),
            lyrics: RefCell::new(None),
            generation: Cell::new(0),
            visible: Cell::new(true),
            demo,
            title,
            artist,
            source,
            art,
            placeholder,
            header,
            metadata,
            controls_box: controls,
            toolbar_slot,
            cover,
            move_handle,
            transport: vec![
                previous.clone(),
                play.clone(),
                next.clone(),
                full_lyrics.clone(),
            ],
            dragging: Cell::new(false),
            play,
            next,
            previous,
            progress,
            seek_pending: RefCell::new(None),
            view,
            material,
            css,
            _hold: app.hold(),
        });
        ui.on_button(&ui.play, |ui| ui.media(MediaCommand::PlayPause));
        ui.on_button(&ui.next, |ui| ui.media(MediaCommand::Next));
        ui.on_button(&ui.previous, |ui| ui.media(MediaCommand::Previous));
        ui.on_button(&settings, crate::settings::open);
        ui.on_button(&hide, |ui| ui.set_visible(false));
        ui.on_button(&full_lyrics, |ui| ui.show_lyrics());
        let drag = gtk::GestureDrag::new();
        let weak = Rc::downgrade(&ui);
        drag.connect_drag_begin(move |gesture, _, _| {
            if let Some(ui) = weak.upgrade() {
                if ui.config.borrow().lock_position {
                    return;
                }
                if ui.platform.begin_move(&ui.window, gesture) {
                    return;
                }
                ui.begin_drag();
            }
        });
        let weak = Rc::downgrade(&ui);
        drag.connect_drag_update(move |_, x, y| {
            if let Some(ui) = weak.upgrade() {
                ui.drag_by(x, y);
            }
        });
        let weak = Rc::downgrade(&ui);
        drag.connect_drag_end(move |_, _, _| {
            if let Some(ui) = weak.upgrade() {
                ui.dragging.set(false);
                ui.move_handle.set_cursor_from_name(Some("grab"));
                ui.save_preferences();
            }
        });
        ui.move_handle.add_controller(drag);
        let weak = Rc::downgrade(&ui);
        ui.progress.connect_change_value(move |_, _, value| {
            if let Some(ui) = weak.upgrade() {
                *ui.seek_pending.borrow_mut() = Some((value, Instant::now()));
            }
            glib::Propagation::Proceed
        });
        let weak = Rc::downgrade(&ui);
        ui.window.connect_map(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.apply_input_region();
                ui.platform.position(&ui.window, &ui.config.borrow());
            }
        });
        let weak = Rc::downgrade(&ui);
        ui.window.connect_close_request(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.set_visible(false);
            }
            glib::Propagation::Stop
        });
        let weak = Rc::downgrade(&ui);
        glib::timeout_add_local(Duration::from_millis(50), move || {
            let Some(ui) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            for event in events.try_iter().take(64) {
                ui.event(event);
            }
            ui.flush_seek();
            for action in ui.platform.actions() {
                match action.as_str() {
                    "toggle" => ui.toggle(),
                    "settings" => crate::settings::open(&ui),
                    "game-mode" => ui.game_mode(),
                    "play-pause" => ui.media(MediaCommand::PlayPause),
                    "previous" => ui.media(MediaCommand::Previous),
                    "next" => ui.media(MediaCommand::Next),
                    _ => {}
                }
            }
            glib::ControlFlow::Continue
        });
        let weak = Rc::downgrade(&ui);
        let last = Cell::new(Instant::now());
        ui.window.add_tick_callback(move |_, _| {
            let Some(ui) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if last.get().elapsed() >= Duration::from_millis(25) {
                ui.tick();
                last.set(Instant::now());
            }
            glib::ControlFlow::Continue
        });
        ui.configure();
        ui.save_preferences();
        if demo {
            ui.load_demo();
        }
        Ok(ui)
    }

    pub fn on_button(self: &Rc<Self>, button: &gtk::Button, action: impl Fn(&Rc<Self>) + 'static) {
        let weak = Rc::downgrade(self);
        button.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                action(&ui);
            }
        });
    }
    pub fn media(&self, command: MediaCommand) {
        if self.demo {
            return;
        }
        let _ = self.backend.media.send(command);
    }
    pub fn change(&self, update: impl FnOnce(&mut Config)) {
        {
            let mut config = self.config.borrow_mut();
            update(&mut config);
            config.normalize();
        }
        self.configure();
        self.save_preferences();
    }
    fn save_preferences(&self) {
        let config = self.config.borrow().clone();
        if let Some(action) = self
            .window
            .application()
            .and_then(|app| app.lookup_action("apply-config"))
            .and_then(|action| action.downcast::<gtk::gio::SimpleAction>().ok())
            && let Ok(json) = serde_json::to_string(&config)
        {
            action.set_state(&json.to_variant());
        }
        let _ = self.backend.preferences.send(Preference::Save(config));
    }
    pub fn toggle(&self) {
        self.set_visible(!self.visible.get());
    }
    pub fn set_visible(&self, visible: bool) {
        self.visible.set(visible);
        self.visibility();
    }
    pub fn game_mode(&self) {
        self.change(|c| c.click_through = !c.click_through);
    }
    pub fn place(&self) {
        self.change(|c| {
            c.lock_position = false;
            c.free_position = true;
            c.click_through = false;
            c.lyrics_only = false;
        });
        self.set_visible(true);
    }
    pub fn recover(&self) {
        self.change(Config::recover_overlay);
        self.set_visible(true);
    }
    fn monitor_size(&self) -> (i32, i32) {
        let config = self.config.borrow();
        let monitors = WidgetExt::display(&self.window).monitors();
        let all: Vec<gdk::Monitor> = (0..monitors.n_items())
            .filter_map(|i| monitors.item(i)?.downcast().ok())
            .collect();
        all.iter()
            .find(|m| m.connector().as_deref() == Some(config.monitor.as_str()))
            .or_else(|| all.first())
            .map(|m| (m.geometry().width(), m.geometry().height()))
            .unwrap_or((1920, 1080))
    }
    fn begin_drag(&self) {
        let (width, monitor_height) = self.monitor_size();
        let height = self.window.height();
        self.change(|c| {
            if !c.free_position {
                c.x = (width - self.window.width()) / 2;
                c.y = if c.bottom {
                    monitor_height - height - c.margin
                } else {
                    c.margin
                };
            }
            c.free_position = true;
            c.click_through = false;
        });
        self.dragging.set(true);
        self.move_handle.set_cursor_from_name(Some("grabbing"));
    }
    fn drag_by(&self, dx: f64, dy: f64) {
        if !self.dragging.get() {
            return;
        }
        let (width, height) = self.monitor_size();
        let mut config = self.config.borrow_mut();
        config.x = (config.x + dx.round() as i32).clamp(0, (width - self.window.width()).max(0));
        config.y = (config.y + dy.round() as i32).clamp(0, (height - self.window.height()).max(0));
        self.platform.position(&self.window, &config);
    }
    fn visibility(&self) {
        let hide_idle = self.config.borrow().hide_idle;
        let idle = self
            .snapshot
            .borrow()
            .as_ref()
            .is_none_or(|s| s.track.title.is_empty() || s.playback == Playback::Stopped);
        let paused = self.config.borrow().hide_paused
            && self
                .snapshot
                .borrow()
                .as_ref()
                .is_some_and(|s| s.playback == Playback::Paused);
        self.window
            .set_visible(self.visible.get() && !(hide_idle && idle) && !paused);
    }
    fn apply_input_region(&self) {
        if let Some(surface) = self.window.surface() {
            if self.config.borrow().click_through {
                surface.set_input_region(Some(&gtk::cairo::Region::create()));
            } else {
                surface.set_input_region(None);
            }
        }
    }
    pub fn configure(&self) {
        crate::i18n::set_language(&self.config.borrow().language);
        if let Some(snapshot) = self.snapshot.borrow().as_ref() {
            let playing = snapshot.playback == Playback::Playing;
            self.artist.set_text(&format!(
                "{}{}",
                snapshot.track.artist(),
                if playing { "" } else { tr("  ·  Paused") }
            ));
            self.play
                .set_tooltip_text(Some(if playing { tr("Pause") } else { tr("Play") }));
        } else {
            self.artist.set_text(tr("Spotify is not running"));
        }
        if let Err(error) = self.platform.shortcuts(&self.config.borrow().shortcuts) {
            self.notice.set_text(&error.to_string());
        }
        let bounds = self.monitor_size();
        {
            let mut config = self.config.borrow_mut();
            let width = if config.square {
                config.square_size
            } else {
                config.width
            };
            let height = if config.square {
                config.square_size
            } else {
                self.window.height().max(120)
            };
            config.clamp_position(bounds, (width, height));
        }
        let config = self.config.borrow();
        if config.square {
            self.window.add_css_class("square");
        } else {
            self.window.remove_css_class("square");
        }
        let display = WidgetExt::display(&self.window);
        let monitors = display.monitors();
        let selected = (0..monitors.n_items())
            .filter_map(|i| monitors.item(i)?.downcast::<gdk::Monitor>().ok())
            .find(|m| m.connector().as_deref() == Some(config.monitor.as_str()));
        #[cfg(feature = "wayland")]
        if self.platform.is_layer() {
            self.window.set_monitor(selected.as_ref());
        }
        let max_width = selected
            .or_else(|| {
                monitors
                    .item(0)
                    .and_then(|m| m.downcast::<gdk::Monitor>().ok())
            })
            .map(|m| m.geometry().width() - 32)
            .unwrap_or(900)
            .max(320);
        self.window.set_default_size(
            if config.square {
                config.square_size
            } else {
                config.width
            }
            .min(max_width),
            if config.square {
                config.square_size
            } else {
                -1
            },
        );
        self.header.set_orientation(if config.square {
            gtk::Orientation::Vertical
        } else {
            gtk::Orientation::Horizontal
        });
        self.header.set_spacing(if config.square { 4 } else { 12 });
        let narrow = !config.square && config.width.min(max_width) < 480;
        if narrow && self.controls_box.parent().as_ref() == Some(self.header.upcast_ref()) {
            self.header.remove(&self.controls_box);
            self.toolbar_slot.append(&self.controls_box);
        } else if !narrow
            && self.controls_box.parent().as_ref() == Some(self.toolbar_slot.upcast_ref())
        {
            self.toolbar_slot.remove(&self.controls_box);
            self.header.append(&self.controls_box);
        }
        self.toolbar_slot.set_visible(narrow);
        self.metadata.set_hexpand(!config.square);
        self.controls_box.set_halign(if config.square {
            gtk::Align::Center
        } else {
            gtk::Align::End
        });
        self.cover.set_halign(if config.square {
            gtk::Align::Center
        } else {
            gtk::Align::Start
        });
        for label in [&self.title, &self.artist, &self.source] {
            label.set_xalign(if config.square { 0.5 } else { 0.0 });
        }
        self.cover.set_visible(config.show_artwork);
        self.header.set_visible(!config.lyrics_only);
        self.toolbar_slot.set_visible(narrow && !config.lyrics_only);
        self.move_handle.set_visible(!config.lock_position);
        for button in &self.transport {
            button.set_visible(config.show_controls);
        }
        self.progress.set_visible(config.show_progress);
        self.view.widget.set_vexpand(config.square);
        self.platform.position(&self.window, &config);
        let system_motion = gtk::Settings::default().is_none_or(|s| s.is_gtk_enable_animations());
        self.view.appearance(&config);
        self.view.configure(
            if config.square {
                config
                    .font_size
                    .min(((config.square_size - 202) / 2).max(14))
            } else {
                config.font_size
            },
            config.compact || !config.show_context,
            config.square,
            config.animations && system_motion,
        );
        let accent = match config.accent.as_str() {
            "custom" => &config.custom_accent,
            "blue" => "#9bc6f4",
            "rose" => "#efa5be",
            "gold" => "#e2cc8b",
            _ => "#91d5b2",
        };
        self.material
            .configure(&config, config.animations && system_motion);
        for style in ["glass", "dark", "light", "contrast", "minimal"] {
            self.window.remove_css_class(&format!("theme-{style}"));
        }
        self.window
            .add_css_class(&format!("theme-{}", config.theme));
        if config.liquid && config.theme == "glass" {
            self.window.add_css_class("liquid");
        } else {
            self.window.remove_css_class("liquid");
        }
        self.css.load_from_string(&format!(
            "{}\n.glass {{ background: alpha(#18181b, {}); border-radius: {}px; }}\n.liquid .glass {{ background: transparent; border-color: transparent; border-radius: {}px; }}\n.glass scale highlight, .preferences scale highlight, .preferences switch:checked {{ background: {}; }}",
            include_str!("../data/style.css"),
            config.opacity,config.material_radius(),config.liquid_radius,accent
        ));
        if config.animations {
            self.window.remove_css_class("no-motion");
        } else {
            self.window.add_css_class("no-motion");
        }
        drop(config);
        crate::i18n::refresh(&self.window);
        self.apply_input_region();
        self.visibility();
        self.tick();
    }
    pub fn retry(&self) {
        if let Some(snapshot) = self.snapshot.borrow().as_ref() {
            self.view.set(None, tr("Searching for lyrics"));
            let _ = self.backend.loads.send(Load {
                generation: self.generation.get(),
                track: snapshot.track.clone(),
                force: true,
            });
        }
    }
    fn event(&self, event: Event) {
        match event {
            Event::Shutdown => {
                if let Some(app) = self.window.application() {
                    app.quit();
                }
            }
            Event::Shortcuts(keys) => self.change(|c| c.shortcuts = keys),
            Event::Startup(enabled) => self.change(|c| c.autostart = enabled),
            Event::Imported(mut config) => {
                config.autostart = self.config.borrow().autostart;
                self.change(|c| *c = *config);
                if let Some(window) = self.settings.borrow_mut().take() {
                    window.destroy();
                }
            }
            Event::Player(_) if self.demo => {}
            Event::Player(PlayerEvent::State(snapshot)) => self.update(*snapshot),
            Event::Player(PlayerEvent::Unavailable) => {
                if self.snapshot.borrow_mut().take().is_some() {
                    self.generation.set(self.generation.get() + 1);
                    self.lyrics.borrow_mut().take();
                }
                self.title.set_text("Vitrilyr");
                self.artist.set_text(tr("Spotify is not running"));
                self.source.set_text(tr("Waiting"));
                self.view.set(None, tr("Your music appears here"));
                self.clear_art();
                self.controls(false, false, false, false);
                self.progress.set_value(0.0);
                self.visibility();
            }
            Event::Player(PlayerEvent::Error(error)) | Event::Notice(error) => {
                self.notice.set_text(tr(&error));
                self.source.set_tooltip_text(Some(tr(&error)));
                tracing::warn!(message=%error);
            }
            Event::Lyrics(generation, result) if generation == self.generation.get() => {
                match result {
                    Ok(lyrics) => {
                        let message = match &lyrics {
                            Lyrics::Synced(_) => "",
                            Lyrics::Plain(_) => tr("Unsynchronized lyrics available"),
                            Lyrics::Instrumental => tr("Instrumental"),
                            Lyrics::Missing => tr("No lyrics for this track"),
                        };
                        self.source.set_text(match &lyrics {
                            Lyrics::Synced(_) => "Spotify  ·  LRCLIB",
                            Lyrics::Plain(_) => tr("Spotify  ·  Unsynchronized lyrics"),
                            _ => "Spotify",
                        });
                        self.view.set(Some(lyrics.clone()), message);
                        *self.lyrics.borrow_mut() = Some(lyrics);
                        self.tick();
                    }
                    Err(error) => {
                        self.source.set_text(tr("Lyrics unavailable"));
                        self.view.set(None, tr("Could not load lyrics"));
                        self.notice.set_text(tr(&error));
                        self.source.set_tooltip_text(Some(tr(&error)));
                    }
                }
            }
            Event::Art(generation, art) if generation == self.generation.get() => {
                if let Some(art) = art {
                    let bytes = glib::Bytes::from_owned(art.rgba);
                    let texture = gdk::MemoryTexture::new(
                        art.width as i32,
                        art.height as i32,
                        gdk::MemoryFormat::R8g8b8a8,
                        &bytes,
                        art.width as usize * 4,
                    );
                    self.art.set_paintable(Some(&texture));
                    self.placeholder.set_visible(false);
                } else {
                    self.clear_art();
                }
            }
            _ => {}
        }
    }
    fn clear_art(&self) {
        self.art.set_paintable(None::<&gdk::Paintable>);
        self.placeholder.set_visible(true);
    }
    fn controls(&self, play: bool, next: bool, previous: bool, seek: bool) {
        self.play.set_sensitive(play);
        self.next.set_sensitive(next);
        self.previous.set_sensitive(previous);
        self.progress.set_sensitive(seek);
    }
    fn update(&self, snapshot: Snapshot) {
        let changed = self
            .snapshot
            .borrow()
            .as_ref()
            .is_none_or(|old| old.track != snapshot.track);
        if changed {
            self.generation.set(self.generation.get() + 1);
            self.lyrics.borrow_mut().take();
            self.clear_art();
            self.seek_pending.borrow_mut().take();
            self.title.set_text(if snapshot.track.title.is_empty() {
                "Spotify"
            } else {
                &snapshot.track.title
            });
            self.title.set_tooltip_text(Some(&snapshot.track.title));
            self.artist.set_text(&snapshot.track.artist());
            self.artist.set_tooltip_text(Some(&snapshot.track.artist()));
            self.source.set_text(tr("Searching for lyrics"));
            self.source.set_tooltip_text(None);
            self.view.set(None, tr("Searching for lyrics"));
            let _ = self.backend.loads.send(Load {
                generation: self.generation.get(),
                track: snapshot.track.clone(),
                force: false,
            });
            self.progress
                .set_range(0.0, snapshot.track.duration().as_secs_f64().max(1.0));
        }
        let playing = snapshot.playback == Playback::Playing;
        self.play.set_icon_name(if playing {
            "media-playback-pause-symbolic"
        } else {
            "media-playback-start-symbolic"
        });
        self.play
            .set_tooltip_text(Some(if playing { tr("Pause") } else { tr("Play") }));
        self.artist.set_text(&format!(
            "{}{}",
            snapshot.track.artist(),
            if playing { "" } else { tr("  ·  Paused") }
        ));
        self.controls(
            snapshot.can_control
                && if playing {
                    snapshot.can_pause
                } else {
                    snapshot.can_play
                },
            snapshot.can_control && snapshot.can_next,
            snapshot.can_control && snapshot.can_previous,
            snapshot.can_control && snapshot.can_seek && snapshot.track.length_us > 0,
        );
        *self.snapshot.borrow_mut() = Some(snapshot);
        self.visibility();
        self.tick();
    }
    fn tick(&self) {
        self.material.tick();
        if matches!(self.platform, crate::platform::Platform::X11(_)) && self.window.is_mapped() {
            self.platform.position(&self.window, &self.config.borrow());
        }
        if self.window.is_mapped() && !self.dragging.get() {
            let bounds = self.monitor_size();
            let mut config = self.config.borrow_mut();
            let previous = (config.x, config.y);
            if config.free_position {
                config.clamp_position(bounds, (self.window.width(), self.window.height()));
                if previous != (config.x, config.y) {
                    self.platform.position(&self.window, &config);
                    drop(config);
                    self.save_preferences();
                }
            }
        }
        let snapshot = self.snapshot.borrow();
        let Some(snapshot) = snapshot.as_ref() else {
            return;
        };
        let position = snapshot.position_at(Instant::now());
        let offset = self.config.borrow().lyric_offset_ms;
        let adjusted = if offset >= 0 {
            position.saturating_add(Duration::from_millis(offset as u64))
        } else {
            position.saturating_sub(Duration::from_millis(offset.unsigned_abs() as u64))
        };
        self.view.tick(adjusted);
        if self.seek_pending.borrow().is_none() {
            let value = position.as_secs_f64();
            if (self.progress.value() - value).abs() >= 0.025 {
                self.progress.set_value(value);
            }
        }
    }
    fn flush_seek(&self) {
        let ready = self
            .seek_pending
            .borrow()
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() >= Duration::from_millis(180));
        if ready
            && let Some((value, _)) = self.seek_pending.borrow_mut().take()
            && let Some(snapshot) = self.snapshot.borrow_mut().as_mut()
        {
            self.media(MediaCommand::SetPosition(snapshot.track.id.clone(), value));
            snapshot.position = Duration::from_secs_f64(value.max(0.0));
            snapshot.sampled_at = Instant::now();
        }
    }
    pub fn volume(&self) -> Option<f64> {
        self.snapshot.borrow().as_ref().and_then(|s| s.volume)
    }
    pub fn status(&self) -> String {
        let snapshot = self.snapshot.borrow();
        serde_json::json!({"visible":self.window.is_visible(),"click_through":self.config.borrow().click_through,
            "backend":self.platform.name(),
            "spotify": snapshot.is_some() && !self.demo,"demo":self.demo,"title":snapshot.as_ref().map(|s| &s.track.title),
            "language":self.config.borrow().language,"theme":self.config.borrow().theme,
            "settings_page":self.settings.borrow().as_ref().map(|_|self.settings_page.borrow().clone()),
            "playback":snapshot.as_ref().map(|s| s.playback),"position":snapshot.as_ref().map(|s| s.position_at(Instant::now()).as_secs_f64()),
            "duration":snapshot.as_ref().map(|s|s.track.duration().as_secs_f64()),
            "can_seek":snapshot.as_ref().is_some_and(|s|s.can_seek),
            "layout":if self.config.borrow().square {"square"} else if self.config.borrow().compact {"line"} else {"normal"},
            "geometry":{"width":self.window.width(),"height":self.window.height(),"x":self.config.borrow().x,"y":self.config.borrow().y,"free":self.config.borrow().free_position},
            "lyrics": match self.lyrics.borrow().as_ref() {Some(Lyrics::Synced(_))=>"synced",Some(Lyrics::Plain(_))=>"plain",Some(Lyrics::Instrumental)=>"instrumental",Some(Lyrics::Missing)=>"missing",None=>"loading"}}).to_string()
    }
    pub fn capture(&self, path: &std::path::Path, settings: bool) -> anyhow::Result<()> {
        use anyhow::Context;
        let window = if settings {
            self.settings
                .borrow()
                .clone()
                .context("Settings are not open")?
        } else {
            self.window.clone().upcast::<gtk::Window>()
        };
        let snapshot = gtk::Snapshot::new();
        let paintable = gtk::WidgetPaintable::new(Some(&window));
        paintable.snapshot(
            &snapshot,
            f64::from(window.width()),
            f64::from(window.height()),
        );
        let node = snapshot.to_node().context("Overlay has no render node")?;
        let renderer = window.renderer().context("Overlay has no renderer")?;
        renderer.render_texture(&node, None).save_to_png(path)?;
        Ok(())
    }
    pub fn show_lyrics(&self) {
        let text = match self.lyrics.borrow().as_ref() {
            Some(Lyrics::Synced(lines)) => lines
                .iter()
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n"),
            Some(Lyrics::Plain(text)) => text.clone(),
            Some(Lyrics::Instrumental) => tr("Instrumental").into(),
            _ => tr("No lyrics available").into(),
        };
        let window = crate::settings::dialog(self, tr("Full lyrics"), 500, 650);
        let label = gtk::Label::new(Some(&text));
        label.set_wrap(true);
        label.set_selectable(true);
        label.set_xalign(0.0);
        label.add_css_class("plain-lyrics");
        label.set_margin_top(24);
        label.set_margin_bottom(24);
        label.set_margin_start(24);
        label.set_margin_end(24);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&label)
            .vexpand(true)
            .build();
        crate::settings::set_dialog_content(&window, &scroll);
        window.present();
    }
    fn load_demo(&self) {
        let snapshot = Snapshot {
            track: Track {
                title: "A little room for music".into(),
                artists: vec!["Vitrilyr".into()],
                length_us: 180_000_000,
                ..Track::default()
            },
            playback: Playback::Playing,
            position: Duration::from_secs(4),
            sampled_at: Instant::now(),
            rate: 1.0,
            volume: Some(0.5),
            can_control: false,
            can_play: false,
            can_pause: false,
            can_seek: false,
            can_next: false,
            can_previous: false,
        };
        self.update(snapshot);
        self.generation.set(self.generation.get() + 1);
        self.event(Event::Lyrics(self.generation.get(),Ok(Lyrics::Synced(vitrilyr::lrc::parse(
            "[00:00.00]A quiet moment\n[00:04.00]Let the music stay with you\n[00:09.00]One line at a time\n[00:14.00]A little room for music\n[00:19.00]Wherever the evening goes")))));
        self.source.set_text(tr("Preview"));
    }
}

fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_width_chars(1);
    label.add_css_class(class);
    label
}
