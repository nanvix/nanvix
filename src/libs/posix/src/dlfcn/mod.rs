// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::string::String;
use ::core::{
    ffi,
    ffi::{
        c_int,
        CStr,
    },
    mem,
    ptr,
};
use ::num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};
use ::spin::Mutex;
use ::sys::mm::{
    Address,
    VirtualAddress,
};
use ::sysapi::ffi::{
    c_char,
    c_void,
};
use ::syscall::dlfcn::{
    self,
    DlHandle,
    DlInfo,
};
use ::syslog::trace_libcall;

//==================================================================================================
// DlError
//==================================================================================================

struct DlError {
    /// Error message encoded in a C-compatible string.
    msg: [i8; Self::MAX_ERROR_MSG_LEN],
    /// Indicates whether an error has occurred.
    set: bool,
}

impl DlError {
    /// Maximum length of an error message (including the terminating null byte).
    const MAX_ERROR_MSG_LEN: usize = 256;

    /// Creates a new `DlError` instance.
    const fn new() -> Self {
        DlError {
            msg: [0; Self::MAX_ERROR_MSG_LEN],
            set: false,
        }
    }

    /// Sets the error message.
    fn set(&mut self, msg: &str) {
        let bytes: &[u8] = msg.as_bytes();
        let len: usize = bytes.len().min(Self::MAX_ERROR_MSG_LEN - 1);
        self.msg[..len].copy_from_slice(unsafe { mem::transmute::<&[u8], &[i8]>(&bytes[..len]) });
        self.msg[len] = b'\0' as i8;
        self.set = true;
    }

    /// Takes the error message if any.
    fn take(&mut self) -> Option<&CStr> {
        if self.set {
            self.set = false;
            let c_str: &CStr = unsafe { ffi::CStr::from_ptr(self.msg.as_ptr()) };
            Some(c_str)
        } else {
            None
        }
    }
}

//==================================================================================================
// DlOpenMode
//==================================================================================================

///
/// # Description
///
/// A type that represents the mode in which a dynamic library may be opened.
///
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
pub enum DlOpenMode {
    /// Relocations are performed at an implementation-defined time.
    Local = 0,
    /// Relocations are performed when the object is loaded.
    Lazy = 1,
    /// All symbols are available for relocation processing of other modules.
    Now = 2,
    /// All symbols are not made available for relocation processing by other modules.
    Global = 4,
}

//==================================================================================================
// DlLastError
//==================================================================================================

/// Holds the last error message not yet retrieved by a call to [`dlerror()`].
static DL_LAST_ERROR: Mutex<DlError> = Mutex::new(DlError::new());

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets information relating to an address.
///
/// # Parameters
///
/// - `addr`: The address to be looked up.
///
/// # Return Value
///
///  On success, zero is returned. On failure, -1 is returned. More detailed diagnostic
/// information is available through [`dlerror()`].
///
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may operate on global variables.
///
/// It is safe to use this function if the caller ensures that:
/// - `addr` points to a valid address.
/// - No other thread modifies the error state while this function is being executed.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn dladdr(addr: *const c_void, dlip: *mut DlInfo) -> i32 {
    // Check if `addr` is not valid.
    if addr.is_null() {
        let reason: &str = "addr is null";
        DL_LAST_ERROR.lock().set(reason);
        ::syslog::error!("dladdr(): {}", reason);
        return -1;
    }

    // Check if `info`` is not valid.
    if dlip.is_null() {
        let reason: &str = "info is null";
        DL_LAST_ERROR.lock().set(reason);
        ::syslog::error!("dladdr(): {}", reason);
        return -1;
    }

    // Convert `addr` to a `VirtualAddress`.
    let addr: VirtualAddress = VirtualAddress::from_raw_value(addr as usize);

    // Attempt to get the symbol information.
    match dlfcn::dladdr(addr, &mut *dlip) {
        Ok(()) => 0,
        Err(error) => {
            DL_LAST_ERROR.lock().set(error.reason);
            ::syslog::error!("dladdr(): {:?}", error);
            -1
        },
    }
}

///
/// # Description
///
/// Closes a dynamic library handle.
///
/// # Parameters
///
/// - `handle`: A pointer to the dynamic library handle to be closed.
///
/// # Return Value
///
/// On success, zero is returned. On failure, -1 is returned. More detailed diagnostic
/// information is available through [`dlerror()`].
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may operate on global variables.
///
/// It is safe to use this function if the caller ensures that:
/// - `handle` points to a valid dynamic library handle.
/// - No other thread modifies the error state while this function is being executed.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn dlclose(handle: *mut c_void) -> i32 {
    // Check if handle is not valid.
    if handle.is_null() {
        let reason: &str = "handle is null";
        DL_LAST_ERROR.lock().set(reason);
        ::syslog::error!("dlclose(): {}", reason);
        return -1;
    }

    // The global scope handle (from dlopen(NULL)) is not closeable.
    if DlHandle::from_mut_ptr(handle) == DlHandle::GLOBAL {
        return 0;
    }

    // Attempt to close the dynamic library handle.
    match dlfcn::dlclose(&DlHandle::from_mut_ptr(handle)) {
        Ok(()) => 0,
        Err(error) => {
            DL_LAST_ERROR.lock().set(error.reason);
            ::syslog::error!("dlclose(): {:?}", error);
            -1
        },
    }
}

///
/// # Description
///
/// Returns a string describing the last error that occurred during a dynamic linking operation.
///
/// # Return Value
///
/// A pointer to a null-terminated string containing the error message is returned. If no error
/// has occurred, a null pointer is returned instead.
///
/// # Safety
///
/// This function is unsafe because:
/// - It may operate on global variables.
///
/// It is safe to use this function if the caller ensures that:
/// - No other thread modifies the error state while this function is being executed.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn dlerror() -> *mut c_char {
    // Get the last error message.
    match DL_LAST_ERROR.lock().take() {
        Some(c_str) => c_str.as_ptr() as *mut c_char,
        None => ptr::null_mut(),
    }
}

///
/// # Description
///
/// Opens a symbol table handle.
///
/// # Parameters
///
/// - `filename`: The name of the shared object file to be opened, or NULL to obtain
///   a handle to the global symbol scope (main executable + loaded libraries).
/// - `mode`: The mode in which the object is opened.
///
/// # Return Value
///
/// On success, a handle to the opened object is returned. On failure, a null pointer is returned.
/// More detailed diagnostic information is available through [`dlerror()`].
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may operate on global variables.
///
/// It is safe to use this function if the caller ensures that:
/// - `filename` points to a valid C-style string, or is NULL.
/// - No other thread modifies the error state while this function is being executed.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn dlopen(filename: *const c_char, mode: c_int) -> *mut c_void {
    // Per POSIX: dlopen(NULL, mode) returns a handle to the global symbol scope.
    if filename.is_null() {
        // Populate the global symbol table from the executable's .dynsym section.
        dlfcn::dlinit();
        return DlHandle::GLOBAL.as_mut_ptr();
    }

    // Attempt to convert `filename` to a Rust string.
    let filename: &str = match ffi::CStr::from_ptr(filename).to_str() {
        Ok(pathname) => pathname,
        Err(error) => {
            let reason: String = alloc::format!("invalid filename (error={error:?})");
            ::syslog::error!("dlopen(): {}", reason);
            DL_LAST_ERROR.lock().set(&reason);
            return ptr::null_mut();
        },
    };

    // Attempt to convert `mode` to `DlOpenMode`.
    let mode: DlOpenMode = match DlOpenMode::try_from(mode) {
        Ok(mode) => mode,
        Err(error) => {
            let reason: String = alloc::format!("invalid mode (error={error:?})");
            DL_LAST_ERROR.lock().set(&reason);
            ::syslog::error!("dlopen(): {}", reason);
            return ptr::null_mut();
        },
    };

    // Check if open mode is not supported.
    if mode == DlOpenMode::Local {
        let reason: &str = "local mode is not supported";
        DL_LAST_ERROR.lock().set(reason);
        ::syslog::error!("dlopen(): {}", reason);
        return ptr::null_mut();
    }

    // Attempt to open the shared object file.
    match dlfcn::dlopen(filename) {
        Ok(handle) => handle.as_mut_ptr(),
        Err(error) => {
            DL_LAST_ERROR.lock().set(error.reason);
            ::syslog::error!("dlopen(): {:?}", error);
            ptr::null_mut()
        },
    }
}

///
/// # Description
///
/// Resolves a symbol in a shared object or executable.
///
/// # Parameters
///
/// - `handle`: A handle to the shared object or executable, or NULL/`RTLD_DEFAULT`
///   to search the global symbol scope (main executable + loaded libraries).
/// - `symbol`: The name of the symbol to be resolved.
///
/// # Return Value
///
/// On success, a pointer to the symbol is returned. On failure, a null pointer is returned. More
/// detailed diagnostic information is available through [`dlerror()`].
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may operate on global variables.
///
/// It is safe to use this function if the caller ensures that:
/// - `handle` points to a valid dynamic library handle, or is NULL (`RTLD_DEFAULT`).
/// - `symbol` points to a valid C-style string.
/// - No other thread modifies the error state while this function is being executed.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void {
    // Per POSIX: NULL handle (RTLD_DEFAULT) searches the global symbol scope.
    // The GLOBAL sentinel (from dlopen(NULL)) is also routed to the same path.
    let use_global: bool = handle.is_null() || DlHandle::from_mut_ptr(handle) == DlHandle::GLOBAL;

    // Check if symbol is not valid.
    if symbol.is_null() {
        let reason: &str = "symbol is null";
        DL_LAST_ERROR.lock().set(reason);
        ::syslog::error!("dlsym(): {}", reason);
        return ptr::null_mut();
    }

    // Attempt to convert `symbol` to a Rust string.
    let symbol: &str = match ffi::CStr::from_ptr(symbol).to_str() {
        Ok(symbol) => symbol,
        Err(error) => {
            let reason: String = alloc::format!("invalid symbol (error={error:?})");
            ::syslog::error!("dlsym(): {}", reason);
            DL_LAST_ERROR.lock().set(&reason);
            return ptr::null_mut();
        },
    };

    // Attempt to resolve the symbol.
    // For global scope, use the GLOBAL sentinel which dlsym() in the syscall
    // layer routes to global_symbol_lookup(). For regular handles, use as-is.
    let effective_handle: DlHandle = if use_global {
        DlHandle::GLOBAL
    } else {
        DlHandle::from_mut_ptr(handle)
    };
    match dlfcn::dlsym(&effective_handle, symbol) {
        Ok(symbol) => symbol.into_raw_value() as *mut c_void,
        Err(error) => {
            DL_LAST_ERROR.lock().set(error.reason);
            ::syslog::error!("dlsym(): {:?}", error);
            ptr::null_mut()
        },
    }
}
