// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::btree_map::BTreeMap;
use ::spin::{
    Lazy,
    Mutex,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::{
    limits::PTHREAD_KEYS_MAX,
    sys_types::pthread_key_t,
};

//==================================================================================================
// Globals
//==================================================================================================

/// Table of thread-specific data.
static THREAD_DATA: Lazy<Mutex<[Option<ThreadData>; PTHREAD_KEYS_MAX]>> =
    Lazy::new(|| Mutex::new([const { None }; PTHREAD_KEYS_MAX]));

//==================================================================================================
// Pointer
//==================================================================================================

#[derive(Debug, Clone)]
pub struct Pointer {
    addr: usize,
}

impl Pointer {
    pub fn null() -> Pointer {
        Pointer { addr: 0 }
    }
}

impl<T> From<*const T> for Pointer {
    fn from(ptr: *const T) -> Pointer {
        Pointer { addr: ptr as usize }
    }
}

impl<T> From<Pointer> for *const T {
    fn from(ptr: Pointer) -> *const T {
        ptr.addr as *const T
    }
}

impl<T> From<*mut T> for Pointer {
    fn from(ptr: *mut T) -> Pointer {
        Pointer { addr: ptr as usize }
    }
}

impl<T> From<Pointer> for *mut T {
    fn from(ptr: Pointer) -> *mut T {
        ptr.addr as *mut T
    }
}

//==================================================================================================
// Thread Data
//==================================================================================================

#[derive(Default, Debug, Clone)]
pub struct ThreadData {
    value: BTreeMap<ThreadIdentifier, Pointer>,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a thread-specific data key.
///
/// # Returns
///
/// If successful, the newly created key is returned. Otherwise, `None` is returned instead.
///
pub fn pthread_key_create() -> Option<pthread_key_t> {
    THREAD_DATA
        .lock()
        .iter_mut()
        .enumerate()
        .find_map(|(i, key)| {
            if key.is_none() {
                *key = Some(ThreadData::default());
                Some(i as pthread_key_t)
            } else {
                None
            }
        })
}

///
/// # Description
///
/// Deletes a thread-specific data key.
///
/// # Parameters
///
/// - `key`: Key
///
/// # Returns
///
/// If successful, `Ok(())` is returned. Otherwise, an error is returned instead.
///
pub fn pthread_key_delete(key: pthread_key_t) -> Result<(), Error> {
    match THREAD_DATA.lock().get_mut(key as usize) {
        Some(tda) => {
            if let Some(tda) = tda {
                for (tid, ptr) in tda.value.iter() {
                    // Warn if we are leaking data.
                    ::syslog::debug!(
                        "pthread_key_delete(): leaking thread data (tid={:?}, ptr={:#x?})",
                        tid,
                        ptr
                    );
                }
                tda.value.clear();
            }
            *tda = None;
            Ok(())
        },
        None => {
            let reason: &str = "key not found";
            ::syslog::error!("pthread_key_delete(): {:?}", reason);
            Err(Error::new(ErrorCode::NoSuchEntry, reason))
        },
    }
}

///
/// # Description
///
/// Gets the value associated with a thread-specific data key.
///
/// # Parameters
///
///  - `key`: Key
///
/// # Returns
///
/// If successful, the value associated with the key is returned. Otherwise, an error is returned
/// instead.
///
pub fn pthread_getspecific(key: pthread_key_t) -> Result<Pointer, Error> {
    // Lookup thread identifier before locking up the table of thread data keys.
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid().unwrap();

    // Lookup thread-specific data.
    match THREAD_DATA.lock().get(key as usize) {
        // Key found and it has data associated to it.
        Some(Some(tda)) => match tda.value.get(&tid) {
            // The calling thread associated data to the key.
            Some(ptr) => Ok(ptr.clone()),
            // The calling thread did not associate data to the key.
            None => Ok(Pointer::null()),
        },
        // Key not found.
        Some(None) | None => {
            let reason: &str = "key not found";
            ::syslog::error!("pthread_getspecific(): {:?}", reason);
            Err(Error::new(ErrorCode::NoSuchEntry, reason))
        },
    }
}

///
/// # Description
///
/// Sets the value associated with a thread-specific data key.
///
/// # Parameters
///
/// - `key`: Key
/// - `value`: Value
///
/// # Returns
///
/// If successful, `Ok(())` is returned. Otherwise, an error is returned instead.
///
pub fn pthread_setspecific(key: pthread_key_t, value: Pointer) -> Result<(), Error> {
    // Lookup thread identifier before locking up the table of thread data keys.
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid().unwrap();

    // Lookup thread-specific data.
    match THREAD_DATA.lock().get_mut(key as usize) {
        // Key found.
        Some(Some(tda)) => {
            tda.value.insert(tid, value);
            Ok(())
        },
        // Key not found.
        Some(None) | None => {
            let reason: &str = "key not found";
            ::syslog::error!("pthread_setspecific(): {:?}", reason);
            Err(Error::new(ErrorCode::NoSuchEntry, reason))
        },
    }
}
