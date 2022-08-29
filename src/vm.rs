use crate::keypad;
use crate::opcodes::DecodedOpcode;
use bitvec::prelude::*;
use rand::prelude::*;

pub struct Graphics {
    data: [u8; 64 * 32],
}

impl Graphics {
    pub fn new() -> Self {
        Self { data: [0; 64 * 32] }
    }

    pub fn clear(&mut self) {
        self.data = [0; 64 * 32];
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> u8 {
        self.data[y * 64 + x]
    }

    pub fn flip_pixel(&mut self, x: usize, y: usize, value: u8) -> bool {
        self.data[y * 64 + x] ^= value;
        self.data[y * 64 + x] == 0 // Return if the XOR made this pixel turn off.
    }
}

pub struct Memory {
    data: [u8; 4096],
}

impl Memory {
    pub fn new() -> Self {
        Self { data: [0; 4096] }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.data[addr as usize] = value;
    }

    pub fn write_array(&mut self, addr: u16, values: &[u8]) {
        for (i, byte) in values.iter().enumerate() {
            self.data[(addr as usize) + i] = *byte;
        }
    }
}

macro_rules! write_fonts {
    ($self:ident, $idx:literal, $data:expr) => {
        $self.memory.write_array(0x50u16 + ($idx * 5), &$data)
    };
}

pub struct VM<'a> {
    memory: Memory,
    registers: [u8; 16],

    pub I: u16,
    pub PC: u16,
    pub SP: u16,

    stack: Vec<u16>,

    pub speed: i32,
    paused: bool,
    block_for_key: Option<u8>, // If theres a u8, that means a register is waiting on any key

    delay_timer: u8,
    sound_timer: u8,

    beep_function: Option<&'a dyn Fn()>,

    pub graphics: Graphics,
}

impl<'a> VM<'a> {
    pub fn new() -> VM<'a> {
        VM {
            paused: false,
            block_for_key: None,

            delay_timer: 0,
            sound_timer: 0,

            I: 0,
            PC: 0x200,
            SP: 0,

            stack: Vec::with_capacity(32),
            memory: Memory::new(),
            registers: [0; 16],
            graphics: Graphics::new(),

            beep_function: None,

            speed: 10,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8], beep_fn: &'a dyn Fn()) {
        for (idx, byte) in rom.iter().enumerate() {
            self.memory.write(0x200u16 + idx as u16, *byte);
        }

        self.PC = 0x200;
        self.beep_function = Some(beep_fn);

        // We also need to load fonts
        write_fonts!(self, 0, [0xF0, 0x90, 0x90, 0x90, 0xF0]); // 0
        write_fonts!(self, 1, [0x20, 0x60, 0x20, 0x20, 0x70]); // 1
        write_fonts!(self, 2, [0xF0, 0x10, 0xF0, 0x80, 0xF0]); // 2
        write_fonts!(self, 3, [0xF0, 0x10, 0xF0, 0x10, 0xF0]); // 3
        write_fonts!(self, 4, [0x90, 0x90, 0xF0, 0x10, 0x10]); // 4
        write_fonts!(self, 5, [0xF0, 0x80, 0xF0, 0x10, 0xF0]); // 5
        write_fonts!(self, 6, [0xF0, 0x80, 0xF0, 0x90, 0xF0]); // 6
        write_fonts!(self, 7, [0xF0, 0x10, 0x20, 0x40, 0x40]); // 7
        write_fonts!(self, 8, [0xF0, 0x90, 0xF0, 0x90, 0xF0]); // 8
        write_fonts!(self, 9, [0xF0, 0x90, 0xF0, 0x10, 0xF0]); // 9

        write_fonts!(self, 10, [0xF0, 0x90, 0xF0, 0x90, 0x90]); // A
        write_fonts!(self, 11, [0xE0, 0x90, 0xE0, 0x90, 0xE0]); // B
        write_fonts!(self, 12, [0xF0, 0x80, 0x80, 0x80, 0xF0]); // C
        write_fonts!(self, 13, [0xE0, 0x90, 0x90, 0x90, 0xE0]); // D
        write_fonts!(self, 14, [0xF0, 0x80, 0xF0, 0x80, 0xF0]); // E
        write_fonts!(self, 15, [0xF0, 0x80, 0xF0, 0x80, 0x80]); // F
    }

    pub fn reset(&mut self) {
        // Clear display
        self.graphics.clear();
        self.PC = 0x200;
    }

    pub fn execute(&mut self, opcode: u16) {
        // Execute.

        let decoded = DecodedOpcode::from(opcode);
        match decoded.instr_type {
            0xA => {
                // ANNN
                self.I = decoded.NNN;
            }
            0xC => {
                // CXNN

                self.registers[decoded.X as usize] = thread_rng().gen_range(0..=255) & decoded.NN;
            }
            0x3 => {
                // 3XNN
                if self.registers[decoded.X as usize] == decoded.NN {
                    self.PC += 2; // Skip the next instruction
                }
            }
            0xD => {
                // DXYN
                let x_coord = self.registers[decoded.X as usize] & 63;
                let y_coord = self.registers[decoded.Y as usize] & 31; // Wrap the starting position but clip the sprite drawing
                let sprite_addr = self.I;

                let rows = decoded.N;

                for n in 0..rows {
                    let row_address = sprite_addr + n as u16;
                    let byte = self.memory.read(row_address);

                    let bits = byte.view_bits::<Msb0>();
                    for (i, bit) in bits.iter().by_vals().enumerate() {
                        let new_x = x_coord + i as u8;
                        let new_y = y_coord + n;

                        if new_x > 63 || new_y > 31 {
                            continue;
                        }

                        // Flip pixels
                        if self
                            .graphics
                            .flip_pixel(new_x.into(), new_y.into(), bit.into())
                        {
                            // If this check goes through, then that means we must set VF to 1 (usually means collision)
                            self.registers[0xF] = 1;
                        }
                    }
                }
            }
            0x7 => {
                // 7XNN
                self.registers[decoded.X as usize] =
                    self.registers[decoded.X as usize].wrapping_add(decoded.NN);
            }
            0x6 => {
                // 6XNN
                self.registers[decoded.X as usize] = decoded.NN;
            }
            0x1 => {
                // 1NNN jmp
                self.PC = decoded.NNN - 2 // Subtract by 2 because PC += 2 will be called after this.
            }
            0x2 => {
                // 2NNN
                // Calls, which means that the current PC (but we actually save the IP which is the next instruction) is saved and when RET is called, we come back to this place

                self.stack.push(self.PC + 2);
                self.PC = decoded.NNN - 2
            }
            0x9 => {
                // 9XY0
                if self.registers[decoded.X as usize] != self.registers[decoded.Y as usize] {
                    self.PC += 2;
                }
            }
            0xE => match decoded.NN {
                0x9E => {
                    // EX9E
                    if keypad::is_key_down(self.registers[decoded.X as usize]) {
                        self.PC += 2
                    }
                }

                0xA1 => {
                    // EXA1
                    if !keypad::is_key_down(self.registers[decoded.X as usize]) {
                        self.PC += 2
                    }
                }

                _ => unimplemented!(
                    "Unimplemented opcode. Hex: {:#x}, {:#?}",
                    decoded.opcode,
                    decoded
                ),
            },
            // 0x00**
            0x0 => match decoded.NN {
                0xEE => {
                    // 00EE (RET)
                    let last_pc = self.stack.pop();
                    if last_pc.is_none() {
                        panic!("Stack underflow! (RET but no CALL)");
                    }

                    let last_pc = last_pc.unwrap();
                    self.PC = last_pc - 2 // - 2 because it'll be += 2 at the end of this.
                }

                0xE0 => {
                    // 00E0 (disp_clear)
                    self.graphics.clear();
                }

                _ => unimplemented!(
                    "Unimplemented opcode. Hex: {:#x}, {:#?}",
                    decoded.opcode,
                    decoded
                ),
            },
            0x8 => match decoded.N {
                0x0 => {
                    // 8XY0
                    self.registers[decoded.X as usize] = self.registers[decoded.Y as usize];
                }
                0x1 => {
                    self.registers[decoded.X as usize] |= self.registers[decoded.Y as usize];
                }
                0x2 => {
                    // 8XY2
                    self.registers[decoded.X as usize] &= self.registers[decoded.Y as usize];
                }
                0x3 => {
                    // 8XY3
                    self.registers[decoded.X as usize] ^= self.registers[decoded.Y as usize];
                }
                0xE => {
                    // 8XYE
                    self.registers[0xF] = self.registers[decoded.X as usize] & 1;
                    self.registers[decoded.X as usize] <<= 1;
                }
                0x6 => {
                    // 8XY6
                    // TODO: store LSB in VF
                    self.registers[decoded.X as usize] >>= 1;
                }
                0x4 => {
                    // 8XY4
                    // TODO: add carry flag
                    self.registers[decoded.X as usize] = self.registers[decoded.X as usize]
                        .wrapping_add(self.registers[decoded.Y as usize]);
                }
                0x5 => {
                    // 8XY5
                    self.registers[decoded.X as usize] = self.registers[decoded.X as usize]
                        .wrapping_sub(self.registers[decoded.Y as usize]);
                }
                _ => unimplemented!(
                    "Unimplemented opcode. Hex: {:#x}, {:#?}",
                    decoded.opcode,
                    decoded
                ),
            },
            0x5 => {
                // 5XY0
                if (self.registers[decoded.X as usize] == self.registers[decoded.Y as usize]) {
                    self.PC += 2; // Skip next instruction
                }
            }
            0x4 => {
                // 4XNN
                if self.registers[decoded.X as usize] != decoded.NN {
                    self.PC += 2;
                }
            }
            // 0xF***
            0xF => match decoded.NN {
                0x1E => {
                    // FX1E
                    self.I += self.registers[decoded.X as usize] as u16;
                }

                0x55 => {
                    // FX55
                    // Same as FX65 but we dump our regs to the memory
                    let register_range = decoded.X; // Basically, reg_load fills up registers 0 to register X (x is included!)

                    for reg in 0..=register_range {
                        let address_to_write = self.I + reg as u16;
                        self.memory
                            .write(address_to_write, self.registers[reg as usize]);
                    }
                }

                0x29 => {
                    // FX29
                    // Get default font for Vx

                    let addr = 0x50u16 + (self.registers[decoded.X as usize] as u16 * 5);
                    self.I = addr;
                }

                0x07 => {
                    // FX07
                    self.registers[decoded.X as usize] = self.delay_timer;
                }

                0x0A => {
                    // FX0A

                    self.block_for_key = Some(decoded.X);
                    self.paused = true;
                }

                0x33 => {
                    // FX33 BCD
                    let val = self.registers[decoded.X as usize];
                    let hundreds = val / 100; // No need for any type of integer division
                    let tens = (val % 100) / 10;
                    let ones = val % 10;

                    self.memory.write(self.I, hundreds);
                    self.memory.write(self.I + 1, tens);
                    self.memory.write(self.I + 2, ones);
                }

                0x18 => {
                    // FX18
                    self.sound_timer = self.registers[decoded.X as usize];
                }

                0x15 => {
                    // FX15
                    self.delay_timer = self.registers[decoded.X as usize];
                }

                0x65 => {
                    // FX65
                    let register_range = decoded.X; // Basically, reg_load fills up registers 0 to register X (x is included!)

                    for reg in 0..=register_range {
                        let address_to_read = self.I + reg as u16;
                        let value = self.memory.read(address_to_read);

                        self.registers[reg as usize] = value;
                    }
                }

                _ => unimplemented!(
                    "Unimplemented opcode. Hex: {:#x}, {:#?}",
                    decoded.opcode,
                    decoded
                ),
            },
            _ => unimplemented!(
                "Unimplemented opcode. Hex: {:#x}, {:#?}",
                decoded.opcode,
                decoded
            ),
        };
    }

    pub fn cycle(&mut self) {
        for _ in 0..self.speed {
            if (!self.paused) {
                let opcode_1 = (self.memory.read(self.PC) as u16) << 8u16;
                let opcode_2 = self.memory.read(self.PC + 1) as u16;
                // Combine the 2 bytes into a word
                let opcode: u16 = opcode_1 | opcode_2;

                self.execute(opcode);
                self.PC += 2;
            } else {
                if let Some(reg) = self.block_for_key {
                    if let Some(nibble_key) = keypad::is_any_key_down() {
                        self.registers[reg as usize] = nibble_key;
                        self.paused = false;
                        self.block_for_key = None;
                    }
                }
            }

            if (self.delay_timer > 0) {
                self.delay_timer -= 1;
            }

            if (self.sound_timer > 0) {
                if (self.sound_timer == 1) {
                    if let Some(beep) = self.beep_function {
                        beep();
                    }
                }

                self.sound_timer -= 1;
            }
        }
    }
}
