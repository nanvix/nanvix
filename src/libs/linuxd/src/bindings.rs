// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[no_mangle]
pub extern "C" fn kill(_pid: i32, _signal: i32) -> i32 {
    ::nvx::log!("kill(): pid = {}, signal = {}", _pid, _signal);
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
    ::nvx::log!("fstat(): fd = {}, buf = {:?}", fd, buf);
    crate::sys::stat::fstat(fd, &mut *buf)
}

#[no_mangle]
pub extern "C" fn close(fd: i32) -> i32 {
    ::nvx::log!("close(): fd = {}", fd);
    crate::unistd::close(fd)
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
