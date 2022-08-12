// hide the console window on non-debug builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod gui;
mod renderer;

fn main() {
    setup_logger();
    run_native();
}

/// Initialize and run a native [`eframe`] app.
fn run_native() {
    eframe::run_native(
        "chip8!",
        eframe::NativeOptions {
            initial_window_size: Some(eframe::egui::vec2(1200.0, 800.0)),
            ..Default::default()
        },
        Box::new(|cc| Box::new(app::App::new(cc))),
    );
}

/// Setup the [`fern`] logger.
fn setup_logger() {
    #[cfg(debug_assertions)]
    let level = log::LevelFilter::Debug;

    #[cfg(not(debug_assertions))]
    let level = log::LevelFilter::Info;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
        .unwrap();
}
