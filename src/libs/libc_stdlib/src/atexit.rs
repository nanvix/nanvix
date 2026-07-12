// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::spin::Mutex;
use ::sysapi::ffi::{
    c_int,
    c_void,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of process-exit handlers that can be registered.
const MAX_ATEXIT_HANDLERS: usize = 32;

//==================================================================================================
// Structures
//==================================================================================================

/// Registered process-exit handler.
#[derive(Clone, Copy)]
enum AtExitHandler {
    /// Handler registered through `atexit()`.
    Plain(unsafe extern "C" fn()),
    /// Handler registered through `__cxa_atexit()`.
    Cxa {
        func: unsafe extern "C" fn(*mut c_void),
        arg: *mut c_void,
        dso_handle: *mut c_void,
    },
}

// SAFETY: C++ ABI arguments and DSO handles are opaque pointer values. The registry only stores
// them and passes each argument back to its handler; it never dereferences either pointer.
unsafe impl Send for AtExitHandler {}

impl AtExitHandler {
    /// Invokes this handler.
    unsafe fn call(self) {
        match self {
            Self::Plain(func) => unsafe { func() },
            Self::Cxa {
                func,
                arg,
                dso_handle: _,
            } => unsafe { func(arg) },
        }
    }
}

/// Registry of process-exit handlers in registration order.
struct AtExitRegistry {
    handlers: [Option<AtExitHandler>; MAX_ATEXIT_HANDLERS],
    count: usize,
}

impl AtExitRegistry {
    /// Creates an empty process-exit handler registry.
    const fn new() -> Self {
        Self {
            handlers: [None; MAX_ATEXIT_HANDLERS],
            count: 0,
        }
    }

    /// Appends `handler` to the registry.
    fn push(&mut self, handler: AtExitHandler) -> bool {
        if self.count >= MAX_ATEXIT_HANDLERS {
            return false;
        }

        self.handlers[self.count] = Some(handler);
        self.count += 1;
        true
    }

    /// Removes and returns the most recently registered handler.
    fn pop(&mut self) -> Option<AtExitHandler> {
        if self.count == 0 {
            return None;
        }

        self.count -= 1;
        self.handlers[self.count].take()
    }

    /// Removes and returns the most recently registered C++ ABI handler for `dso_handle`.
    fn pop_cxa(&mut self, dso_handle: *mut c_void) -> Option<AtExitHandler> {
        let mut index: usize = self.count;

        while index > 0 {
            index -= 1;

            let matches: bool = match self.handlers[index] {
                Some(AtExitHandler::Cxa {
                    dso_handle: registered_dso,
                    ..
                }) => dso_handle.is_null() || registered_dso == dso_handle,
                _ => false,
            };

            if matches {
                return self.remove(index);
            }
        }

        None
    }

    /// Removes and returns the handler at `index`, preserving registration order.
    fn remove(&mut self, index: usize) -> Option<AtExitHandler> {
        let handler: Option<AtExitHandler> = self.handlers[index];
        let mut current: usize = index;

        while current + 1 < self.count {
            self.handlers[current] = self.handlers[current + 1];
            current += 1;
        }

        self.count -= 1;
        self.handlers[self.count] = None;
        handler
    }
}

//==================================================================================================
// Global State
//==================================================================================================

/// Registry of process-exit handlers.
static ATEXIT_REGISTRY: Mutex<AtExitRegistry> = Mutex::new(AtExitRegistry::new());

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Registers a function to be called at normal process termination.
///
/// # Parameters
///
/// - `func`: Function to be called at exit.
///
/// # Returns
///
/// `0` on success, or `-1` if the handler table is full or `func` is `None`.
///
/// # Safety
///
/// The caller must ensure that `func` remains valid until it is called at process exit.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/atexit.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn atexit(func: Option<unsafe extern "C" fn()>) -> c_int {
    let func: unsafe extern "C" fn() = match func {
        Some(func) => func,
        None => return -1,
    };

    if ATEXIT_REGISTRY.lock().push(AtExitHandler::Plain(func)) {
        0
    } else {
        -1
    }
}

///
/// # Description
///
/// Registers a C++ ABI destructor to run at process exit or when its shared object is finalized.
///
/// # Parameters
///
/// - `func`: Destructor function to register.
/// - `arg`: Opaque argument passed to `func` when it is invoked.
/// - `dso_handle`: Shared-object handle associated with this registration.
///
/// # Returns
///
/// `0` on success, or `-1` if the handler table is full or `func` is `None`.
///
/// # Safety
///
/// The caller must ensure that `func`, `arg`, and `dso_handle` remain valid until the handler is
/// finalized.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __cxa_atexit(
    func: Option<unsafe extern "C" fn(*mut c_void)>,
    arg: *mut c_void,
    dso_handle: *mut c_void,
) -> c_int {
    let func: unsafe extern "C" fn(*mut c_void) = match func {
        Some(func) => func,
        None => return -1,
    };

    let handler: AtExitHandler = AtExitHandler::Cxa {
        func,
        arg,
        dso_handle,
    };

    if ATEXIT_REGISTRY.lock().push(handler) {
        0
    } else {
        -1
    }
}

///
/// # Description
///
/// Invokes registered C++ ABI destructors in reverse registration order. A null `dso_handle`
/// selects every C++ ABI destructor; otherwise, only destructors registered with the matching
/// handle are selected. Each registration is removed before its destructor is called.
///
/// # Parameters
///
/// - `dso_handle`: Shared-object handle to finalize, or null to finalize all shared objects.
///
/// # Safety
///
/// The registered destructors must still be valid and uphold their individual safety contracts.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __cxa_finalize(dso_handle: *mut c_void) {
    loop {
        let handler: Option<AtExitHandler> = ATEXIT_REGISTRY.lock().pop_cxa(dso_handle);

        match handler {
            Some(handler) => unsafe { handler.call() },
            None => break,
        }
    }
}

///
/// # Description
///
/// Invokes all registered process-exit handlers in reverse registration order. Each registration
/// is removed before its handler is called.
///
/// # Safety
///
/// The registered handlers must still be valid and uphold their individual safety contracts.
///
pub unsafe fn call_atexit_handlers() {
    loop {
        let handler: Option<AtExitHandler> = ATEXIT_REGISTRY.lock().pop();

        match handler {
            Some(handler) => unsafe { handler.call() },
            None => break,
        }
    }
}

/// Resets the process-exit handler registry. Used for test isolation.
#[cfg(test)]
fn reset_atexit_handlers() {
    *ATEXIT_REGISTRY.lock() = AtExitRegistry::new();
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        __cxa_atexit,
        __cxa_finalize,
        atexit,
        call_atexit_handlers,
        reset_atexit_handlers,
        MAX_ATEXIT_HANDLERS,
    };
    use ::core::sync::atomic::{
        AtomicUsize,
        Ordering,
    };
    use ::std::sync::{
        Mutex,
        MutexGuard,
    };
    use ::sysapi::ffi::c_void;

    static CALL_ORDER: AtomicUsize = AtomicUsize::new(0);
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    unsafe extern "C" fn plain_handler() {
        record_call(9);
    }

    unsafe extern "C" fn cxa_handler(arg: *mut c_void) {
        let value: usize = unsafe { *arg.cast::<usize>() };
        record_call(value);
    }

    fn record_call(value: usize) {
        let previous: usize = CALL_ORDER.load(Ordering::SeqCst);
        CALL_ORDER.store(previous * 10 + value, Ordering::SeqCst);
    }

    fn lock_tests() -> MutexGuard<'static, ()> {
        match TEST_LOCK.lock() {
            Ok(guard) => guard,
            Err(error) => error.into_inner(),
        }
    }

    #[test]
    fn register_handler() {
        let _guard: MutexGuard<'static, ()> = lock_tests();
        reset_atexit_handlers();
        assert_eq!(unsafe { atexit(Some(plain_handler)) }, 0);
        reset_atexit_handlers();
    }

    #[test]
    fn register_too_many() {
        let _guard: MutexGuard<'static, ()> = lock_tests();
        reset_atexit_handlers();
        for _ in 0..MAX_ATEXIT_HANDLERS {
            assert_eq!(unsafe { atexit(Some(plain_handler)) }, 0);
        }
        assert_eq!(unsafe { atexit(Some(plain_handler)) }, -1);
        reset_atexit_handlers();
    }

    #[test]
    fn finalize_cxa_handlers_by_dso() {
        static DSO_A: u8 = 0;
        static DSO_B: u8 = 0;
        static FIRST: usize = 1;
        static SECOND: usize = 2;
        static THIRD: usize = 3;

        let _guard: MutexGuard<'static, ()> = lock_tests();
        let first_dso: *mut c_void = (&raw const DSO_A).cast_mut().cast();
        let second_dso: *mut c_void = (&raw const DSO_B).cast_mut().cast();

        reset_atexit_handlers();
        CALL_ORDER.store(0, Ordering::SeqCst);

        assert_eq!(unsafe { atexit(Some(plain_handler)) }, 0);
        assert_eq!(
            unsafe {
                __cxa_atexit(Some(cxa_handler), (&raw const FIRST).cast_mut().cast(), first_dso)
            },
            0
        );
        assert_eq!(
            unsafe {
                __cxa_atexit(Some(cxa_handler), (&raw const SECOND).cast_mut().cast(), second_dso)
            },
            0
        );
        assert_eq!(
            unsafe {
                __cxa_atexit(Some(cxa_handler), (&raw const THIRD).cast_mut().cast(), first_dso)
            },
            0
        );

        unsafe { __cxa_finalize(first_dso) };
        assert_eq!(CALL_ORDER.load(Ordering::SeqCst), 31);

        unsafe { __cxa_finalize(first_dso) };
        assert_eq!(CALL_ORDER.load(Ordering::SeqCst), 31);

        unsafe { call_atexit_handlers() };
        assert_eq!(CALL_ORDER.load(Ordering::SeqCst), 3129);
        reset_atexit_handlers();
    }

    #[test]
    fn finalize_all_cxa_handlers() {
        static DSO_A: u8 = 0;
        static DSO_B: u8 = 0;
        static FIRST: usize = 1;
        static SECOND: usize = 2;

        let _guard: MutexGuard<'static, ()> = lock_tests();

        reset_atexit_handlers();
        CALL_ORDER.store(0, Ordering::SeqCst);

        assert_eq!(
            unsafe {
                __cxa_atexit(
                    Some(cxa_handler),
                    (&raw const FIRST).cast_mut().cast(),
                    (&raw const DSO_A).cast_mut().cast(),
                )
            },
            0
        );
        assert_eq!(unsafe { atexit(Some(plain_handler)) }, 0);
        assert_eq!(
            unsafe {
                __cxa_atexit(
                    Some(cxa_handler),
                    (&raw const SECOND).cast_mut().cast(),
                    (&raw const DSO_B).cast_mut().cast(),
                )
            },
            0
        );

        unsafe { __cxa_finalize(::core::ptr::null_mut()) };
        assert_eq!(CALL_ORDER.load(Ordering::SeqCst), 21);

        unsafe { call_atexit_handlers() };
        assert_eq!(CALL_ORDER.load(Ordering::SeqCst), 219);
        reset_atexit_handlers();
    }
}
