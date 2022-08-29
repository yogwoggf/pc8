//! Emulates the COSMAC VIP keypad with modern remappings.

use macroquad::prelude::*;
use phf::phf_map;
use std::collections::HashMap;

/*
COSMAC VIP
1 	2 	3 	C
4 	5 	6 	D
7 	8 	9 	E
A 	0 	B 	F
Modern Chip8
1 	2 	3 	4
Q 	W 	E 	R
A 	S 	D 	F
Z 	X 	C 	V
*/

static NIBBLE_TO_KEYCODE: phf::Map<u8, KeyCode> = phf_map! {
    0x0u8 => KeyCode::Key0,
    0x1u8 => KeyCode::Key1,
    0x2u8 => KeyCode::Key2,
    0x3u8 => KeyCode::Key3,
    0x4u8 => KeyCode::Key4,
    0x5u8 => KeyCode::Key5,
    0x6u8 => KeyCode::Key6,
    0x7u8 => KeyCode::Key7,
    0x8u8 => KeyCode::Key8,
    0x9u8 => KeyCode::Key9,

    0xAu8 => KeyCode::A,
    0xBu8 => KeyCode::B,
    0xCu8 => KeyCode::C,
    0xDu8 => KeyCode::D,
    0xEu8 => KeyCode::E,
    0xFu8 => KeyCode::F,
};

pub fn is_any_key_down() -> Option<u8> {
    for (nibble, keycode) in &NIBBLE_TO_KEYCODE {
        if macroquad::input::is_key_down(*keycode) {
            return Some(*nibble);
        }
    }

    None
}

pub fn is_key_down(nibble: u8) -> bool {
    let keycode = match NIBBLE_TO_KEYCODE.get(&nibble) {
        Some(code) => code,
        None => panic!("Keypad received an unknown nibble code? {:#x}", nibble),
    };

    macroquad::input::is_key_down(*keycode)
}
