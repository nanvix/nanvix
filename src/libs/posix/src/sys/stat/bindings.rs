// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

///
/// # Safety
///
/// This function has undefined behavior if buf points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn fstat(fd: i32, buf: *mut crate::sys::stat::stat) -> i32 {
    ::nvx::log!("fstat(): fd = {}, buf = {:?}", fd, buf);
    crate::sys::stat::fstat(fd, &mut *buf)
}
