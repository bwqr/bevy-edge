use std::ffi::CString;

use jni::{
    objects::JClass,
    JNIEnv, sys::jint,
};
use libc::c_char;

mod log;

fn capture_stderr() {
    std::thread::spawn(|| unsafe {
        let mut pipes: [i32; 2] = [0; 2];
        libc::pipe(&mut pipes as *mut i32);
        libc::dup2(pipes[1], libc::STDERR_FILENO);

        let readonly = CString::new("r").unwrap();
        let file = libc::fdopen(pipes[0], readonly.as_ptr());

        let mut buff: [c_char; 256] = [0; 256];
        let tag = CString::new("stderr").unwrap();

        loop {
            libc::fgets(&mut buff as *mut c_char, 256, file);
            log::__android_log_write(5, tag.as_ptr(), buff.as_ptr());
        }
    });
}

#[no_mangle]
pub extern "C" fn Java_com_bwqr_bevyedge_InputKt__1init(
    _: JNIEnv,
    _: JClass,
) {
    capture_stderr();

    log::init();

    input::init("10.0.2.2:4001".to_string());

    ::log::info!("bevyedge is built with {} profile", if cfg!(debug_assertions) { "debug" } else { "release" });
    ::log::info!("bevyedge runtime is initialized");
}

#[no_mangle]
pub extern "C" fn Java_com_bwqr_bevyedge_InputKt__1press(
    _: JNIEnv,
    _: JClass,
    id: jint,
) {
    ::log::info!("{id} is pressed");

    input::press(input::key_code_from_i32(id));
}

#[no_mangle]
pub extern "C" fn Java_com_bwqr_bevyedge_InputKt__1release(
    _: JNIEnv,
    _: JClass,
    id: jint,
) {
    ::log::info!("{id} is released");
    input::release(input::key_code_from_i32(id));
}
