// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
