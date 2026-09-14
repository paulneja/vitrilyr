use gtk::{gdk, prelude::*};
#[cfg(feature = "wayland")]
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use vitrilyr::config::Config;
use x11rb::{
    connection::Connection,
    protocol::xproto::{self, ConfigureWindowAux, ConnectionExt, PropMode},
    wrapper::ConnectionExt as _,
};

pub enum Platform {
    #[cfg_attr(not(feature = "wayland"), allow(dead_code))]
    LayerShell,
    X11(Box<RefCell<X11>>),
    Managed,
}
pub struct X11 {
    connection: x11rb::rust_connection::RustConnection,
    root: u32,
    last_position: Option<(i32, i32, i32, i32)>,
    shortcut_keys: Vec<String>,
    bindings: Vec<(u8, u16, String)>,
}

impl Platform {
    pub fn detect() -> Self {
        #[cfg(feature = "wayland")]
        if gdk::Display::default().is_some_and(|display| {
            gtk::glib::Type::from_name("GdkWaylandDisplay")
                .is_some_and(|wayland| display.type_().is_a(wayland))
        }) && gtk4_layer_shell::is_supported()
        {
            return Self::LayerShell;
        }
        if gdk::Display::default().is_some_and(|d| d.is::<gdk_x11::X11Display>()) {
            match x11rb::connect(None) {
                Ok((connection, screen)) => {
                    let root = connection.setup().roots[screen].root;
                    return Self::X11(Box::new(RefCell::new(X11 {
                        connection,
                        root,
                        last_position: None,
                        shortcut_keys: Vec::new(),
                        bindings: Vec::new(),
                    })));
                }
                Err(error) => tracing::warn!(%error, "X11 positioning unavailable"),
            }
        }
        Self::Managed
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::LayerShell => "Wayland layer-shell",
            Self::X11(_) => "X11",
            Self::Managed => "Desktop window",
        }
    }
    #[cfg(feature = "wayland")]
    pub fn is_layer(&self) -> bool {
        matches!(self, Self::LayerShell)
    }
    pub fn shortcuts(&self, keys: &vitrilyr::config::Shortcuts) -> anyhow::Result<()> {
        if let Self::X11(x11) = self {
            x11.borrow_mut().shortcuts(keys)?;
        }
        Ok(())
    }
    pub fn actions(&self) -> Vec<String> {
        let Self::X11(x11) = self else {
            return Vec::new();
        };
        let x11 = x11.borrow();
        let mut actions = Vec::new();
        for _ in 0..32 {
            match x11.connection.poll_for_event() {
                Ok(Some(x11rb::protocol::Event::KeyPress(event))) => {
                    let mask = u16::from(event.state) & 77;
                    if let Some((_, _, action)) = x11
                        .bindings
                        .iter()
                        .find(|(key, modifiers, _)| *key == event.detail && *modifiers == mask)
                    {
                        actions.push(action.clone());
                    }
                }
                Ok(Some(_)) => {}
                _ => break,
            }
        }
        actions
    }
    pub fn setup(&self, window: &impl IsA<gtk::Window>, settings: bool) {
        #[cfg(feature = "wayland")]
        if self.is_layer() {
            window.init_layer_shell();
            window.set_namespace(Some(if settings {
                "vitrilyr-settings"
            } else {
                "vitrilyr"
            }));
            window.set_layer(Layer::Overlay);
            window.set_exclusive_zone(0);
            window.set_keyboard_mode(if settings {
                KeyboardMode::OnDemand
            } else {
                KeyboardMode::None
            });
            return;
        }
        window.set_decorated(settings || matches!(self, Self::Managed));
    }
    pub fn position(&self, window: &gtk::ApplicationWindow, config: &Config) {
        #[cfg(feature = "wayland")]
        if self.is_layer() {
            window.set_exclusive_zone(if config.free_position { -1 } else { 0 });
            window.set_anchor(Edge::Left, config.free_position);
            window.set_anchor(Edge::Top, config.free_position || !config.bottom);
            window.set_anchor(Edge::Bottom, !config.free_position && config.bottom);
            window.set_margin(Edge::Left, if config.free_position { config.x } else { 0 });
            window.set_margin(
                Edge::Top,
                if config.free_position {
                    config.y
                } else if config.bottom {
                    0
                } else {
                    config.margin
                },
            );
            window.set_margin(
                Edge::Bottom,
                if !config.free_position && config.bottom {
                    config.margin
                } else {
                    0
                },
            );
            return;
        }
        if let Self::X11(x11) = self
            && let Err(error) = x11.borrow_mut().position(window, config)
        {
            tracing::debug!(%error, "X11 geometry update failed");
        }
    }
    pub fn begin_move(&self, window: &gtk::ApplicationWindow, gesture: &gtk::GestureDrag) -> bool {
        if !matches!(self, Self::Managed) {
            return false;
        }
        if let (Some(surface), Some(event)) = (window.surface(), gesture.current_event())
            && let (Ok(toplevel), Some(device), Some((x, y))) = (
                surface.downcast::<gdk::Toplevel>(),
                event.device(),
                event.position(),
            )
        {
            toplevel.begin_move(&device, 1, x, y, event.time());
        }
        true
    }
}

impl X11 {
    fn atom(&self, name: &[u8]) -> anyhow::Result<u32> {
        Ok(self.connection.intern_atom(false, name)?.reply()?.atom)
    }
    fn position(&mut self, window: &gtk::ApplicationWindow, config: &Config) -> anyhow::Result<()> {
        let Some(surface) = window
            .surface()
            .and_then(|s| s.downcast::<gdk_x11::X11Surface>().ok())
        else {
            return Ok(());
        };
        let xid = surface.xid() as u32;
        let monitors = WidgetExt::display(window).monitors();
        let monitor = (0..monitors.n_items())
            .filter_map(|i| monitors.item(i)?.downcast::<gdk::Monitor>().ok())
            .find(|m| m.connector().as_deref() == Some(&config.monitor))
            .or_else(|| monitors.item(0)?.downcast::<gdk::Monitor>().ok());
        let Some(monitor) = monitor else {
            return Ok(());
        };
        let geo = monitor.geometry();
        let x = geo.x()
            + if config.free_position {
                config.x
            } else {
                (geo.width() - window.width()) / 2
            };
        let y = geo.y()
            + if config.free_position {
                config.y
            } else if config.bottom {
                geo.height() - window.height() - config.margin
            } else {
                config.margin
            };
        let scale = surface.scale_factor();
        let geometry = (
            x * scale,
            y * scale,
            window.width() * scale,
            window.height() * scale,
        );
        if self.last_position == Some(geometry) {
            return Ok(());
        }
        let state = self.atom(b"_NET_WM_STATE")?;
        let above = self.atom(b"_NET_WM_STATE_ABOVE")?;
        let skip = self.atom(b"_NET_WM_STATE_SKIP_TASKBAR")?;
        self.connection.change_property32(
            PropMode::REPLACE,
            xid,
            state,
            xproto::AtomEnum::ATOM,
            &[above, skip],
        )?;
        // Mapped windows must also ask the window manager to change EWMH state.
        self.connection.send_event(
            false,
            self.root,
            xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
            xproto::ClientMessageEvent::new(32, xid, state, [1, above, skip, 1, 0]),
        )?;
        self.connection.change_property32(
            PropMode::REPLACE,
            xid,
            self.atom(b"_NET_WM_WINDOW_TYPE")?,
            xproto::AtomEnum::ATOM,
            &[self.atom(b"_NET_WM_WINDOW_TYPE_UTILITY")?],
        )?;
        self.connection
            .configure_window(xid, &ConfigureWindowAux::new().x(geometry.0).y(geometry.1))?;
        self.connection.change_property32(
            PropMode::REPLACE,
            xid,
            self.atom(b"WM_HINTS")?,
            self.atom(b"WM_HINTS")?,
            &[1, 0, 0, 0, 0, 0, 0, 0, 0],
        )?;
        self.connection.flush()?;
        self.last_position = Some(geometry);
        Ok(())
    }
    fn shortcuts(&mut self, keys: &vitrilyr::config::Shortcuts) -> anyhow::Result<()> {
        let names: Vec<String> = keys
            .pairs()
            .iter()
            .map(|(key, _)| key.to_string())
            .collect();
        if self.shortcut_keys == names {
            return Ok(());
        }
        keys.render(std::path::Path::new("vitrilyr"))?;
        let setup = self.connection.setup();
        let mapping = self
            .connection
            .get_keyboard_mapping(setup.min_keycode, setup.max_keycode - setup.min_keycode + 1)?
            .reply()?;
        let mut bindings = Vec::new();
        for (key, action) in keys.pairs() {
            if key.is_empty() {
                continue;
            }
            let (keysym, modifiers) = parse_shortcut(key)?;
            let index = mapping
                .keysyms
                .chunks(usize::from(mapping.keysyms_per_keycode))
                .position(|row| row.contains(&keysym))
                .ok_or_else(|| anyhow::anyhow!("Key is not on this keyboard: {key}"))?;
            bindings.push((
                setup.min_keycode + index as u8,
                modifiers,
                action.to_string(),
            ));
        }
        self.connection
            .ungrab_key(xproto::Grab::ANY, self.root, xproto::ModMask::ANY)?
            .check()?;
        if let Err(error) = self.grab(&bindings) {
            let _ = self
                .connection
                .ungrab_key(xproto::Grab::ANY, self.root, xproto::ModMask::ANY);
            let _ = self.grab(&self.bindings);
            return Err(error.context("Shortcut already used by another application"));
        }
        self.connection.flush()?;
        self.shortcut_keys = names;
        self.bindings = bindings;
        Ok(())
    }
    fn grab(&self, bindings: &[(u8, u16, String)]) -> anyhow::Result<()> {
        for (key, modifiers, _) in bindings {
            for ignored in [0, 2, 16, 18] {
                self.connection
                    .grab_key(
                        false,
                        self.root,
                        xproto::ModMask::from(*modifiers | ignored),
                        *key,
                        xproto::GrabMode::ASYNC,
                        xproto::GrabMode::ASYNC,
                    )?
                    .check()?;
            }
        }
        Ok(())
    }
}

fn parse_shortcut(binding: &str) -> anyhow::Result<(u32, u16)> {
    let mut parts: Vec<&str> = binding.split('+').collect();
    let key = parts.pop().unwrap_or("");
    let mut modifiers = 0;
    for modifier in parts {
        modifiers |= match modifier.to_ascii_lowercase().as_str() {
            "shift" => 1,
            "ctrl" | "control" => 4,
            "alt" => 8,
            "super" | "mod" => 64,
            _ => anyhow::bail!("Unknown modifier: {modifier}"),
        };
    }
    let symbol = if let Some(number) = key.strip_prefix('F').and_then(|s| s.parse::<u32>().ok()) {
        anyhow::ensure!((1..=35).contains(&number), "Invalid function key");
        0xffbd + number
    } else if key.len() == 1 && key.is_ascii() {
        u32::from(key.to_ascii_lowercase().as_bytes()[0])
    } else {
        match key {
            "space" => 32,
            "Return" => 0xff0d,
            "Escape" => 0xff1b,
            "Tab" => 0xff09,
            "Left" => 0xff51,
            "Up" => 0xff52,
            "Right" => 0xff53,
            "Down" => 0xff54,
            _ => anyhow::bail!("Unsupported key: {key}"),
        }
    };
    Ok((symbol, modifiers))
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_shortcuts_without_shell_commands() {
        assert_eq!(super::parse_shortcut("Ctrl+F7").unwrap(), (0xffc4, 4));
        assert_eq!(super::parse_shortcut("Super+Shift+A").unwrap(), (97, 65));
        assert!(super::parse_shortcut("F99").is_err());
        assert!(super::parse_shortcut("Bad+F7").is_err());
    }
}
