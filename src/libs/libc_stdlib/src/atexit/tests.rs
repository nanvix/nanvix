// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
        unsafe { __cxa_atexit(Some(cxa_handler), (&raw const FIRST).cast_mut().cast(), first_dso) },
        0
    );
    assert_eq!(
        unsafe {
            __cxa_atexit(Some(cxa_handler), (&raw const SECOND).cast_mut().cast(), second_dso)
        },
        0
    );
    assert_eq!(
        unsafe { __cxa_atexit(Some(cxa_handler), (&raw const THIRD).cast_mut().cast(), first_dso) },
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
