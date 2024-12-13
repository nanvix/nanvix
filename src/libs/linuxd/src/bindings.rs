// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[no_mangle]
pub extern "C" fn kill(_pid: i32, _signal: i32) -> i32 {
    // TODO: Implement this system call.
    -1
}

#[no_mangle]
pub extern "C" fn isatty(_fd: i32) -> i32 {
    // TODO: Implement this system call.
    0
}

///
/// # Safety
///
/// This function has undefined behavior if buf points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn fstat(fd: i32, buf: *mut crate::sys::stat::stat) -> i32 {
    crate::sys::stat::fstat(fd, &mut *buf)
}

#[no_mangle]
pub extern "C" fn close(fd: i32) -> i32 {
    crate::unistd::close(fd)
}

#[no_mangle]
pub extern "C" fn lseek(fd: i32, offset: i64, whence: i32) -> i64 {
    crate::unistd::lseek(fd, offset, whence)
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn read(fd: i32, buffer: *mut u8, count: u32) -> i32 {
    crate::unistd::read(fd, buffer, count)
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn write(fd: i32, buffer: *const u8, count: u32) -> i32 {
    crate::unistd::write(fd, buffer, count)
}
