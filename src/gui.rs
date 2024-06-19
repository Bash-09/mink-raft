use egui::{Align2, Color32, Context, Frame, RichText, Vec2};

pub mod chat_windows;

pub fn fps_counter(gui_ctx: &Context, fps: u32, delta: f64) {
    let col = if fps < 60 {
        Color32::RED
    } else {
        Color32::GREEN
    };

    egui::Window::new("FPS Counter")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::LEFT_TOP, Vec2::new(5.0, 5.0))
        .frame(Frame::none())
        .show(gui_ctx, |ui| {
            ui.label(
                RichText::new(format!("FPS:  {fps}"))
                    .color(col)
                    .background_color(Color32::from_rgba_unmultiplied(0, 0, 0, 175))
                    .strong()
                    .heading(),
            );
            ui.label(
                RichText::new(format!("TIME: {:.2}ms", delta * 1000.0))
                    .color(col)
                    .background_color(Color32::from_rgba_unmultiplied(0, 0, 0, 175))
                    .strong()
                    .heading(),
            );
        });
}
