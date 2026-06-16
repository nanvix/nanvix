// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::{
    string::String,
    vec::Vec,
};
use ::core::ffi;
use ::spin::{
    Lazy,
    Mutex,
};
use ::sysapi::ffi::c_char;

//==================================================================================================
// Types
//==================================================================================================

/// Callback invoked after `setenv()` successfully writes a variable.
///
/// Parameters: `(key, value)` — `key` is a valid UTF-8 `&str`, `value` is a raw byte slice
/// `&[u8]` (may contain non-UTF-8 bytes, but never interior NULs).
pub type SetenvCallback = fn(&str, &[u8]);

//==================================================================================================
// Structures
//==================================================================================================

/// A single environment variable entry.
struct EnvEntry {
    /// Variable name.
    key: String,
    /// Null-terminated "KEY=VALUE" C string for pointer stability from `getenv()`.
    raw: Vec<u8>,
}

//==================================================================================================
// Global State
//==================================================================================================

/// Process-local environment variable table.
static ENV_TABLE: Lazy<Mutex<Vec<EnvEntry>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Optional callback invoked after a successful `set()` that writes a new or updated value.
static SETENV_CALLBACK: Lazy<Mutex<Option<SetenvCallback>>> = Lazy::new(|| Mutex::new(None));

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the environment table from a raw `envp` pointer (null-terminated array of
/// null-terminated `KEY=VALUE` C strings). This is intended to be called once during process
/// startup.
///
/// # Parameters
///
/// - `envp`: A pointer to a null-terminated array of null-terminated C strings in `KEY=VALUE`
///   format. If null, the table is left empty.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// It is safe to call this function if and only if:
/// - `envp` is either null or points to a valid null-terminated array of null-terminated C strings.
/// - Each string in the array remains valid for the duration of this call.
///
pub unsafe fn init_from_raw(envp: *const *const c_char) {
    if envp.is_null() {
        return;
    }

    let mut table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();
    table.clear();

    let mut i: usize = 0;
    loop {
        let entry_ptr: *const c_char = *envp.add(i);
        if entry_ptr.is_null() {
            break;
        }

        let c_str: &ffi::CStr = ffi::CStr::from_ptr(entry_ptr);
        let bytes: &[u8] = c_str.to_bytes();
        if let Some(eq_pos) = bytes.iter().position(|&b| b == b'=') {
            let key_bytes: &[u8] = &bytes[..eq_pos];
            let value_bytes: &[u8] = &bytes[eq_pos + 1..];
            if let Ok(key) = ::core::str::from_utf8(key_bytes) {
                if !key.is_empty() {
                    upsert_entry(&mut table, key, value_bytes);
                }
            }
        }

        i += 1;
    }
}

///
/// # Description
///
/// Looks up an environment variable by name.
///
/// # Parameters
///
/// - `key`: The variable name to search for.
///
/// # Returns
///
/// A pointer to the value portion of the `KEY=VALUE` C string if the variable is found, or
/// `core::ptr::null()` if not found. The returned pointer remains valid until the next `set()` or
/// `unset()` call that modifies this key.
///
/// # Note
///
/// The mutex guard is released before the caller uses the returned pointer. In a concurrent
/// environment, another thread calling `set()` or `unset()` on the same key between this function's
/// return and the caller's use of the pointer would invalidate it. This matches POSIX `getenv()`
/// semantics, which are inherently not thread-safe.
///
pub fn get(key: &str) -> *const c_char {
    let table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();
    for entry in table.iter() {
        if entry.key == key {
            // Return pointer to the value portion, which starts after "KEY=".
            let offset: usize = entry.key.len() + 1;
            return entry.raw[offset..].as_ptr().cast::<c_char>();
        }
    }
    ::core::ptr::null()
}

///
/// # Description
///
/// Sets an environment variable. If `overwrite` is false and the variable already exists, the
/// existing value is preserved.
///
/// # Parameters
///
/// - `key`: The variable name. Must not be empty and must not contain `=`.
/// - `value`: The variable value as raw bytes. Must not contain interior NUL bytes.
/// - `overwrite`: If true, replace an existing variable; if false, keep the existing value.
///
/// # Returns
///
/// `Ok(true)` if the value was written (new variable or overwritten), `Ok(false)` if the variable
/// already existed and `overwrite` was false, or `Err(())` if `key` is empty, contains `=`, or
/// `value` contains an interior NUL.
///
#[allow(clippy::result_unit_err)]
pub fn set(key: &str, value: &[u8], overwrite: bool) -> Result<bool, ()> {
    if key.is_empty() || key.contains('=') || value.contains(&0) {
        return Err(());
    }

    let mut table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();

    // Check if the key already exists.
    for entry in table.iter_mut() {
        if entry.key == key {
            if overwrite {
                entry.raw = make_raw(key, value);
                // Release the lock before invoking the callback.
                drop(table);
                invoke_setenv_callback(key, value);
                return Ok(true);
            }
            return Ok(false);
        }
    }

    // Key not found, insert new entry.
    insert_entry(&mut table, key, value);
    // Release the lock before invoking the callback.
    drop(table);
    invoke_setenv_callback(key, value);
    Ok(true)
}

///
/// # Description
///
/// Removes an environment variable by name. If the variable does not exist, this is a no-op.
///
/// # Parameters
///
/// - `key`: The variable name to remove.
///
pub fn unset(key: &str) {
    let mut table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();
    table.retain(|entry| entry.key != key);
}

///
/// # Description
///
/// Serializes the current environment as a vector of owned `KEY=VALUE` tokens.
///
/// This is intended for the `exec`-family wrappers that must inherit the caller's environment: the
/// returned tokens can be borrowed as `&str` and handed to the kernel `execv` path, which expects
/// space-separated `KEY=VALUE` entries.
///
/// Entries whose `KEY=VALUE` text is not valid UTF-8 are omitted, because the `execv` wire format
/// (and the kernel) accept UTF-8 only; such an entry could not be conveyed to the new image
/// regardless.
///
/// # Returns
///
/// A vector of `KEY=VALUE` strings, one per environment variable, in table order.
///
pub fn snapshot() -> Vec<String> {
    let table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();
    let mut out: Vec<String> = Vec::with_capacity(table.len());
    for entry in table.iter() {
        // `raw` is a NUL-terminated "KEY=VALUE" C string; drop the trailing NUL before decoding.
        let bytes: &[u8] = &entry.raw[..entry.raw.len().saturating_sub(1)];
        if let Ok(token) = ::core::str::from_utf8(bytes) {
            out.push(String::from(token));
        }
    }
    out
}

///
/// # Description
///
/// Registers a callback that is invoked after every successful `set()` that writes a new or
/// updated value. Only one callback can be active at a time; a subsequent call replaces any
/// previously registered callback.
///
/// # Parameters
///
/// - `cb`: The callback function to register.
///
pub fn register_setenv_callback(cb: SetenvCallback) {
    let mut guard: spin::MutexGuard<'_, Option<SetenvCallback>> = SETENV_CALLBACK.lock();
    *guard = Some(cb);
}

/// Invokes the registered setenv callback, if any.
fn invoke_setenv_callback(key: &str, value: &[u8]) {
    if let Some(cb) = { *SETENV_CALLBACK.lock() } {
        cb(key, value);
    }
}

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Builds a null-terminated `KEY=VALUE\0` byte vector.
fn make_raw(key: &str, value: &[u8]) -> Vec<u8> {
    let mut raw: Vec<u8> = Vec::with_capacity(key.len() + 1 + value.len() + 1);
    raw.extend_from_slice(key.as_bytes());
    raw.push(b'=');
    raw.extend_from_slice(value);
    raw.push(0);
    raw
}

/// Inserts a new entry into the table.
fn insert_entry(table: &mut Vec<EnvEntry>, key: &str, value: &[u8]) {
    let raw: Vec<u8> = make_raw(key, value);
    table.push(EnvEntry {
        key: String::from(key),
        raw,
    });
}

/// Inserts or updates an entry in the table. If `key` already exists, its value is replaced.
fn upsert_entry(table: &mut Vec<EnvEntry>, key: &str, value: &[u8]) {
    for entry in table.iter_mut() {
        if entry.key == key {
            entry.raw = make_raw(key, value);
            return;
        }
    }
    insert_entry(table, key, value);
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;

    /// Clears all entries so each test starts with a clean table.
    fn clear() {
        let mut table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();
        table.clear();
    }

    /// Tests that setting and getting a variable returns the correct value.
    #[test]
    fn test_set_and_get() {
        clear();
        assert_eq!(set("TEST_KEY_1", b"hello", true), Ok(true));
        let ptr: *const c_char = get("TEST_KEY_1");
        assert!(!ptr.is_null());
        let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
        assert_eq!(value.to_str(), Ok("hello"));
    }

    /// Tests that getting a non-existent variable returns null.
    #[test]
    fn test_get_missing() {
        clear();
        let ptr: *const c_char = get("TEST_NONEXISTENT");
        assert!(ptr.is_null());
    }

    /// Tests that overwrite=false preserves an existing variable.
    #[test]
    fn test_set_no_overwrite_existing() {
        clear();
        assert_eq!(set("TEST_KEY_2", b"first", true), Ok(true));
        assert_eq!(set("TEST_KEY_2", b"second", false), Ok(false));
        let ptr: *const c_char = get("TEST_KEY_2");
        let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
        assert_eq!(value.to_str(), Ok("first"));
    }

    /// Tests that overwrite=true replaces an existing variable.
    #[test]
    fn test_set_overwrite_existing() {
        clear();
        assert_eq!(set("TEST_KEY_3", b"first", true), Ok(true));
        assert_eq!(set("TEST_KEY_3", b"second", true), Ok(true));
        let ptr: *const c_char = get("TEST_KEY_3");
        let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
        assert_eq!(value.to_str(), Ok("second"));
    }

    /// Tests that set returns Ok(true) for a new variable regardless of overwrite flag.
    #[test]
    fn test_set_new_variable_no_overwrite() {
        clear();
        assert_eq!(set("TEST_KEY_4", b"value", false), Ok(true));
        let ptr: *const c_char = get("TEST_KEY_4");
        assert!(!ptr.is_null());
    }

    /// Tests that setting a variable with an empty key fails.
    #[test]
    fn test_set_empty_key() {
        assert!(set("", b"value", true).is_err());
    }

    /// Tests that setting a variable with '=' in the key fails.
    #[test]
    fn test_set_key_with_equals() {
        assert!(set("BAD=KEY", b"value", true).is_err());
    }

    /// Tests that unset removes a variable.
    #[test]
    fn test_unset() {
        clear();
        assert_eq!(set("TEST_KEY_5", b"value", true), Ok(true));
        unset("TEST_KEY_5");
        let ptr: *const c_char = get("TEST_KEY_5");
        assert!(ptr.is_null());
    }

    /// Tests that unset on a non-existent variable is a no-op.
    #[test]
    fn test_unset_missing() {
        clear();
        unset("TEST_KEY_NEVER_SET");
    }

    /// Tests that `snapshot` serializes entries as `KEY=VALUE` tokens.
    #[test]
    fn test_snapshot() {
        clear();
        assert_eq!(set("SNAPSHOT_KEY_A", b"one", true), Ok(true));
        assert_eq!(set("SNAPSHOT_KEY_B", b"two", true), Ok(true));
        let snap: Vec<String> = snapshot();
        assert!(snap.iter().any(|token| token == "SNAPSHOT_KEY_A=one"));
        assert!(snap.iter().any(|token| token == "SNAPSHOT_KEY_B=two"));
    }

    /// Tests that a value containing '=' is stored and retrieved correctly.
    #[test]
    fn test_value_with_equals() {
        clear();
        assert_eq!(set("TEST_KEY_EQ", b"a=b=c", true), Ok(true));
        let ptr: *const c_char = get("TEST_KEY_EQ");
        assert!(!ptr.is_null());
        let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
        assert_eq!(value.to_str(), Ok("a=b=c"));
    }

    /// Tests that `init_from_raw` populates the table from a C-style `envp`.
    #[test]
    fn test_init_from_raw() {
        clear();
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
        clear();
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

        clear();
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
        clear();
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
        clear();
        assert_eq!(set("BYTES_KEY", b"\xff", true), Ok(true));
        let ptr: *const c_char = get("BYTES_KEY");
        assert!(!ptr.is_null());
        let value: &ffi::CStr = unsafe { ffi::CStr::from_ptr(ptr) };
        assert_eq!(value.to_bytes(), b"\xff");
    }

    /// Tests that values containing interior NUL bytes are rejected.
    #[test]
    fn test_set_interior_nul_rejected() {
        assert!(set("NUL_KEY", b"a\0b", true).is_err());
    }

    /// Tests that `init_from_raw` preserves non-UTF-8 values.
    #[test]
    fn test_init_from_raw_non_utf8_value() {
        clear();
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
}
