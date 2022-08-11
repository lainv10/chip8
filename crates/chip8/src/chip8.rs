use crate::processor::Processor;

mod clock;
pub mod graphics;
mod input;
mod memory;
mod processor;

/// Contains all the different components of the `Chip8` system, excluding the `Processor`.
#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct Bus {
    pub clock: clock::Clock,
    pub graphics: graphics::GraphicsBuffer,
    pub input: input::Input,
    pub memory: memory::Memory,
}

/// The main CHIP-8 interpreter state, contains all the components of the
/// CHIP-8 and procedures to interact with them at a high level.
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub struct Chip8 {
    pub processor: Processor,
    pub bus: Bus,
}

impl Chip8 {
    /// Create a new Chip8 instance.
    pub fn new() -> Self {
        Self {
            processor: Processor::new(),
            ..Default::default()
        }
    }

    /// Performs one execution step in the interpreter, cycling
    /// the processor and updating all state accordingly.
    pub fn step(&mut self) {
        self.bus.clock.update();
        self.processor.cycle(&mut self.bus);
    }

    /// Load the given ROM data into memory.
    /// This will resize the ROM in place to the correct length
    /// if it is too large/small.
    pub fn load_rom_data(&mut self, data: Vec<u8>) {
        self.bus.memory.load_rom(data);
    }

    /// Update the input state for the given key code.
    pub fn update_key_state(&mut self, key_code: u8, pressed: bool) {
        self.bus.input.update(key_code, pressed);
    }

    /// Reset the state of the `Chip8` instance.
    /// This does not reset the foreground/background colors of the `GraphicsBuffer`.
    pub fn reset(&mut self) {
        self.bus.graphics.clear();
        self.bus = Bus {
            graphics: self.bus.graphics,
            ..Default::default()
        };
        // create new processor with shift quirk and vblank wait settings retained
        let shift_quirk_enabled = self.processor.shift_quirk_enabled;
        let vblank_wait = self.processor.vblank_wait;
        self.processor = Processor::new();
        self.processor.shift_quirk_enabled = shift_quirk_enabled;
        self.processor.vblank_wait = vblank_wait;
    }

    /// Convenience method for resetting the `Chip8` and loading the given ROM.
    pub fn reset_and_load(&mut self, data: Vec<u8>) {
        self.reset();
        self.load_rom_data(data);
    }
}
