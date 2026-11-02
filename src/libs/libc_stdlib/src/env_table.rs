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
mod test;
