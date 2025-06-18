// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use crate::errno::__errno_location;
    use ::sys::error::ErrorCode;

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn mmap(
        _addr: *mut u8,
        _length: usize,
        _prot: i32,
        _flags: i32,
        _fd: i32,
        _offset: isize,
    ) -> *mut u8 {
        ::syslog::error!("mmap(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        core::ptr::null_mut()
    }

    /// Dummy implementation of `munmap`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn munmap(_addr: *mut u8, _length: usize) -> isize {
        ::syslog::error!("munmap(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}
