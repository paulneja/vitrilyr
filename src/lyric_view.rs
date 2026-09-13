use gtk::{cairo, pango, prelude::*};
use lyricglass::{lrc, lyrics::Lyrics};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

pub struct LyricView {
    pub widget: gtk::DrawingArea,
    state: Rc<RefCell<Presentation>>,
}
struct Presentation {
    lyrics: Option<Lyrics>,
    message: String,
    active: Option<usize>,
    from: Option<usize>,
    changed: Instant,
    size: i32,
    compact: bool,
    square: bool,
    animate: bool,
}
impl LyricView {
    pub fn new() -> Self {
        let widget = gtk::DrawingArea::builder()
            .content_height(84)
            .hexpand(true)
            .build();
        let state = Rc::new(RefCell::new(Presentation {
            lyrics: None,
            message: "Esperando a Spotify".into(),
            active: None,
            from: None,
            changed: Instant::now(),
            size: 19,
            compact: false,
            square: false,
            animate: true,
        }));
        let drawn = state.clone();
        widget.set_draw_func(move |_, cr, width, height| drawn.borrow().draw(cr, width, height));
        Self { widget, state }
    }
    pub fn configure(&self, size: i32, compact: bool, square: bool, animate: bool) {
        let mut state = self.state.borrow_mut();
        state.size = size;
        state.compact = compact;
        state.square = square;
        state.animate = animate;
        self.widget.set_content_height(if square {
            size * 2 + 12
        } else {
            (size + 9) * if compact { 1 } else { 3 }
        });
        self.widget.queue_draw();
    }
    pub fn set(&self, lyrics: Option<Lyrics>, message: &str) {
        let mut state = self.state.borrow_mut();
        state.lyrics = lyrics;
        state.message = message.into();
        state.active = None;
        state.from = None;
        self.widget.queue_draw();
        self.widget.set_tooltip_text(Some(message));
    }
    pub fn tick(&self, position: Duration) {
        let mut state = self.state.borrow_mut();
        let active = match &state.lyrics {
            Some(Lyrics::Synced(lines)) => lrc::active(lines, position),
            _ => None,
        };
        if active != state.active {
            state.from = state.active;
            state.active = active;
            state.changed = Instant::now();
            let text = match &state.lyrics {
                Some(Lyrics::Synced(lines)) => active.map(|i| lines[i].text.clone()),
                _ => None,
            };
            if let Some(text) = text {
                self.widget.set_tooltip_text(Some(&text));
                self.widget
                    .update_property(&[gtk::accessible::Property::Label(&text)]);
            }
            self.widget.queue_draw();
        } else if state.animate && state.changed.elapsed() < Duration::from_millis(220) {
            self.widget.queue_draw();
        }
    }
}

impl Presentation {
    fn draw(&self, cr: &cairo::Context, width: i32, height: i32) {
        let Some(Lyrics::Synced(lines)) = &self.lyrics else {
            self.text(
                cr,
                &self.message,
                width,
                f64::from(height) / 2.0,
                self.size.min(17),
                0.72,
                false,
            );
            return;
        };
        if lines.is_empty() {
            return;
        }
        if self.square {
            let text = self
                .active
                .map(|i| lines[i].text.as_str())
                .unwrap_or("Inicio instrumental");
            self.text(
                cr,
                if text.is_empty() {
                    "Instrumental"
                } else {
                    text
                },
                width,
                f64::from(height) / 2.0,
                self.size,
                if self.animate {
                    (self.changed.elapsed().as_secs_f64() / 0.2).clamp(0.3, 1.0)
                } else {
                    1.0
                },
                true,
            );
            return;
        }
        let progress = if self.animate {
            (self.changed.elapsed().as_secs_f64() / 0.2).min(1.0)
        } else {
            1.0
        };
        let eased = 1.0 - (1.0 - progress).powi(3);
        let target = self.active.map(|i| i as f64).unwrap_or(-1.0);
        let from = self.from.map(|i| i as f64).unwrap_or(-1.0);
        let sequential = (target - from).abs() == 1.0;
        let center = if sequential && !self.compact {
            from + (target - from) * eased
        } else {
            target
        };
        let row = f64::from(self.size + 9);
        let start = (center.floor() as i64 - 2).max(-1);
        let end = (center.ceil() as i64 + 2).min(lines.len() as i64 - 1);
        for index in start..=end {
            let distance = (index as f64 - center).abs();
            let y = f64::from(height) / 2.0 + (index as f64 - center) * row;
            if y < -row || y > f64::from(height) + row {
                continue;
            }
            if self.compact && index as f64 != target {
                continue;
            }
            let alpha = (0.98 - distance * 0.63).clamp(0.0, 0.98);
            let alpha = if !sequential && progress < 1.0 {
                alpha * (0.3 + 0.7 * eased)
            } else {
                alpha
            };
            let text = if index == -1 {
                "Inicio instrumental"
            } else {
                let text = &lines[index as usize].text;
                if text.is_empty() {
                    "Instrumental"
                } else {
                    text
                }
            };
            let emphasis = (1.0 - distance).clamp(0.0, 1.0);
            let size = (13.0 + f64::from(self.size - 13) * emphasis).round() as i32;
            self.text(cr, text, width, y, size, alpha, emphasis > 0.5);
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn text(
        &self,
        cr: &cairo::Context,
        text: &str,
        width: i32,
        y: f64,
        size: i32,
        opacity: f64,
        bold: bool,
    ) {
        let layout = pangocairo::functions::create_layout(cr);
        let mut font = pango::FontDescription::from_string("Sans");
        font.set_absolute_size(f64::from(size) * f64::from(pango::SCALE));
        font.set_weight(if bold {
            pango::Weight::Semibold
        } else {
            pango::Weight::Normal
        });
        layout.set_font_description(Some(&font));
        layout.set_text(text);
        layout.set_width(width.max(1) * pango::SCALE);
        layout.set_ellipsize(pango::EllipsizeMode::End);
        layout.set_single_paragraph_mode(!self.square);
        if self.square {
            layout.set_wrap(pango::WrapMode::WordChar);
            layout.set_height(((y * 2.0 - 4.0).max(f64::from(size)) as i32) * pango::SCALE);
        }
        layout.set_alignment(pango::Alignment::Center);
        let (_, height) = layout.pixel_size();
        cr.set_source_rgba(0.0, 0.0, 0.0, opacity * 0.6);
        cr.move_to(0.0, y - f64::from(height) / 2.0 + 1.0);
        pangocairo::functions::show_layout(cr, &layout);
        cr.set_source_rgba(0.945, 0.945, 0.953, opacity);
        cr.move_to(0.0, y - f64::from(height) / 2.0);
        pangocairo::functions::show_layout(cr, &layout);
    }
}
