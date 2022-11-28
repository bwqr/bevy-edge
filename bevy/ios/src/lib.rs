use std::ffi::c_int;

#[no_mangle]
pub extern fn input_init() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    input::init("192.168.1.37:4001".to_string());

    log::info!("bevyedge is built with {} profile", if cfg!(debug_assertions) { "debug" } else { "release" });
    log::info!("bevyedge runtime is initialized");
}

#[no_mangle]
pub extern fn input_press(id: c_int) {
    log::info!("{id} is pressed");

    input::press(input::key_code_from_i32(id));
}

#[no_mangle]
pub extern fn input_release(id: c_int) {
    log::info!("{id} is released");

    input::release(input::key_code_from_i32(id));
}
