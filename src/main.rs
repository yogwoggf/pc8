use macroquad::{
    audio::{load_sound_from_bytes, play_sound_once},
    prelude::*,
    ui::root_ui,
};
use native_dialog::FileDialog;

mod keypad;
mod opcodes;
mod vm;

#[macroquad::main("PC8")]
async fn main() {
    let mut chip8 = vm::VM::new();

    let path = FileDialog::new()
        .set_location("C:/")
        .add_filter("Chip8 ROM", &["bin", "chip8", "ch8"])
        .show_open_single_file()
        .unwrap();

    if path.is_none() {
        println!("No path selected? Exiting.");
        return;
    }

    let path = path.unwrap();
    let bytes = std::fs::read(path);

    if let Err(err) = bytes {
        println!("Error while reading file: {}", err.to_string());
        return;
    }

    let beep_bytes = include_bytes!("assets/beep.wav");
    let beep_sound = load_sound_from_bytes(beep_bytes)
        .await
        .expect("Couldn't load the beep sound!");

    let bytes = bytes.unwrap();
    let play_beep = || {
        play_sound_once(beep_sound);
    };

    chip8.load_rom(bytes.as_slice(), &play_beep);

    loop {
        clear_background(WHITE);
        for y in 0..32 {
            for x in 0..64 {
                let pixel = chip8.graphics.get_pixel(x, y);
                let mut col = WHITE;

                if pixel == 0 {
                    col = BLACK;
                }

                let fl_x = x as f32 * 9.0;
                let fl_y = y as f32 * 9.0;

                draw_rectangle(fl_x, fl_y, 9.0, 9.0, col)
            }
        }

        draw_text(&format!("PC: {}", chip8.PC), 20.0, 360.0, 30.0, DARKGRAY);
        draw_text(&format!("I: {}", chip8.I), 20.0, 390.0, 30.0, DARKGRAY);
        draw_text(&format!("SP: {}", chip8.SP), 20.0, 410.0, 30.0, DARKGRAY);
        draw_text(
            &format!("Instructions per frame: {}", chip8.speed),
            20.0,
            430.0,
            30.0,
            DARKGRAY,
        );

        if root_ui().button(Vec2::new(20.0, 460.0), "Reset PC to 0x200") {
            chip8.reset();
        }

        if root_ui().button(Vec2::new(20.0, 490.0), "Test speaker") {
            play_sound_once(beep_sound);
        }

        if root_ui().button(Vec2::new(20.0, 520.0), "Increase program speed") {
            chip8.speed += 10;
        }

        if root_ui().button(Vec2::new(20.0, 550.0), "Decrease program speed") {
            chip8.speed -= 10;
        }

        chip8.cycle();
        next_frame().await;
    }
}
