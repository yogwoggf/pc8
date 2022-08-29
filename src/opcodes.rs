#[derive(Debug)]
pub struct DecodedOpcode {
    pub opcode: u16,
    pub instr_type: u8,
    pub X: u8,
    pub Y: u8,
    pub N: u8,
    pub NN: u8,
    pub NNN: u16,
}

impl DecodedOpcode {
    pub fn from(opcode: u16) -> Self {
        Self {
            opcode: opcode,
            instr_type: ((opcode >> 12) & 0xF) as u8,
            X: ((opcode >> 8) & 0xF) as u8, // Take the 2nd nibble out
            Y: ((opcode >> 4) & 0xF) as u8, // Take the 3rd nibble out
            N: (opcode & 0xF) as u8,        // Take the last nibble out
            NN: (opcode & 0xFF) as u8,      // Take the last byte out
            NNN: (opcode & 0xFFF), // Take the entire contents out except for the 1st nibble.
        }
    }
}
