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
    /// Storage for the null-terminated `KEY=VALUE` C string.
    storage: EnvStorage,
}

/// Storage backing for an environment variable entry.
enum EnvStorage {
    /// Owned storage used by `setenv()` and process startup initialization.
    Owned(Vec<u8>),
    /// Caller-owned storage installed by `putenv()`.
    Borrowed(usize),
}

//==================================================================================================
// Global State
//==================================================================================================

/// Process-local environment variable table.
static ENV_TABLE: Lazy<Mutex<Vec<EnvEntry>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Optional callback invoked after a successful `set()` that writes a new or updated value.
static SETENV_CALLBACK: Lazy<Mutex<Option<SetenvCallback>>> = Lazy::new(|| Mutex::new(None));

/// Null-terminated array of pointers that backs the C-visible `environ` global.
///
/// One entry per environment variable (each pointing at a table entry's `KEY=VALUE` C string),
/// followed by a single `0` terminator. Pointer values are stored as `usize` (rather than
/// `*mut c_char`) so the static is `Send`/`Sync`, matching how [`EnvStorage::Borrowed`] keeps its
/// address. It is rebuilt by [`sync_environ`] after every mutation and the `environ` global is
/// repointed at its buffer, so C code reading `extern char **environ` always observes the current
/// environment.
static ENVIRON_ARRAY: Lazy<Mutex<Vec<usize>>> = Lazy::new(|| Mutex::new(Vec::new()));

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
    let mut table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();
    table.clear();

    if envp.is_null() {
        sync_environ(&table);
        return;
    }

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

    // Publish the freshly populated table through the C-visible `environ` global.
    sync_environ(&table);
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
        if let Some(value_ptr) = entry.value_ptr_if_key(key) {
            return value_ptr;
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

    // Locate an existing entry by index so the table can be reborrowed for `sync_environ` after the
    // mutation without holding a `iter_mut()` borrow across the call.
    let existing: Option<usize> = table.iter().position(|entry| entry.matches_key(key));
    match existing {
        Some(idx) => {
            if !overwrite {
                return Ok(false);
            }
            table[idx] = EnvEntry::owned(key, value);
        },
        None => insert_entry(&mut table, key, value),
    }

    // Repoint `environ`, then release the lock before invoking the callback.
    sync_environ(&table);
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
    table.retain(|entry| !entry.matches_key(key));
    sync_environ(&table);
}

///
/// # Description
///
/// Removes every variable from the environment table, leaving it empty.
///
pub fn clear() {
    let mut table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();
    table.clear();
    sync_environ(&table);
}

///
/// # Description
///
/// Installs a caller-owned `KEY=VALUE` string in the environment.
///
/// # Parameters
///
/// - `string`: Pointer to the caller-owned null-terminated environment string.
///
/// # Returns
///
/// `Ok(())` if the entry was installed, or `Err(())` if the string is not of the form
/// `KEY=VALUE` with a non-empty UTF-8 key.
///
/// # Safety
///
/// `string` must be a valid null-terminated C string. The caller must keep the storage valid while
/// it remains part of the environment.
///
#[allow(clippy::result_unit_err)]
pub unsafe fn put_raw(string: *mut c_char) -> Result<(), ()> {
    let bytes: &[u8] = ffi::CStr::from_ptr(string).to_bytes();
    let Some(eq_pos) = bytes.iter().position(|&b| b == b'=') else {
        return Err(());
    };
    if eq_pos == 0 {
        return Err(());
    }
    let key: &str = ::core::str::from_utf8(&bytes[..eq_pos]).map_err(|_| ())?;
    let value: &[u8] = &bytes[eq_pos + 1..];

    let mut table: spin::MutexGuard<'_, Vec<EnvEntry>> = ENV_TABLE.lock();
    let existing: Option<usize> = table.iter().position(|entry| entry.matches_key(key));
    match existing {
        Some(idx) => table[idx] = EnvEntry::borrowed(string),
        None => table.push(EnvEntry::borrowed(string)),
    }
    sync_environ(&table);
    drop(table);
    invoke_setenv_callback(key, value);
    Ok(())
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
        let bytes: &[u8] = entry.raw_bytes_with_nul();
        let bytes: &[u8] = &bytes[..bytes.len().saturating_sub(1)];
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

/// Rebuilds the C-visible [`ENVIRON_ARRAY`] from the current table and repoints the `environ`
/// global at it.
///
/// The caller must hold the [`ENV_TABLE`] lock and pass the live entries, so this runs while the
/// table is quiesced and the rebuilt pointer array is always consistent with it. Each surviving
/// entry contributes the address of its `KEY=VALUE` C string; a trailing `0` terminates the array,
/// matching the `char **environ` contract.
///
/// In a hosted (unit-test) build there is no C `environ` symbol to publish, so the array is still
/// maintained — letting the table logic be exercised directly — but the global write is compiled
/// out. In the freestanding libc the `environ` global is updated in place.
fn sync_environ(table: &[EnvEntry]) {
    let mut array: spin::MutexGuard<'_, Vec<usize>> = ENVIRON_ARRAY.lock();
    array.clear();
    array.reserve(table.len() + 1);
    for entry in table.iter() {
        array.push(entry.raw_ptr() as usize);
    }
    array.push(0);

    let base: usize = array.as_ptr() as usize;

    #[cfg(not(feature = "std"))]
    {
        unsafe extern "C" {
            static mut environ: *mut *mut c_char;
        }
        // SAFETY: `environ` is the process-wide `char **environ` defined by libposix startup. The
        // write is performed under the `ENV_TABLE` lock (held by the caller) and `ENVIRON_ARRAY`
        // lock (held here), serializing it against any concurrent rebuild. Reads of `environ` from
        // C are inherently unsynchronized, exactly as on a conventional libc.
        unsafe {
            let slot: *mut *mut *mut c_char = &raw mut environ;
            *slot = base as *mut *mut c_char;
        }
    }
    #[cfg(feature = "std")]
    let _ = base;
}

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
    table.push(EnvEntry::owned(key, value));
}

/// Inserts or updates an entry in the table. If `key` already exists, its value is replaced.
fn upsert_entry(table: &mut Vec<EnvEntry>, key: &str, value: &[u8]) {
    for entry in table.iter_mut() {
        if entry.matches_key(key) {
            *entry = EnvEntry::owned(key, value);
            return;
        }
    }
    insert_entry(table, key, value);
}

impl EnvEntry {
    /// Creates an owned environment entry.
    fn owned(key: &str, value: &[u8]) -> Self {
        Self {
            storage: EnvStorage::Owned(make_raw(key, value)),
        }
    }

    /// Creates a borrowed environment entry.
    fn borrowed(ptr: *mut c_char) -> Self {
        Self {
            storage: EnvStorage::Borrowed(ptr as usize),
        }
    }

    /// Returns a pointer to the first byte of the null-terminated `KEY=VALUE` C string, suitable for
    /// publishing through the `environ` array.
    fn raw_ptr(&self) -> *const c_char {
        match &self.storage {
            EnvStorage::Owned(raw) => raw.as_ptr().cast::<c_char>(),
            EnvStorage::Borrowed(addr) => *addr as *const c_char,
        }
    }

    /// Returns the raw null-terminated `KEY=VALUE` bytes.
    fn raw_bytes_with_nul(&self) -> &[u8] {
        match &self.storage {
            EnvStorage::Owned(raw) => raw,
            EnvStorage::Borrowed(addr) => unsafe {
                ffi::CStr::from_ptr((*addr) as *const c_char).to_bytes_with_nul()
            },
        }
    }

    /// Returns the offset of the value if this entry currently names `key`.
    fn value_offset_if_key(&self, key: &str) -> Option<usize> {
        let bytes: &[u8] = self.raw_bytes_with_nul();
        let eq_pos: usize = bytes.iter().position(|&b| b == b'=')?;
        if ::core::str::from_utf8(&bytes[..eq_pos]).ok()? == key {
            Some(eq_pos + 1)
        } else {
            None
        }
    }

    /// Returns `true` if this entry currently names `key`.
    fn matches_key(&self, key: &str) -> bool {
        self.value_offset_if_key(key).is_some()
    }

    /// Returns the value pointer if this entry currently names `key`.
    fn value_ptr_if_key(&self, key: &str) -> Option<*const c_char> {
        let offset: usize = self.value_offset_if_key(key)?;
        match &self.storage {
            EnvStorage::Owned(raw) => Some(raw[offset..].as_ptr().cast::<c_char>()),
            EnvStorage::Borrowed(addr) => Some((*addr + offset) as *const c_char),
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
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
}
