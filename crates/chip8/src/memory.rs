use std::ops::{Index, IndexMut};

/// Total size of the Chip8 memory.
const MEMORY_SIZE: usize = 4096;

/// The size of the interpreter.
/// 
/// This is really only used to determine where 
/// the program memory should start.
const INTERPRETER_SIZE: usize = 512;

/// Built in Chip8 font data. This will be stored in the
/// interpreter's memory.
const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

/// The memory of the `Chip8`.
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct Memory {
    #[cfg_attr(feature = "persistence", serde(with = "serde_big_array::BigArray"))]
    memory: [u8; MEMORY_SIZE],
}

impl Default for Memory {
    fn default() -> Self {
        let mut memory = [0; MEMORY_SIZE];
        memory[..80].clone_from_slice(&FONT);
        Self { memory }
    }
}

impl Memory {
    /// Create a new `Memory` object filled with zeroes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the ROM bytes from `data`.
    ///
    /// If this is smaller than the program size
    /// (`MEMORY_SIZE - INTERPRETER_SIZE`), then the remaining
    /// memory will be filled with zeroes.
    pub fn load_rom(&mut self, mut data: Vec<u8>) {
        data.resize(MEMORY_SIZE - INTERPRETER_SIZE, 0);
        self.memory[INTERPRETER_SIZE..=0xFFF].clone_from_slice(&data);
    }
}

impl Index<usize> for Memory {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.memory[index]
    }
}

impl IndexMut<usize> for Memory {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.memory[index]
    }
}
