use std::path::Path;

use crate::audio::AudioSystem;
use crate::gui::{Chip8Message, Gui};
use anyhow::Context;
use chip8::Chip8;

/// The main application state.
/// 
/// Handles interactions between the frontend [`Gui`] and the backend [`Chip8`].
pub struct App {
    pub chip8: Chip8,
    gui: Gui,
    // keep the audio system alive for as long as the app,
    // so the stream is not dropped.
    _audio: AudioSystem,
    steps_per_frame: u32,
    paused: bool,
    last_rom: Vec<u8>,
}

impl App {
    /// Create a new `App` instance.
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut chip8 = Chip8::new();
        let mut last_rom = Vec::new();

        if let Some(data) = Self::get_arg_rom() {
            chip8.load_rom_data(data.clone());
            last_rom = data;
        }

        let gui = Gui::new(cc);

        let audio = AudioSystem::new(chip8.bus.clock.sound_timer.clone())
            .expect("Failed to initialize audio system.");
        if audio.play().is_err() {
            log::error!("Failed to play audio stream.");
        }

        Self {
            gui,
            chip8,
            _audio: audio,
            steps_per_frame: 10,
            paused: false,
            last_rom,
        }
    }

    /// Get the ROM data from the path provided as the first argument when
    /// run from the command line.
    fn get_arg_rom() -> Option<Vec<u8>> {
        std::env::args().nth(1).and_then(|rom_path| {
            std::fs::read(&rom_path)
                .map_err(|e| log::error!("Failed to read ROM from {rom_path}: {e}"))
                .ok()
        })
    }

    /// Save `Chip8` state to a file specified by `path`.
    fn save_chip8(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let bytes = bincode::serialize(&self.chip8)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Load `Chip8` state from the given `path`.
    fn load_chip8(&mut self, path: impl AsRef<Path>) -> anyhow::Result<Chip8> {
        let bytes = std::fs::read(path)?;
        let chip8 = bincode::deserialize(&bytes)
            .context("Failed to deserialize Chip8 instance from file.")?;
        Ok(chip8)
    }

    /// Update the `Gui` and handle all state-changing messages.
    fn update_gui(&mut self, ctx: &eframe::egui::Context) {
        for message in self.gui.update(ctx, &self.chip8) {
            match message {
                Chip8Message::LoadRom(data) => {
                    self.chip8.reset_and_load(data.clone());
                    self.last_rom = data;
                }
                Chip8Message::ResetROM => {
                    // load the last loaded ROM
                    self.chip8.reset_and_load(self.last_rom.clone());
                }
                Chip8Message::SetForegroundColor(color) => {
                    self.chip8.bus.graphics.set_foreground_color(color)
                }
                Chip8Message::SetBackgroundColor(color) => {
                    self.chip8.bus.graphics.set_background_color(color)
                }
                Chip8Message::SetStepRate(steps) => self.steps_per_frame = steps,
                Chip8Message::UpdateKeys(key_updates) => {
                    for (key_code, pressed) in key_updates {
                        self.chip8.update_key_state(key_code, pressed);
                    }
                }
                Chip8Message::TogglePause => self.paused = !self.paused,
                Chip8Message::SaveState(path) => {
                    if let Err(e) = self.save_chip8(&path) {
                        log::error!("Failed to save Chip8 state to {}: {e}.", path.display());
                    }
                }
                Chip8Message::LoadState(path) => match self.load_chip8(&path) {
                    Ok(chip8) => self.chip8 = chip8,
                    Err(e) => {
                        log::error!("Failed to load Chip8 state from {}: {e}.", path.display())
                    }
                },
                Chip8Message::Step => self.chip8.step(),
            }
        }
    }
}

impl eframe::App for App {
    /// Updates the app and gui state and renders the GUI.
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // update chip8 state
        if !self.paused {
            for _ in 0..self.steps_per_frame {
                self.chip8.step();
            }
        }

        // update gui
        self.update_gui(ctx);

        // request another call to `update` right after this call
        ctx.request_repaint();
    }

    /// Clean up the gui on app exit.
    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.gui.clean_up(gl.unwrap());
    }
}
