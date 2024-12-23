// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[no_mangle]
pub extern "C" fn kill(_pid: i32, _signal: i32) -> i32 {
    ::nvx::log!("kill(): pid = {}, signal = {}", _pid, _signal);
    // TODO: Implement this system call.
    -1
}

///
/// # Safety
///
/// The function has undefined behavior if the `path` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn getentropy(_buffer: *mut u8, _length: u32) -> i32 {
    ::nvx::log!("getentropy(): buffer = {:?}, length = {}", _buffer, _length);
    -1
}
