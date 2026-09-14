//! Deterministic background for checking compositor refraction, not an app mockup.
use gtk::{cairo, prelude::*};

fn main() -> gtk::glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id("io.github.paulneja.Vitrilyr.OpticalTest")
        .build();
    app.connect_activate(|app| {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Liquid Glass optical test")
            .decorated(false)
            .build();
        let scene = gtk::DrawingArea::new();
        scene.set_draw_func(|_, cr, width, height| {
            cr.set_source_rgb(0.08, 0.085, 0.095);
            let _ = cr.paint();
            let colors = [(0.16, 0.64, 0.58), (0.94, 0.35, 0.39), (0.88, 0.77, 0.27)];
            for (i, (r, g, b)) in colors.into_iter().enumerate() {
                cr.set_source_rgb(r, g, b);
                cr.move_to(f64::from(width) * (0.12 + i as f64 * 0.34), 0.0);
                cr.line_to(f64::from(width) * (0.26 + i as f64 * 0.34), 0.0);
                cr.line_to(
                    f64::from(width) * (-0.14 + i as f64 * 0.34),
                    f64::from(height),
                );
                cr.line_to(
                    f64::from(width) * (-0.28 + i as f64 * 0.34),
                    f64::from(height),
                );
                cr.close_path();
                let _ = cr.fill();
            }
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.35);
            cr.set_line_width(1.0);
            for x in (0..width).step_by(24) {
                cr.move_to(f64::from(x) + 0.5, 0.0);
                cr.line_to(f64::from(x) + 0.5, f64::from(height));
            }
            for y in (0..height).step_by(24) {
                cr.move_to(0.0, f64::from(y) + 0.5);
                cr.line_to(f64::from(width), f64::from(y) + 0.5);
            }
            let _ = cr.stroke();
            cr.select_font_face(
                "sans-serif",
                cairo::FontSlant::Normal,
                cairo::FontWeight::Normal,
            );
            cr.set_font_size(18.0);
            cr.set_source_rgb(0.94, 0.94, 0.95);
            cr.move_to(36.0, f64::from(height) - 36.0);
            let _ = cr.show_text("VITRILYR / OPTICAL TEST");
        });
        window.set_child(Some(&scene));
        window.fullscreen();
        window.present();
    });
    app.run()
}
