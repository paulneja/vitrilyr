use gtk::{cairo, prelude::*};
use lyricglass::config::Config;
use std::{cell::RefCell, f64::consts::PI, rc::Rc};

struct State {
    radius: f64,
    opacity: f64,
    reflection: f64,
    pointer: f64,
    target: f64,
    animate: bool,
}

pub struct Material {
    pub widget: gtk::DrawingArea,
    state: Rc<RefCell<State>>,
}

impl Material {
    pub fn new(root: &gtk::Overlay) -> Self {
        let widget = gtk::DrawingArea::new();
        widget.set_can_target(false);
        widget.set_hexpand(true);
        widget.set_vexpand(true);
        let state = Rc::new(RefCell::new(State {
            radius: 32.0,
            opacity: 0.3,
            reflection: 0.65,
            pointer: 0.3,
            target: 0.3,
            animate: true,
        }));
        let paint = state.clone();
        widget.set_draw_func(move |_, cr, width, height| {
            draw(cr, f64::from(width), f64::from(height), &paint.borrow());
        });
        let motion = gtk::EventControllerMotion::new();
        let state_motion = state.clone();
        let weak = root.downgrade();
        motion.connect_motion(move |_, x, _| {
            if let Some(root) = weak.upgrade() {
                state_motion.borrow_mut().target =
                    (x / f64::from(root.width().max(1))).clamp(0.0, 1.0);
            }
        });
        let state_leave = state.clone();
        motion.connect_leave(move |_| state_leave.borrow_mut().target = 0.3);
        root.add_controller(motion);
        Self { widget, state }
    }
    pub fn configure(&self, config: &Config, animations: bool) {
        self.widget
            .set_visible(config.liquid && config.theme == "glass");
        let mut state = self.state.borrow_mut();
        state.radius = f64::from(config.liquid_radius);
        state.opacity = config.opacity;
        state.reflection = config.reflections;
        state.animate = animations;
        self.widget.queue_draw();
    }
    pub fn tick(&self) {
        if !self.widget.is_visible() {
            return;
        }
        let mut state = self.state.borrow_mut();
        let delta = state.target - state.pointer;
        if delta.abs() > 0.001 {
            state.pointer += delta * if state.animate { 0.3 } else { 1.0 };
            self.widget.queue_draw();
        }
    }
}

fn rounded(cr: &cairo::Context, w: f64, h: f64, radius: f64, inset: f64) {
    let r = (radius - inset)
        .max(0.0)
        .min((w.min(h) / 2.0 - inset).max(0.0));
    cr.new_sub_path();
    cr.arc(w - inset - r, inset + r, r, -PI / 2.0, 0.0);
    cr.arc(w - inset - r, h - inset - r, r, 0.0, PI / 2.0);
    cr.arc(inset + r, h - inset - r, r, PI / 2.0, PI);
    cr.arc(inset + r, inset + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}

fn draw(cr: &cairo::Context, w: f64, h: f64, s: &State) {
    if w < 2.0 || h < 2.0 {
        return;
    }
    rounded(cr, w, h, s.radius, 0.5);
    // The compositor supplies the refracted scene. This layer is only the glass surface.
    cr.set_source_rgba(0.045, 0.048, 0.055, 0.14 + s.opacity * 0.43);
    let _ = cr.fill_preserve();
    let sheen = cairo::LinearGradient::new(w * s.pointer, 0.0, w * (1.0 - s.pointer), h);
    sheen.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.18 * s.reflection);
    sheen.add_color_stop_rgba(0.38, 1.0, 1.0, 1.0, 0.025 * s.reflection);
    sheen.add_color_stop_rgba(0.7, 0.0, 0.0, 0.0, 0.025);
    sheen.add_color_stop_rgba(1.0, 1.0, 1.0, 1.0, 0.08 * s.reflection);
    let _ = cr.set_source(&sheen);
    let _ = cr.fill();
    let rim = cairo::LinearGradient::new(w * s.pointer, 0.0, w * (1.0 - s.pointer), h);
    rim.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.25 + s.reflection * 0.65);
    rim.add_color_stop_rgba(0.27, 1.0, 1.0, 1.0, 0.12);
    rim.add_color_stop_rgba(0.58, 1.0, 1.0, 1.0, 0.03);
    rim.add_color_stop_rgba(1.0, 1.0, 1.0, 1.0, 0.12 + s.reflection * 0.5);
    rounded(cr, w, h, s.radius, 0.75);
    let _ = cr.set_source(&rim);
    cr.set_line_width(1.5);
    let _ = cr.stroke();
    rounded(cr, w, h, s.radius, 2.5);
    cr.set_source_rgba(1.0, 1.0, 1.0, s.reflection * 0.07);
    cr.set_line_width(1.0);
    let _ = cr.stroke();
}
