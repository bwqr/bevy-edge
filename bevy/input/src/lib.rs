use std::{collections::HashMap, sync::RwLock, io::Read};

use bevy_input::{prelude::KeyCode, ButtonState};
use once_cell::sync::OnceCell;

static KEY_EVENTS: OnceCell<RwLock<HashMap<KeyCode, ButtonState>>> = OnceCell::new();

pub fn init(address: String) {
    KEY_EVENTS.set(RwLock::new(HashMap::new())).unwrap();

    std::thread::spawn(move || send_key_events(address));
}

pub fn press(key_code: KeyCode) {
    let mut events = KEY_EVENTS.get().unwrap().write().unwrap();

    match events.get(&key_code) {
        Some(ButtonState::Released) => { events.remove(&key_code); },
        None => { events.insert(key_code, ButtonState::Pressed); },
        _ => {},
    };
}

pub fn release(key_code: KeyCode) {
    let mut events = KEY_EVENTS.get().unwrap().write().unwrap();

    match events.get(&key_code) {
        Some(ButtonState::Pressed) => { events.remove(&key_code); },
        None => { events.insert(key_code, ButtonState::Released); },
        _ => {},
    };

}

pub fn key_code_from_i32(value: i32) -> KeyCode {
    match value {
        0 => KeyCode::W,
        1 => KeyCode::D,
        2 => KeyCode::S,
        3 => KeyCode::A,
        _ => panic!("unknown value is provided for key code {value}"),
    }
}

fn send_key_events(address: String) {
    let mut stream = std::net::TcpStream::connect(address.as_str()).unwrap();

    loop {
        bincode::serialize_into(&mut stream, &collect_key_events()).unwrap();
        let mut byte = [0];
        while stream.read(&mut byte).unwrap() == 0 {
        }
    }
}

fn collect_key_events() -> Vec<(KeyCode, ButtonState)> {
    let events = std::mem::replace(&mut *KEY_EVENTS.get().unwrap().write().unwrap(), HashMap::new());
    events.into_iter().collect()
}
