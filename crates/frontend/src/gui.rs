use std::{
    path::PathBuf,
    sync::{atomic::Ordering, Arc, Mutex},
};

use chip8::{graphics::RGB8, Chip8};
use eframe::egui::{self, Context, Key, Ui};

use crate::renderer::Renderer;

/// Key mapping from a standard english keyboard to Chip8 key codes.
static KEY_MAP: [(Key, u8); 16] = [
    (Key::Num1, 0x1),
    (Key::Num2, 0x2),
    (Key::Num3, 0x3),
    (Key::Num4, 0xC),
    (Key::Q, 0x4),
    (Key::W, 0x5),
    (Key::E, 0x6),
    (Key::R, 0xD),
    (Key::A, 0x7),
    (Key::S, 0x8),
    (Key::D, 0x9),
    (Key::F, 0xE),
    (Key::Z, 0xA),
    (Key::X, 0x0),
    (Key::C, 0xB),
    (Key::V, 0xF),
];

/// A message sent from the GUI to the backend.
pub enum Chip8Message {
    /// Load the given ROM into the `Chip8`.
    LoadRom(Vec<u8>),

    /// Reset the currently loaded `Chip8` ROM.
    ResetROM,

    /// Set the foreground color of the `Chip8` graphics.
    SetForegroundColor(RGB8),

    /// Set the background color of the `Chip8` graphics.
    SetBackgroundColor(RGB8),

    /// Set the amount of steps the `Chip8` interpreter should
    /// advance on each frame.
    SetStepRate(u32),

    /// Enable/disable the shift quirk in the Chip8 instance
    SetShiftQuirk(bool),

    /// Enable/disable the vblank wait option in the Chip8 instance.
    SetVblankWait(bool),

    /// Update the key state of the `Chip8`. This contains
    /// a `Vec` of tuples, where each tuple contains a `u8` `Chip8` key
    /// code, as well as a `bool` representing if it is pressed down or not.
    UpdateKeys(Vec<(u8, bool)>),

    /// Toggle the app's paused state.
    TogglePause,

    /// Save the `Chip8` state and any `App` state to disk.
    SaveState(PathBuf),

    /// Load the `Chip8` state and any `App` state.
    LoadState(PathBuf),

    /// This indicates that the "step" button was clicked,
    /// meaning the user would like to execute one step of the interpreter.
    /// This should still step the interpreter even if the execution is paused.
    Step,
}

/// The current view in the `Gui`.
#[derive(Default)]
enum CurrentView {
    /// Show the `ScreenView`.
    #[default]
    Screen,

    /// Show the `DebugView`.
    Debug,
}

/// A user interface constructed with `egui`,
/// with a `glow` renderer used to display the `Chip8` state.
pub struct Gui {
    menu_panel: MenuPanel,
    config_window: ConfigWindow,
    screen_view: ScreenView,
    debug_view: DebugView,
    current_view: CurrentView,
}

impl Gui {
    /// Create a new `Gui` from an [`eframe::CreationContext`].
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let gl = cc.gl.as_ref().unwrap();

        Self {
            menu_panel: Default::default(),
            config_window: Default::default(),
            screen_view: ScreenView::new(gl),
            debug_view: Default::default(),
            current_view: Default::default(),
        }
    }

    /// Renders the next frame, which includes any UI updates as well
    /// as the `Chip8` graphics state.
    pub fn update(&mut self, ctx: &Context, chip8: &Chip8) -> Vec<Chip8Message> {
        let mut messages = Vec::new();

        let menu_response = self
            .menu_panel
            .update(ctx, &self.current_view, &mut messages);
        if menu_response.toggle_config {
            self.config_window.toggle_visibility();
        }
        if menu_response.reset {
            // send the color message to the chip8 backend so that
            // it restores the color settings for this session
            self.config_window.push_color_messages(&mut messages);
        }
        if menu_response.toggle_view {
            self.current_view = match self.current_view {
                CurrentView::Screen => CurrentView::Debug,
                CurrentView::Debug => CurrentView::Screen,
            }
        }
        if menu_response.toggle_pause {
            self.menu_panel.toggle_pause();
            self.debug_view.toggle_pause();
        }

        match self.current_view {
            CurrentView::Screen => self.screen_view.update(ctx, chip8),
            CurrentView::Debug => self.debug_view.update(ctx, &self.screen_view, chip8),
        }

        self.config_window.update(ctx, &mut messages);

        self.update_key_state(ctx, &mut messages);

        messages
    }

    /// Handles key events by updating the key
    /// state in the `Chip8` instance if necessary.
    fn update_key_state(&mut self, ctx: &Context, messages: &mut Vec<Chip8Message>) {
        let mut update = Vec::new();
        if !ctx.wants_keyboard_input() {
            let keys_down = &ctx.input().keys_down;
            for (key, key_code) in KEY_MAP {
                update.push((key_code, keys_down.contains(&key)));
            }
        }
        if !update.is_empty() {
            messages.push(Chip8Message::UpdateKeys(update));
        }
    }

    /// Clean up this Gui's state.
    pub fn clean_up(&self, gl: &eframe::glow::Context) {
        self.screen_view.clean_up(gl)
    }
}

#[derive(Default)]
struct MenuPanelResponse {
    /// Indicates whether the config window should be toggled.
    toggle_config: bool,

    /// Indicates that the `Gui` state should be reset. This is `true`
    /// when a new ROM has been loaded, or persisted state has been restored.
    reset: bool,

    /// Indicates to the `Gui` to toggle the current view.
    toggle_view: bool,

    /// Indicates to the `Gui` to toggle its pause state.
    toggle_pause: bool,
}

/// A menu panel intended to be placed near the top of the window,
/// shows Ui widgets for selecting roms, saving state, etc.
#[derive(Default)]
struct MenuPanel {
    paused: bool,
}

impl MenuPanel {
    /// Update the Ui of this `MenuPanel`. This will return a [`MenuPanelResponse`] indicating
    /// how other Ui components should be updated.
    fn update(
        &mut self,
        ctx: &Context,
        view: &CurrentView,
        messages: &mut Vec<Chip8Message>,
    ) -> MenuPanelResponse {
        let mut response = MenuPanelResponse::default();
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.button("\u{1F4C1} Open ROM").clicked() {
                    if let Some(data) = Self::load_file_from_dialog() {
                        messages.push(Chip8Message::LoadRom(data));
                        response.reset = true;
                    }
                };

                if ui.button("\u{2699} Config").clicked() {
                    response.toggle_config = true;
                }

                ui.separator();

                if ui.button("\u{2B06} Save State").clicked() {
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        messages.push(Chip8Message::SaveState(path));
                    }
                }

                if ui.button("\u{2B07} Load state").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        messages.push(Chip8Message::LoadState(path));
                        response.reset = true;
                    }
                }

                ui.separator();

                Self::draw_view_toggle(view, ui, &mut response);

                self.draw_execution_controls(ui, messages, &mut response);
            });
        });
        response
    }

    /// Draw the button that toggles the `Gui` view.
    fn draw_view_toggle(view: &CurrentView, ui: &mut Ui, response: &mut MenuPanelResponse) {
        let label = match view {
            CurrentView::Screen => "\u{1F6E0} Debug",
            CurrentView::Debug => "\u{1F4FA} Screen",
        };
        if ui.button(label).clicked() {
            response.toggle_view = true;
        }
    }

    /// Draw the buttons that control the Chip8 program's execution.
    fn draw_execution_controls(
        &mut self,
        ui: &mut Ui,
        messages: &mut Vec<Chip8Message>,
        response: &mut MenuPanelResponse,
    ) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let play_pause_label = if self.paused {
                "\u{23F5} Play"
            } else {
                "\u{23F8} Pause"
            };
            if ui.button(play_pause_label).clicked() {
                messages.push(Chip8Message::TogglePause);
                response.toggle_pause = true;
            }

            if ui.button("\u{27A1} Step").clicked() {
                messages.push(Chip8Message::Step);
            }

            if ui.button("\u{21BB} Reset").clicked() {
                messages.push(Chip8Message::ResetROM);
                response.reset = true;
            }
        });
    }

    /// Toggle the `MenuPanel` paused state.
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Retrieves data from a file selected by a file dialog.
    /// Returns `None` if the chosen file cannot be read, or if the user
    /// cancelled the operation. Otherwise, returns the file's data as a `Vec<u8>`.
    fn load_file_from_dialog() -> Option<Vec<u8>> {
        rfd::FileDialog::new().pick_file().and_then(|file| {
            std::fs::read(file)
                .map_err(|e| log::error!("Failed to load ROM file: {}", e))
                .ok()
        })
    }
}

/// A screen panel that displays the Chip8 graphics state with a `Renderer`.
/// Note that this component uses an [`egui::CentralPanel`], and should be added
/// after all other panels.
struct ScreenView {
    renderer: Arc<Mutex<Renderer>>,
}

impl ScreenView {
    fn new(gl: &eframe::glow::Context) -> Self {
        Self {
            renderer: Arc::new(Mutex::new(Renderer::new(gl))),
        }
    }

    /// Update and draw this `ScreenView`. This creates a central panel, therefore it
    /// should be called after all other panels are drawn.
    fn update(&self, ctx: &Context, chip8: &Chip8) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().inner_margin(egui::vec2(0.0, 0.0)))
            .show(ctx, |ui| {
                self.draw_chip8_renderer(ui, chip8);
            });
    }

    /// Clean up the renderer's GL context.
    fn clean_up(&self, gl: &eframe::glow::Context) {
        self.renderer.lock().unwrap().clean_up(gl);
    }

    /// Draw the `Chip8` graphics state onto a `Ui` object.
    ///
    /// This uses the rest of the available size in the `Ui`.
    fn draw_chip8_renderer(&self, ui: &mut Ui, chip8: &Chip8) {
        let renderer = self.renderer.clone();
        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        ui.available_size(),
                        egui::Sense::focusable_noninteractive(),
                    );
                    let graphics_buffer = chip8.bus.graphics.as_rgb8();
                    let callback = egui::PaintCallback {
                        rect,
                        callback: Arc::new(eframe::egui_glow::CallbackFn::new(
                            move |_, painter| {
                                // at this point, egui has set the rect viewport,
                                // so all we do is render like normal
                                renderer
                                    .lock()
                                    .unwrap()
                                    .render(painter.gl(), graphics_buffer.as_slice());
                            },
                        )),
                    };
                    ui.painter().add(callback);
                });
            },
        );
    }
}

/// A configuration window which allows the user to customize
/// certain aspects of the `Chip8` instance.
struct ConfigWindow {
    visible: bool,
    foreground_rgb: [u8; 3],
    background_rgb: [u8; 3],
    steps_per_frame: u32,
    shift_quirk_enabled: bool,
    vblank_wait_enabled: bool,
}

impl Default for ConfigWindow {
    fn default() -> Self {
        Self {
            visible: false,
            foreground_rgb: chip8::graphics::DEFAULT_FOREGROUND.0,
            background_rgb: chip8::graphics::DEFAULT_BACKGROUND.0,
            steps_per_frame: crate::app::DEFAULT_STEPS_PER_FRAME,
            shift_quirk_enabled: false,
            vblank_wait_enabled: false,
        }
    }
}

impl ConfigWindow {
    /// Update and render the `ConfigWindow` to the given `Context`.
    /// This will append any GUI messages to `messages` if the `Chip8` state should be updated.
    fn update(&mut self, ctx: &Context, messages: &mut Vec<Chip8Message>) {
        egui::Window::new("Config")
            .open(&mut self.visible)
            .show(ctx, |ui| {
                egui::Grid::new("config_grid").show(ui, |ui| {
                    // foreground color selector
                    ui.label("Foreground Color");
                    if ui
                        .color_edit_button_srgb(&mut self.foreground_rgb)
                        .changed()
                    {
                        messages.push(Chip8Message::SetForegroundColor(RGB8(self.foreground_rgb)));
                    }
                    ui.end_row();

                    // background color selector
                    ui.label("Background Color");
                    if ui
                        .color_edit_button_srgb(&mut self.background_rgb)
                        .changed()
                    {
                        messages.push(Chip8Message::SetBackgroundColor(RGB8(self.background_rgb)));
                    }
                    ui.end_row();

                    // step rate selector
                    ui.label("Steps Per Frame");
                    let drag = egui::DragValue::new(&mut self.steps_per_frame);
                    if ui.add(drag).changed() {
                        messages.push(Chip8Message::SetStepRate(self.steps_per_frame));
                    }
                    ui.end_row();

                    ui.label("Enable Shift Quirk");
                    let shift_quirk_checkbox = ui.checkbox(&mut self.shift_quirk_enabled, "");
                    if shift_quirk_checkbox.changed() {
                        messages.push(Chip8Message::SetShiftQuirk(self.shift_quirk_enabled))
                    }
                    shift_quirk_checkbox.on_hover_text(
                        "Enable/disable the shift quirk in the interpreter. \
                        Try toggling this if a program isn't working as expected.",
                    );
                    ui.end_row();

                    ui.label("Enable VBLANK Wait");
                    let vblank_wait_checkbox = ui.checkbox(&mut self.vblank_wait_enabled, "");
                    if vblank_wait_checkbox.changed() {
                        messages.push(Chip8Message::SetVblankWait(self.vblank_wait_enabled));
                    }
                    vblank_wait_checkbox.on_hover_text(
                        "Enable/disable waiting for the vertical blank interrupt before drawing a sprite. \
                        This will limit the amount of sprite draw calls to 60 calls per second."
                    );
                    ui.end_row();
                });
            });
    }

    /// Push both foreground and background color update messages to `messages`.
    fn push_color_messages(&self, messages: &mut Vec<Chip8Message>) {
        messages.push(Chip8Message::SetForegroundColor(RGB8(self.foreground_rgb)));
        messages.push(Chip8Message::SetBackgroundColor(RGB8(self.background_rgb)));
    }

    /// Toggle the visibility of this `ConfigWindow`,
    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}

/// A debug screen showing the details of the underlying state of the `Chip8`,
/// such as registers, stack memory, instructions, and timers.
#[derive(Default)]
struct DebugView {
    /// Mirrors the paused state of the `App`. This is used to determine
    /// whether the instructions window should be drawn with every instruction or not.
    paused: bool,
}

impl DebugView {
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Update the `DebugView`. This will draw all windows on the given context,
    /// and should be called last.
    fn update(&mut self, ctx: &Context, screen: &ScreenView, chip8: &Chip8) {
        Self::draw_registers_window(ctx, chip8);
        Self::draw_stack_window(ctx, chip8);
        Self::draw_screen_window(ctx, screen, chip8);
        Self::draw_timers_window(ctx, chip8);
        Self::draw_key_window(ctx, chip8);
        self.draw_instructions_window(ctx, chip8);
    }

    /// Draw a window that shows every register in the given `Chip8`.
    fn draw_registers_window(ctx: &Context, chip8: &Chip8) {
        egui::Window::new("Registers").show(ctx, |ui| {
            egui::Grid::new("registers_grid")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    ui.heading("I");
                    ui.heading(format!("{:#06X}", chip8.processor.i));
                    ui.end_row();
                    for (i, register) in chip8.processor.v.iter().enumerate() {
                        ui.heading(format!("V{i:X}"));
                        ui.heading(register.to_string());
                        ui.end_row();
                    }
                })
        });
    }

    /// Draw a window that shows information about the stack
    /// (stack pointer, stack memory) of the given `Chip8`.
    fn draw_stack_window(ctx: &Context, chip8: &Chip8) {
        egui::Window::new("Stack").show(ctx, |ui| {
            ui.heading(format!("Pointer: {}", chip8.processor.sp));
            egui::Grid::new("Stack grid")
                .striped(true)
                .num_columns(2)
                .show(ui, |ui| {
                    for (i, value) in chip8.processor.stack.iter().enumerate() {
                        ui.heading(i.to_string());
                        ui.heading(format!("{value:#06X}"));
                        ui.end_row();
                    }
                });
        });
    }

    /// Draw a window that shows the instructions executed by the `Chip8`,
    /// in their opcode form as well as a more descriptive readable form.
    fn draw_instructions_window(&mut self, ctx: &Context, chip8: &Chip8) {
        egui::Window::new("Instructions").show(ctx, |ui| {
            if !self.paused {
                ui.heading("Pause the execution to inspect instructions.");
                return;
            }

            ui.heading(format!(
                "Current Program Counter: {:#06X}",
                chip8.processor.pc
            ));
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("instr_grid")
                        .striped(true)
                        .num_columns(3)
                        .show(ui, |ui| {
                            ui.heading("Address");
                            ui.add(egui::Separator::default().vertical());
                            ui.heading("Opcode");
                            ui.add(egui::Separator::default().vertical());
                            ui.heading("Description");
                            ui.end_row();
                            for instr in &chip8.processor.instructions {
                                ui.heading(format!("{:#06X}", instr.address));
                                ui.add(egui::Separator::default().vertical());
                                ui.heading(format!("{:#06X}", instr.opcode));
                                ui.add(egui::Separator::default().vertical());
                                ui.heading(&instr.display);
                                ui.end_row();
                            }
                        });
                });
        });
    }

    /// Draw a window that displays the `Chip8` graphics state.
    fn draw_screen_window(ctx: &Context, screen: &ScreenView, chip8: &Chip8) {
        egui::Window::new("Screen")
            .default_size(egui::vec2(500.0, 250.0))
            .show(ctx, |ui| {
                screen.draw_chip8_renderer(ui, chip8);
            });
    }

    /// Draw a window that displays the state of both the delay and sound
    /// timer of the given `Chip8`.
    fn draw_timers_window(ctx: &Context, chip8: &Chip8) {
        egui::Window::new("Timers").show(ctx, |ui| {
            egui::Grid::new("timer_grid").show(ui, |ui| {
                ui.heading("Delay");
                ui.heading(chip8.bus.clock.delay_timer.to_string());
                ui.end_row();
                ui.heading("Sound");
                ui.heading(
                    chip8
                        .bus
                        .clock
                        .sound_timer
                        .load(Ordering::SeqCst)
                        .to_string(),
                );
            });
        });
    }

    /// Draw a window that displays the current pressed state of the keys
    /// in the given `Chip8`.
    fn draw_key_window(ctx: &Context, chip8: &Chip8) {
        egui::Window::new("Keys").show(ctx, |ui| {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Heading);
            let key = |ui: &mut Ui, code: u8| {
                ui.set_enabled(false);
                let label = egui::SelectableLabel::new(
                    chip8.bus.input.is_key_pressed(code),
                    format!("{code:X}"),
                );
                
                ui.add(label);
            };

            egui::Grid::new("key_grid").show(ui, |ui| {
                // layout the keys manually
                key(ui, 1);
                key(ui, 2);
                key(ui, 3);
                key(ui, 0xC);
                ui.end_row();

                key(ui, 4);
                key(ui, 5);
                key(ui, 6);
                key(ui, 0xD);
                ui.end_row();

                key(ui, 7);
                key(ui, 8);
                key(ui, 9);
                key(ui, 0xE);
                ui.end_row();

                key(ui, 0xA);
                key(ui, 0);
                key(ui, 0xB);
                key(ui, 0xF);
            });
        });
    }
}
