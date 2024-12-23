// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[no_mangle]
pub extern "C" fn close(fd: i32) -> i32 {
    ::nvx::log!("close(): fd = {}", fd);
    crate::unistd::close(fd)
}

#[no_mangle]
pub extern "C" fn _exit(status: i32) -> ! {
    let Err(e) = nvx::sys::kcall::pm::exit(status);
    panic!("failed to terminate process (error={:?})", e);
}

#[no_mangle]
pub extern "C" fn getpid() -> i32 {
    match nvx::sys::kcall::pm::getpid() {
        Ok(pid) => pid.into(),
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn isatty(_fd: i32) -> i32 {
    // TODO: Implement this system call.
    0
}

#[no_mangle]
pub extern "C" fn lseek(fd: i32, offset: i64, whence: i32) -> i64 {
    ::nvx::log!("lseek(): fd = {}, offset = {}, whence = {}", fd, offset, whence);
    crate::unistd::lseek(fd, offset, whence)
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn read(fd: i32, buffer: *mut u8, count: u32) -> i32 {
    ::nvx::log!("read(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    crate::unistd::read(fd, buffer, count)
}

///
/// # Safety
///
/// The function has undefined behavior if the `buffer` points to an invalid memory location.
///
#[no_mangle]
pub unsafe extern "C" fn write(fd: i32, buffer: *const u8, count: u32) -> i32 {
    ::nvx::log!("write(): fd = {}, buffer = {:?}, count = {}", fd, buffer, count);
    crate::unistd::write(fd, buffer, count)
}
