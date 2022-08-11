mod app;
mod audio;
mod gui;
mod renderer;

fn main() {
    eframe::run_native(
        "chip8!",
        eframe::NativeOptions {
            initial_window_size: Some(eframe::egui::vec2(1200.0, 800.0)),
            ..Default::default()
        },
        Box::new(|cc| Box::new(app::App::new(cc))),
    );
}
