// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Serializes the tests in this module. Every test exercises the same process-global
/// environment table (and `setenv` callback), so under the default multi-threaded test harness
/// one test's mutation — for example clearing the table — could race against another test's assertions
/// (e.g. wiping a key between a sibling test's write and its read). Holding this guard for the
/// whole body of each test forces them to run serially against the shared state. A spinlock is
/// used (rather than `std::sync::Mutex`) so that a panicking test cannot poison the lock and
/// turn a single failure into a cascade across the others.
static TEST_GUARD: Mutex<()> = Mutex::new(());

/// Acquires the shared [`TEST_GUARD`] and resets the global environment table, giving the
/// calling test exclusive access to a clean table. The returned guard must be held for the
/// duration of the test, so callers must bind it (e.g. `let _guard = setup();`); dropping it
/// immediately would release the lock and re-expose the race.
#[must_use]
fn setup() -> spin::MutexGuard<'static, ()> {
    let guard: spin::MutexGuard<'static, ()> = TEST_GUARD.lock();
    ENV_TABLE.lock().clear();
    guard
}

/// Tests that setting and getting a variable returns the correct value.
#[test]
fn test_set_and_get() {
    let _guard = setup();
    assert_eq!(set("TEST_KEY_1", b"hello", true), Ok(true));
    let ptr: *const c_char = get("TEST_KEY_1");
    assert!(!ptr.is_null());
    let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
    assert_eq!(value.to_str(), Ok("hello"));
}

/// Tests that getting a non-existent variable returns null.
#[test]
fn test_get_missing() {
    let _guard = setup();
    let ptr: *const c_char = get("TEST_NONEXISTENT");
    assert!(ptr.is_null());
}

/// Tests that `clear()` removes every variable from the environment.
#[test]
fn test_clear() {
    let _guard = setup();
    assert_eq!(set("TEST_CLEAR_A", b"1", true), Ok(true));
    assert_eq!(set("TEST_CLEAR_B", b"2", true), Ok(true));
    clear();
    assert!(get("TEST_CLEAR_A").is_null());
    assert!(get("TEST_CLEAR_B").is_null());
}

/// Tests that overwrite=false preserves an existing variable.
#[test]
fn test_set_no_overwrite_existing() {
    let _guard = setup();
    assert_eq!(set("TEST_KEY_2", b"first", true), Ok(true));
    assert_eq!(set("TEST_KEY_2", b"second", false), Ok(false));
    let ptr: *const c_char = get("TEST_KEY_2");
    let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
    assert_eq!(value.to_str(), Ok("first"));
}

/// Tests that overwrite=true replaces an existing variable.
#[test]
fn test_set_overwrite_existing() {
    let _guard = setup();
    assert_eq!(set("TEST_KEY_3", b"first", true), Ok(true));
    assert_eq!(set("TEST_KEY_3", b"second", true), Ok(true));
    let ptr: *const c_char = get("TEST_KEY_3");
    let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
    assert_eq!(value.to_str(), Ok("second"));
}

/// Tests that set returns Ok(true) for a new variable regardless of overwrite flag.
#[test]
fn test_set_new_variable_no_overwrite() {
    let _guard = setup();
    assert_eq!(set("TEST_KEY_4", b"value", false), Ok(true));
    let ptr: *const c_char = get("TEST_KEY_4");
    assert!(!ptr.is_null());
}

/// Tests that setting a variable with an empty key fails.
#[test]
fn test_set_empty_key() {
    let _guard = setup();
    assert!(set("", b"value", true).is_err());
}

/// Tests that setting a variable with '=' in the key fails.
#[test]
fn test_set_key_with_equals() {
    let _guard = setup();
    assert!(set("BAD=KEY", b"value", true).is_err());
}

/// Tests that unset removes a variable.
#[test]
fn test_unset() {
    let _guard = setup();
    assert_eq!(set("TEST_KEY_5", b"value", true), Ok(true));
    unset("TEST_KEY_5");
    let ptr: *const c_char = get("TEST_KEY_5");
    assert!(ptr.is_null());
}

/// Tests that unset on a non-existent variable is a no-op.
#[test]
fn test_unset_missing() {
    let _guard = setup();
    unset("TEST_KEY_NEVER_SET");
}

/// Tests that `snapshot` serializes entries as `KEY=VALUE` tokens.
#[test]
fn test_snapshot() {
    let _guard = setup();
    assert_eq!(set("SNAPSHOT_KEY_A", b"one", true), Ok(true));
    assert_eq!(set("SNAPSHOT_KEY_B", b"two", true), Ok(true));
    let snap: Vec<String> = snapshot();
    assert!(snap.iter().any(|token| token == "SNAPSHOT_KEY_A=one"));
    assert!(snap.iter().any(|token| token == "SNAPSHOT_KEY_B=two"));
}

/// Tests that a value containing '=' is stored and retrieved correctly.
#[test]
fn test_value_with_equals() {
    let _guard = setup();
    assert_eq!(set("TEST_KEY_EQ", b"a=b=c", true), Ok(true));
    let ptr: *const c_char = get("TEST_KEY_EQ");
    assert!(!ptr.is_null());
    let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
    assert_eq!(value.to_str(), Ok("a=b=c"));
}

/// Tests that `init_from_raw` populates the table from a C-style `envp`.
#[test]
fn test_init_from_raw() {
    let _guard = setup();
    let entries: [&[u8]; 3] = [b"FOO=bar\0", b"BAZ=qux\0", b"\0"];
    let ptrs: [*const c_char; 3] = [
        entries[0].as_ptr().cast::<c_char>(),
        entries[1].as_ptr().cast::<c_char>(),
        ::core::ptr::null(),
    ];
    unsafe {
        init_from_raw(ptrs.as_ptr());
    }
    let foo: *const c_char = get("FOO");
    assert!(!foo.is_null());
    let foo_val: &ffi::CStr = unsafe { ffi::CStr::from_ptr(foo) };
    assert_eq!(foo_val.to_str(), Ok("bar"));
    let baz: *const c_char = get("BAZ");
    assert!(!baz.is_null());
    let baz_val: &ffi::CStr = unsafe { ffi::CStr::from_ptr(baz) };
    assert_eq!(baz_val.to_str(), Ok("qux"));
}

/// Tests that `init_from_raw` with a null pointer is a no-op.
#[test]
fn test_init_from_raw_null() {
    let _guard = setup();
    unsafe {
        init_from_raw(::core::ptr::null());
    }
    // Table should remain empty.
    let ptr: *const c_char = get("ANYTHING");
    assert!(ptr.is_null());
}

/// Tests that the registered setenv callback is invoked on `set()`.
#[test]
fn test_setenv_callback() {
    use ::core::sync::atomic::{
        AtomicUsize,
        Ordering,
    };
    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    fn counting_callback(_key: &str, _value: &[u8]) {
        CALL_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    let _guard = setup();
    CALL_COUNT.store(0, Ordering::Relaxed);
    register_setenv_callback(counting_callback);

    // New variable — callback should fire.
    assert_eq!(set("CB_KEY", b"v1", true), Ok(true));
    assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 1);

    // Overwrite — callback should fire again.
    assert_eq!(set("CB_KEY", b"v2", true), Ok(true));
    assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 2);

    // No overwrite on existing key — callback should NOT fire.
    assert_eq!(set("CB_KEY", b"v3", false), Ok(false));
    assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 2);

    // Reset global state for other tests.
    let mut guard: spin::MutexGuard<'_, Option<SetenvCallback>> = SETENV_CALLBACK.lock();
    *guard = None;
}

/// Tests that `init_from_raw` deduplicates keys, with later entries overriding earlier ones.
#[test]
fn test_init_from_raw_dedup() {
    let _guard = setup();
    let entries: [&[u8]; 4] = [b"DUP=first\0", b"OTHER=val\0", b"DUP=second\0", b"\0"];
    let ptrs: [*const c_char; 4] = [
        entries[0].as_ptr().cast::<c_char>(),
        entries[1].as_ptr().cast::<c_char>(),
        entries[2].as_ptr().cast::<c_char>(),
        ::core::ptr::null(),
    ];
    unsafe {
        init_from_raw(ptrs.as_ptr());
    }
    let dup: *const c_char = get("DUP");
    assert!(!dup.is_null());
    let dup_val: &ffi::CStr = unsafe { ffi::CStr::from_ptr(dup) };
    assert_eq!(dup_val.to_str(), Ok("second"));
}

/// Tests that non-UTF-8 values (e.g., 0xFF) are accepted and preserved.
#[test]
fn test_set_non_utf8_value() {
    let _guard = setup();
    assert_eq!(set("BYTES_KEY", b"\xff", true), Ok(true));
    let ptr: *const c_char = get("BYTES_KEY");
    assert!(!ptr.is_null());
    let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
    assert_eq!(value.to_bytes(), b"\xff");
}

/// Tests that values containing interior NUL bytes are rejected.
#[test]
fn test_set_interior_nul_rejected() {
    let _guard = setup();
    assert!(set("NUL_KEY", b"a\0b", true).is_err());
}

/// Tests that `init_from_raw` preserves non-UTF-8 values.
#[test]
fn test_init_from_raw_non_utf8_value() {
    let _guard = setup();
    let entry: &[u8] = b"BIN=\xff\xfe\0";
    let ptrs: [*const c_char; 2] = [entry.as_ptr().cast::<c_char>(), ::core::ptr::null()];
    unsafe {
        init_from_raw(ptrs.as_ptr());
    }
    let ptr: *const c_char = get("BIN");
    assert!(!ptr.is_null());
    let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
    assert_eq!(value.to_bytes(), b"\xff\xfe");
}

/// Tests that `init_from_raw` skips malformed entries (those lacking a `=`) while still
/// importing the well-formed entries that surround them.
#[test]
fn test_init_from_raw_skips_malformed() {
    let _guard = setup();
    let entries: [&[u8]; 4] = [b"GOOD=1\0", b"MALFORMED\0", b"ALSO=2\0", b"\0"];
    let ptrs: [*const c_char; 4] = [
        entries[0].as_ptr().cast::<c_char>(),
        entries[1].as_ptr().cast::<c_char>(),
        entries[2].as_ptr().cast::<c_char>(),
        ::core::ptr::null(),
    ];
    unsafe {
        init_from_raw(ptrs.as_ptr());
    }
    let good: *const c_char = get("GOOD");
    assert!(!good.is_null());
    let good_val: &ffi::CStr = unsafe { ffi::CStr::from_ptr(good) };
    assert_eq!(good_val.to_str(), Ok("1"));
    let also: *const c_char = get("ALSO");
    assert!(!also.is_null());
    let also_val: &ffi::CStr = unsafe { ffi::CStr::from_ptr(also) };
    assert_eq!(also_val.to_str(), Ok("2"));
    // The malformed token carried no `=`, so it must not become an environment entry.
    assert!(get("MALFORMED").is_null());
}

/// Tests that `init_from_raw` with a non-null `envp` whose first element is the terminating
/// NULL produces an empty table (covers the non-null-pointer path distinct from `envp.is_null()`).
#[test]
fn test_init_from_raw_empty_array() {
    let _guard = setup();
    let ptrs: [*const c_char; 1] = [::core::ptr::null()];
    unsafe {
        init_from_raw(ptrs.as_ptr());
    }
    assert!(get("ANYTHING").is_null());
}

/// Reads the [`ENVIRON_ARRAY`] back into owned `KEY=VALUE` strings, dereferencing each published
/// pointer up to the terminating NULL. Mirrors how C code walks `char **environ`.
fn environ_view() -> Vec<String> {
    let array: spin::MutexGuard<'_, Vec<usize>> = ENVIRON_ARRAY.lock();
    let mut out: Vec<String> = Vec::new();
    for &addr in array.iter() {
        if addr == 0 {
            break;
        }
        let s: &ffi::CStr = unsafe { ffi::CStr::from_ptr(addr as *const c_char) };
        if let Ok(text) = s.to_str() {
            out.push(String::from(text));
        }
    }
    out
}

/// Tests that the `environ` view tracks `set()`, `unset()`, and `clear()` mutations.
#[test]
fn environ_tracks_set_unset_clear() {
    let _guard = setup();
    // `setup()` cleared the table directly; the first mutation rebuilds the view from scratch.
    assert_eq!(set("EVZ_A", b"1", true), Ok(true));
    assert_eq!(set("EVZ_B", b"2", true), Ok(true));
    let mut view: Vec<String> = environ_view();
    view.sort();
    assert_eq!(view, vec![String::from("EVZ_A=1"), String::from("EVZ_B=2")]);

    // Overwriting an existing key keeps a single, updated entry.
    assert_eq!(set("EVZ_A", b"9", true), Ok(true));
    let mut view: Vec<String> = environ_view();
    view.sort();
    assert_eq!(view, vec![String::from("EVZ_A=9"), String::from("EVZ_B=2")]);

    unset("EVZ_A");
    assert_eq!(environ_view(), vec![String::from("EVZ_B=2")]);

    clear();
    assert!(environ_view().is_empty());
}

/// Tests that `put_raw()` publishes the caller-owned string through the `environ` view.
#[test]
fn environ_tracks_put_raw() {
    let _guard = setup();
    clear();
    let raw: &[u8] = b"EVZ_P=val\0";
    assert_eq!(unsafe { put_raw(raw.as_ptr() as *mut c_char) }, Ok(()));
    assert_eq!(environ_view(), vec![String::from("EVZ_P=val")]);
}

/// Tests that `init_from_raw()` publishes the startup environment through the `environ` view.
#[test]
fn environ_tracks_init_from_raw() {
    let _guard = setup();
    let entries: [&[u8]; 2] = [b"EVZ_I=seed\0", b"\0"];
    let ptrs: [*const c_char; 2] = [entries[0].as_ptr().cast::<c_char>(), ::core::ptr::null()];
    unsafe {
        init_from_raw(ptrs.as_ptr());
    }
    assert_eq!(environ_view(), vec![String::from("EVZ_I=seed")]);
}
