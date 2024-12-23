// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

///
/// # Safety
///
/// The function has undefined behavior if the `path` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn open(path: *const i8, flags: i32, mode: u32) -> i32 {
    ::nvx::log!("open(): path = {:?}, flags = {}, mode = {}", path, flags, mode);
    crate::unistd::open(path, flags, mode)
}
