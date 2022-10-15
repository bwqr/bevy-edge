use std::collections::HashSet;

use bevy_input::prelude::KeyCode;
use console::{Term, Key};

fn main() {
    input::init("127.0.0.1:4001".to_string());

    let term = Term::stdout();
    let mut pressed_keys: HashSet<KeyCode> = HashSet::new();
   
    while let Ok(key) = term.read_key() {
        let key_code = key_code_from_key(key);

        if pressed_keys.remove(&key_code) {
            input::release(key_code);
        } else {
            input::press(key_code);
            pressed_keys.insert(key_code);
        }
    }
}

fn key_code_from_key(value: Key) -> KeyCode {
    match value {
        Key::Char('w') => KeyCode::W,
        Key::Char('d') => KeyCode::D,
        Key::Char('s') => KeyCode::S,
        Key::Char('a') => KeyCode::A,
        _ => panic!("unknown value is provided for key code"),
    }
}
