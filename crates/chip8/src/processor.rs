use super::Bus;

/// default starting point for most Chip8 programs
const STARTING_PC: usize = 0x200;

/// Describes how the program counter should be updated after
/// executing an instruction.
enum PCUpdate {
    /// Go directly to the next instruction (pc + 2)
    Next,

    /// Skip the next instruction (pc + 4).
    SkipNext,

    /// Jump to the given address.
    Jump(usize),
}

#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub struct Processor {
    /// Vx registers
    pub v: [u8; 16],

    /// Index register
    pub i: usize,

    /// Program counter
    pub pc: usize,

    /// Stack pointer
    pub sp: usize,

    /// Stack memory
    pub stack: [usize; 16],

    /// The last opcode that has been processed
    pub last_opcode: usize,

    /// The address of the last opcode that has been processed
    pub last_opcode_addr: usize,

    /// A display string explaining what the current opcode is doing
    pub display: String,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            pc: STARTING_PC,
            ..Default::default()
        }
    }

    pub fn cycle(&mut self, bus: &mut Bus) {
        // if the input system is waiting for a key, don't process any opcodes
        if bus.input.waiting() {
            return;
        } else if let Some(request) = bus.input.request_response() {
            self.v[request.register] = request.key_code;
        }

        if self.pc >= 4096 {
            return;
        }
        // get the next two bytes and combine into one two-byte instruction
        let opcode = (usize::from(bus.memory[self.pc]) << 8) | usize::from(bus.memory[self.pc + 1]);

        self.last_opcode = opcode;
        self.last_opcode_addr = self.pc;

        match self.process_opcode(opcode, bus) {
            PCUpdate::Next => self.pc += 2,
            PCUpdate::SkipNext => self.pc += 4,
            PCUpdate::Jump(addr) => self.pc = addr,
        }
    }

    fn process_opcode(&mut self, opcode: usize, bus: &mut Bus) -> PCUpdate {
        // define some commonly used variables
        let x = (opcode & 0x0F00) >> 8;
        let y = (opcode & 0x00F0) >> 4;
        let nn = u8::try_from(opcode & 0x00FF).unwrap();
        let nnn = opcode & 0x0FFF;

        match (opcode & 0xF000) >> 12 {
            // 0___
            0x0 => match opcode & 0x000F {
                // 00E0
                0x0000 => {
                    bus.graphics.clear();
                    self.display = "Clear the screen".into();
                    PCUpdate::Next
                }

                // 00EE
                0x000E => {
                    self.sp -= 1;
                    self.display = format!("Return to addr {:#06X}", self.stack[self.sp]);
                    PCUpdate::Jump(self.stack[self.sp])
                }

                // invalid
                _ => {
                    log::error!("Invalid 0x0___ instruction: {opcode:X}");
                    self.display = "Invalid instruction".into();
                    PCUpdate::Next
                }
            },

            // 1nnn
            0x1 => {
                self.display = format!("Jump to addr {nnn:#06X}");
                PCUpdate::Jump(nnn)
            }

            // 2nnn
            0x2 => {
                self.stack[self.sp] = self.pc + 2;
                self.sp += 1;
                self.display = format!("Call subroutine at {nnn:#06X}");
                PCUpdate::Jump(nnn)
            }

            // 3xnn
            0x3 => {
                self.display = format!("If V{x:X} ({}) == {nn}, skip next instr", self.v[x]);
                if self.v[x] == nn {
                    PCUpdate::SkipNext
                } else {
                    PCUpdate::Next
                }
            }

            // 4Xnn
            0x4 => {
                self.display = format!("If V{x:X} ({}) != {nn}, skip next instr", self.v[x]);
                if self.v[x] != nn {
                    PCUpdate::SkipNext
                } else {
                    PCUpdate::Next
                }
            }

            // 5xy0
            0x5 => {
                self.display = format!(
                    "If V{x:X} ({}) == V{y:X} ({}), skip next instr",
                    self.v[x], self.v[y]
                );
                if self.v[x] == self.v[y] {
                    PCUpdate::SkipNext
                } else {
                    PCUpdate::Next
                }
            }

            // 6xnn
            0x6 => {
                self.display = format!("Set V{x:X} to {nn}");
                self.v[x] = nn;
                PCUpdate::Next
            }

            // 7xnn
            0x7 => {
                self.display = format!("Add {nn} to V{x:X}");
                self.v[x] = self.v[x].wrapping_add(nn);
                PCUpdate::Next
            }

            // 8___
            0x8 => match opcode & 0x000F {
                // 8xy0
                0x0 => {
                    self.display = format!("Set V{x:X} to V{y:X} ({})", self.v[y]);
                    self.v[x] = self.v[y];
                    PCUpdate::Next
                }

                // 8xy1
                0x1 => {
                    self.display = format!(
                        "Set V{x:X} to V{x:X} OR V{y:X} ({:2X} OR {:2X})",
                        self.v[x], self.v[y]
                    );
                    self.v[x] |= self.v[y];
                    self.v[0xF] = 0;
                    PCUpdate::Next
                }

                // 8xy2
                0x2 => {
                    self.display = format!(
                        "Set V{x:X} to V{x:X} AND V{y:X} ({:2X} AND {:2X})",
                        self.v[x], self.v[y]
                    );
                    self.v[x] &= self.v[y];
                    self.v[0xF] = 0;
                    PCUpdate::Next
                }

                // 8xy3
                0x3 => {
                    self.display = format!(
                        "Set V{x:X} to V{x:X} XOR V{y:X} ({:2X} XOR {:2X})",
                        self.v[x], self.v[y]
                    );
                    self.v[x] ^= self.v[y];
                    self.v[0xF] = 0;
                    PCUpdate::Next
                }

                // 8xy4
                0x4 => {
                    let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
                    self.display = format!(
                        "Set V{x:X} to ({} + {}), VF = {}",
                        self.v[x],
                        self.v[y],
                        u8::from(overflow)
                    );
                    self.v[x] = result;
                    self.v[0xF] = u8::from(overflow);
                    PCUpdate::Next
                }

                // 8xy5
                0x5 => {
                    let (result, overflow) = self.v[x].overflowing_sub(self.v[y]);
                    self.display = format!(
                        "Set V{x:X} to ({} - {}), VF = {}",
                        self.v[x],
                        self.v[y],
                        u8::from(!overflow)
                    );
                    self.v[x] = result;
                    self.v[0xF] = u8::from(!overflow);
                    PCUpdate::Next
                }

                // 8xy6
                0x6 => {
                    let overflow = self.v[x] & 1;
                    self.display = format!("V{x:X} shifted one right, VF = {}", overflow);
                    self.v[x] >>= 1;
                    self.v[0xF] = overflow;
                    PCUpdate::Next
                }

                // 8xy7
                0x7 => {
                    let (result, overflow) = self.v[y].overflowing_sub(self.v[x]);
                    self.display = format!(
                        "Set V{x:X} to ({} - {}), VF = {}",
                        self.v[y],
                        self.v[x],
                        u8::from(!overflow)
                    );
                    self.v[x] = result;
                    self.v[0xF] = u8::from(!overflow);
                    PCUpdate::Next
                }

                // 8xyE
                0xE => {
                    let overflow = (self.v[x] & 0x80) >> 7;
                    self.display = format!("V{x:X} shifted one left, VF = {}", overflow);
                    self.v[x] <<= 1;
                    // self.v[y] = self.v[x] << 1;
                    self.v[0xF] = overflow;
                    PCUpdate::Next
                }

                // invalid
                _ => {
                    self.display = "Invalid instruction".into();
                    log::error!("Invalid 8XY_ instruction: {opcode:X}");
                    PCUpdate::Next
                }
            },

            // 9xy0
            9 => {
                self.display = format!(
                    "If V{x:X} ({}) != V{y:X} ({}), skip next instr",
                    self.v[x], self.v[y]
                );
                if self.v[x] != self.v[y] {
                    PCUpdate::SkipNext
                } else {
                    PCUpdate::Next
                }
            }

            // Annn
            0xA => {
                self.display = format!("Set I register to {nnn:#06X}");
                self.i = nnn;
                PCUpdate::Next
            }

            // Bnnn
            0xB => {
                self.display = format!("Jump to {nnn:#06X} + {:#06X}", self.v[0]);
                PCUpdate::Jump(nnn + usize::from(self.v[0]))
            }

            // Cxnn
            0xC => {
                let mut buf = [0u8; 1];
                getrandom::getrandom(&mut buf).unwrap();
                self.display = format!("Set V{x:X} to {} [rand] AND {nn:#X}", buf[0]);
                self.v[x] = buf[0] & nn;
                PCUpdate::Next
            }

            // Dxyn
            0xD => {
                let n = opcode & 0xF;
                let x = self.v[x].into();
                let y: usize = self.v[y].into();
                self.display = format!(
                    "Draw {n} byte sprite from addr {:#06X} at point ({x}, {y})",
                    self.i
                );
                let mut collision = false;
                for i in 0..n {
                    let data = bus.memory[self.i + i];
                    collision |= bus.graphics.draw_byte(x, y + i, data);
                }
                self.v[0xF] = collision.into();
                PCUpdate::Next
            }

            // E___
            0xE => match opcode & 0x000F {
                // Ex9E
                0x000E => {
                    let pressed = bus.input.is_key_pressed(self.v[x]);
                    self.display = format!(
                        "Skip instr if key {:#X} pressed ({pressed})",
                        self.v[x]
                    );
                    if pressed {
                        PCUpdate::SkipNext
                    } else {
                        PCUpdate::Next
                    }
                }

                // ExA1
                0x0001 => {
                    let not_pressed = !bus.input.is_key_pressed(self.v[x]);
                    self.display = format!(
                        "Skip next instr if key code {:#X} not pressed ({not_pressed})",
                        self.v[x]
                    );
                    if not_pressed {
                        PCUpdate::SkipNext
                    } else {
                        PCUpdate::Next
                    }
                }

                // invalid
                _ => {
                    self.display = "Invalid instruction".into();
                    log::error!("Invalid EX__ instruction: {opcode:X}");
                    PCUpdate::Next
                }
            },

            // F___
            0xF => match opcode & 0x00FF {
                // Fx07
                0x0007 => {
                    self.display = format!("Set V{x:X} to delay timer ({})", bus.clock.delay_timer);
                    self.v[x] = bus.clock.delay_timer;
                    PCUpdate::Next
                }

                // Fx0A
                0x000A => {
                    self.display = format!("Store next key press in V{x:X}");
                    bus.input.request_key_press(x);
                    PCUpdate::Next
                }

                // Fx15
                0x0015 => {
                    self.display = format!("Set delay timer to V{x:X} ({})", self.v[x]);
                    bus.clock.delay_timer = self.v[x];
                    PCUpdate::Next
                }

                // Fx18
                0x0018 => {
                    self.display = format!("Set sound timer to V{x:X} ({})", self.v[x]);
                    (*bus.clock.sound_timer).store(self.v[x], std::sync::atomic::Ordering::SeqCst);
                    PCUpdate::Next
                }

                // Fx1E
                0x001E => {
                    self.display = format!("Set I to I + V{x:X}");
                    self.i += usize::from(self.v[x]);
                    PCUpdate::Next
                }

                // Fx29
                0x0029 => {
                    self.display = format!("Set I to addr of sprite digit {}", self.v[x]);
                    // set I to the sprite address of the digit in Vx
                    self.i = 5 * usize::from(self.v[x]);
                    PCUpdate::Next
                }

                // Fx33
                0x0033 => {
                    self.display = format!("Store BCD of {} starting at I", self.v[x]);
                    // store BCD representation of decimal in Vx
                    bus.memory[self.i] = (self.v[x] / 100) % 10;
                    bus.memory[self.i + 1] = (self.v[x] / 10) % 10;
                    bus.memory[self.i + 2] = self.v[x] % 10;
                    PCUpdate::Next
                }

                // Fx55
                0x0055 => {
                    self.display = format!("Store V0 to V{x:X} starting at I");
                    for i in 0..=x {
                        bus.memory[self.i] = self.v[i];
                        self.i += 1;
                    }
                    PCUpdate::Next
                }

                // Fx65
                0x0065 => {
                    self.display = format!("Read memory at I into V0 to V{x:X}");
                    for i in 0..=x {
                        self.v[i] = bus.memory[self.i];
                        self.i += 1;
                    }
                    PCUpdate::Next
                }

                // invalid
                _ => {
                    self.display = "Invalid instruction".into();
                    log::error!("Invalid FX__ instruction: {opcode:X}");
                    PCUpdate::Next
                }
            },

            // invalid
            _ => {
                self.display = "Invalid instruction".into();
                log::error!("Unknown opcode: {opcode:X}");
                PCUpdate::Next
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Bus;

    use super::{Processor, STARTING_PC};

    /// Helper function that executes a single opcode on the given
    /// 'Processor` and a new `Bus`.
    fn test_op_with(opcode: u16, processor: &mut Processor) {
        let mut bus = Bus::default();
        bus.memory[processor.pc] = u8::try_from(opcode >> 8).unwrap();
        bus.memory[processor.pc + 1] = u8::try_from(opcode & 0xFF).unwrap();
        processor.cycle(&mut bus);
    }

    /// Helper function that executes a single opcode on a new `Processor`.
    ///
    /// Returns the `Processor` the opcode was executed on so that its
    /// state can be inspected.
    fn test_op(opcode: u16) -> Processor {
        let mut processor = Processor::new();
        test_op_with(opcode, &mut processor);
        processor
    }

    #[test]
    fn test_jump() {
        let p = test_op(0x1300);
        assert_eq!(p.pc, 0x300);
    }

    #[test]
    fn test_call() {
        let p = test_op(0x2300);
        assert_eq!(p.sp, 1);
        assert_eq!(p.pc, 0x300);
        // return address should be original address + 2, so
        // call instruction isn't executed again
        assert_eq!(p.stack[p.sp - 1], STARTING_PC + 2);
    }

    #[test]
    fn test_return() {
        let mut p = test_op(0x2300);
        test_op_with(0x00EE, &mut p);
        assert_eq!(p.sp, 0);
        assert_eq!(p.pc, STARTING_PC + 2);
    }

    /// test the 0x3___ instruction when register and compared value are equal
    #[test]
    fn test_compare_skip_equal() {
        let mut p = test_op(0x6412);
        test_op_with(0x3412, &mut p);
        assert_eq!(p.pc, STARTING_PC + 6);
    }

    /// test the 0x3___ instruction when register and compared value are not equal
    #[test]
    fn test_compare_skip_not_equal() {
        let mut p = test_op(0x6416);
        test_op_with(0x3412, &mut p);
        assert_eq!(p.pc, STARTING_PC + 4);
    }

    /// test the 0x4___ instruction when register and compared value are equal
    #[test]
    fn test_compare_dont_skip_equal() {
        let mut p = test_op(0x6412);
        test_op_with(0x4412, &mut p);
        assert_eq!(p.pc, STARTING_PC + 4);
    }

    /// test the 0x4___ instruction when register and compared value are not equal
    #[test]
    fn test_compare_dont_skip_not_equal() {
        let mut p = test_op(0x6416);
        test_op_with(0x4412, &mut p);
        assert_eq!(p.pc, STARTING_PC + 6);
    }

    /// test the 0x5___ instruction when both compared registers are equal
    #[test]
    fn test_compare_registers_skip_equal() {
        let mut p = test_op(0x6A16);
        test_op_with(0x6B16, &mut p);
        test_op_with(0x5AB0, &mut p);
        assert_eq!(p.pc, STARTING_PC + 8);
    }

    /// test the 0x5___ instruction when both compared registers are not equal
    #[test]
    fn test_compare_registers_skip_not_equal() {
        let mut p = test_op(0x6A16);
        test_op_with(0x6B12, &mut p);
        test_op_with(0x5AB0, &mut p);
        assert_eq!(p.pc, STARTING_PC + 6);
    }

    #[test]
    fn test_load_immediate() {
        let p = test_op(0x6112);
        assert_eq!(p.v[1], 0x12);
    }

    #[test]
    fn test_add() {
        let mut p = test_op(0x6112);
        test_op_with(0x7103, &mut p);
        assert_eq!(p.v[1], 0x15);
    }

    #[test]
    fn test_load_register() {
        let mut p = test_op(0x6B12);
        test_op_with(0x8AB0, &mut p);
        assert_eq!(p.v[0xB], 0x12);
    }

    #[test]
    fn test_or() {
        let mut p = test_op(0x6AFF);
        test_op_with(0x6B00, &mut p);
        test_op_with(0x8AB1, &mut p);
        assert_eq!(p.v[0xA], 0xFF);
    }

    #[test]
    fn test_and() {
        let mut p = test_op(0x6AFF);
        test_op_with(0x6B00, &mut p);
        test_op_with(0x8AB2, &mut p);
        assert_eq!(p.v[0xA], 0x00);
    }

    #[test]
    fn test_xor() {
        let mut p = test_op(0x6A10);
        test_op_with(0x6B11, &mut p);
        test_op_with(0x8AB3, &mut p);
        assert_eq!(p.v[0xA], 0x1);
    }

    #[test]
    fn test_carry_add() {
        let mut p = test_op(0x6AFF);
        test_op_with(0x6B04, &mut p);
        test_op_with(0x8AB4, &mut p);
        assert_eq!(p.v[0xA], 0x03);
        assert_eq!(p.v[0xF], 1);
    }

    #[test]
    fn test_carry_add_no_carry() {
        let mut p = test_op(0x6AF1);
        test_op_with(0x6B04, &mut p);
        test_op_with(0x8AB4, &mut p);
        assert_eq!(p.v[0xA], 0xF5);
        assert_eq!(p.v[0xF], 0);
    }

    /// Test the 8xy5 instruction with carry
    #[test]
    fn test_carry_sub() {
        let mut p = test_op(0x6A00);
        test_op_with(0x6B03, &mut p);
        test_op_with(0x8AB5, &mut p);
        assert_eq!(p.v[0xA], 0xFD);
        assert_eq!(p.v[0xF], 0);
    }

    /// Test the 8xy5 instruction without carry.
    #[test]
    fn test_carry_sub_no_carry() {
        let mut p = test_op(0x6AFF);
        test_op_with(0x6B03, &mut p);
        test_op_with(0x8AB5, &mut p);
        assert_eq!(p.v[0xA], 0xFC);
        assert_eq!(p.v[0xF], 1);
    }

    /// Test the 8xy6 instruction with carry.
    #[test]
    fn test_shift_right_carry() {
        let mut p = test_op(0x6A01);
        test_op_with(0x8AB6, &mut p);
        assert_eq!(p.v[0xA], 0x00);
        assert_eq!(p.v[0xF], 1);
    }

    /// Test the 8xy6 instruction without carry.
    #[test]
    fn test_shift_right_no_carry() {
        let mut p = test_op(0x6A02);
        test_op_with(0x8AB6, &mut p);
        assert_eq!(p.v[0xA], 0x01);
        assert_eq!(p.v[0xF], 0);
    }

    /// Test the 8xy7 instruction with carry.
    #[test]
    fn test_carry_sub_opposite() {
        let mut p = test_op(0x6A03);
        test_op_with(0x6B00, &mut p);
        test_op_with(0x8AB7, &mut p);
        assert_eq!(p.v[0xA], 0xFD);
        assert_eq!(p.v[0xF], 0);
    }

    /// Test the 8xy7 instruction without carry.
    #[test]
    fn test_carry_sub_opposite_no_carry() {
        let mut p = test_op(0x6A03);
        test_op_with(0x6B05, &mut p);
        test_op_with(0x8AB7, &mut p);
        assert_eq!(p.v[0xA], 0x02);
        assert_eq!(p.v[0xF], 1);
    }

    #[test]
    fn test_shift_left_carry() {
        let mut p = test_op(0x6AFF);
        test_op_with(0x8AEE, &mut p);
        assert_eq!(p.v[0xA], 0xFE);
        assert_eq!(p.v[0xF], 1);
    }

    #[test]
    fn test_shift_left_no_carry() {
        let mut p = test_op(0x6A01);
        test_op_with(0x8AEE, &mut p);
        assert_eq!(p.v[0xA], 0x02);
        assert_eq!(p.v[0xF], 0);
    }

    /// Test the 9xy0 instruction when the registers are not equal.
    #[test]
    fn test_skip_instr_opposite_not_equal() {
        let mut p = test_op(0x6A12);
        test_op_with(0x6B16, &mut p);
        test_op_with(0x9AB0, &mut p);
        assert_eq!(p.pc, STARTING_PC + 8);
    }

    /// Test the 9xy0 instruction when the registers are equal.
    #[test]
    fn test_skip_instr_opposite_equal() {
        let mut p = test_op(0x6A12);
        test_op_with(0x6B12, &mut p);
        test_op_with(0x9AB0, &mut p);
        assert_eq!(p.pc, STARTING_PC + 6);
    }

    #[test]
    fn test_load_index_register() {
        let p = test_op(0xA300);
        assert_eq!(p.i, 0x300);
    }

    #[test]
    fn test_jump_to_index_register_plus_offset() {
        let mut p = test_op(0x6012);
        test_op_with(0xB300, &mut p);
        assert_eq!(p.pc, 0x312);
    }

    #[test]
    fn test_get_random() {
        // we just test that the get random instruction doesn't panic
        let p = test_op(0xC000);
        assert_eq!(p.v[0], 0);
    }

    #[test]
    fn test_load_delay_timer() {
        let mut p = Processor::new();
        let mut bus = Bus::default();
        bus.clock.delay_timer = 30;
        p.process_opcode(0xFA07, &mut bus);
        assert_eq!(p.v[0xA], 30);
    }

    #[test]
    fn test_set_delay_timer() {
        let mut p = Processor::new();
        let mut bus = Bus::default();
        p.process_opcode(0x6A12, &mut bus);
        p.process_opcode(0xFA15, &mut bus);
        assert_eq!(bus.clock.delay_timer, 0x12);
    }

    #[test]
    fn test_set_sound_timer() {
        let mut p = Processor::new();
        let mut bus = Bus::default();
        p.process_opcode(0x6A12, &mut bus);
        p.process_opcode(0xFA18, &mut bus);
        assert_eq!(
            bus.clock
                .sound_timer
                .load(std::sync::atomic::Ordering::SeqCst),
            0x12
        );
    }

    #[test]
    fn test_add_to_index_register() {
        let mut p = test_op(0x6A12);
        test_op_with(0xA300, &mut p);
        test_op_with(0xFA1E, &mut p);
        assert_eq!(p.i, 0x312);
    }

    #[test]
    fn test_load_font_address() {
        let mut p = test_op(0x6004);
        test_op_with(0xF029, &mut p);
        // assuming that font data starts at the very
        // beginning of memory
        assert_eq!(p.i, 4 * 5);
    }

    #[test]
    fn test_store_bcd() {
        let mut p = Processor::new();
        let mut bus = Bus::default();
        p.process_opcode(0xA300, &mut bus);
        p.process_opcode(0x6AFD, &mut bus);
        p.process_opcode(0xFA33, &mut bus);
        assert_eq!(bus.memory[p.i], 2);
        assert_eq!(bus.memory[p.i + 1], 5);
        assert_eq!(bus.memory[p.i + 2], 3);
    }

    #[test]
    fn test_store_registers() {
        let mut processor = Processor::new();
        let mut bus = Bus::default();

        for i in 0x0..=0x6 {
            processor.v[usize::from(i)] = i;
        }

        processor.process_opcode(0x6A06, &mut bus);
        processor.process_opcode(0xA300, &mut bus);
        processor.process_opcode(0xFA55, &mut bus);

        for i in 0x0..0x7 {
            assert_eq!(bus.memory[0x300 + usize::from(i)], i);
        }
    }

    #[test]
    fn test_load_registers() {
        let mut processor = Processor::new();
        let mut bus = Bus::default();

        for i in 0x0..=0x6 {
            bus.memory[0x300 + usize::from(i)] = i;
        }

        processor.process_opcode(0x6A06, &mut bus);
        processor.process_opcode(0xA300, &mut bus);
        processor.process_opcode(0xFA65, &mut bus);

        for i in 0x0..0x7 {
            assert_eq!(processor.v[usize::from(i)], i);
        }
    }
}
