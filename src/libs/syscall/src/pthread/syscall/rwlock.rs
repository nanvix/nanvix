// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::{
    boxed::Box,
    collections::btree_map::{
        BTreeMap,
        Entry,
    },
    sync::Arc,
};
use ::spin::{
    Lazy,
    Mutex,
    MutexGuard,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm::{
        lock_mutex,
        signal_cond,
        unlock_mutex,
        wait_cond,
    },
    pm::{
        ConditionAddress,
        MutexAddress,
    },
};
use ::sysapi::{
    pthread::PTHREAD_RWLOCK_INITIALIZER,
    sys_types::{
        pthread_rwlock_t,
        pthread_rwlockattr_t,
    },
};

//==================================================================================================
// Read-Write Lock Runtime Structure
//==================================================================================================

/// Runtime state stored for each read-write lock.
#[derive(Debug)]
struct ReadWriteLockState {
    /// User-provided (or default) attributes.
    _attr: pthread_rwlockattr_t,
    /// Number of readers currently holding the lock.
    readers: usize,
    /// Whether a writer currently holds the lock.
    writer_active: bool,
    /// Number of writers waiting. Used to give preference to writers once they arrive.
    writers_waiting: usize,
    /// Raw allocation backing the state mutex address.
    state_mutex: *mut u8,
    /// Raw allocation backing the readers condition variable address.
    readers_cond: *mut u8,
    /// Raw allocation backing the writers condition variable address.
    writers_cond: *mut u8,
}

impl ReadWriteLockState {
    ///
    /// # Description
    ///
    /// Creates a new `ReadWriteLockState` with the given attributes.
    ///
    /// # Parameters
    ///
    /// - `attr`: The attributes to use for the read-write lock.
    ///
    /// # Return Value
    ///
    /// This function returns a new instance of `ReadWriteLockState`.
    ///
    fn new(attr: pthread_rwlockattr_t) -> Self {
        // Allocate three distinct heap cells to serve as unique stable addresses for the underlying
        // kernel synchronization primitives.
        let state_mutex: *mut u8 = Box::into_raw(Box::new(0u8));
        let readers_cond: *mut u8 = Box::into_raw(Box::new(0u8));
        let writers_cond: *mut u8 = Box::into_raw(Box::new(0u8));
        Self {
            _attr: attr,
            readers: 0,
            writer_active: false,
            writers_waiting: 0,
            state_mutex,
            readers_cond,
            writers_cond,
        }
    }
}

// SAFETY: `ReadWriteLockState` only contains primitive integers and
// `MutexAddress`/`ConditionAddress` (newtype wrappers around integers). The raw pointers were only
// used to provide stable unique addresses; they are not dereferenced after creation. Thus it is
// safe to share between threads.
unsafe impl Send for ReadWriteLockState {}
unsafe impl Sync for ReadWriteLockState {}

impl Drop for ReadWriteLockState {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: Pointers were obtained from Box::into_raw in new() and we ensure drop
            // occurs exactly once when the runtime structure is removed from the global map.
            let _ = Box::from_raw(self.state_mutex);
            let _ = Box::from_raw(self.readers_cond);
            let _ = Box::from_raw(self.writers_cond);
        }
    }
}

//==================================================================================================
// Type Aliases
//==================================================================================================

/// Read-write lock.
type ReadWriteLock = Arc<Mutex<ReadWriteLockState>>;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Global map of read-write locks keyed by address of the user-space object.
static RWLOCKS: Lazy<Mutex<BTreeMap<usize, ReadWriteLock>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes a read-write lock.
///
/// # Parameters
///
/// - `rwlock`: Read-write lock object.
/// - `attr`: Read-write lock attributes.
///
/// # Return Value
///
/// On success, this function returns empty. On failure, it returns an error describing the reason
/// for the failure.
///
pub fn pthread_rwlock_init(
    rwlock: &mut pthread_rwlock_t,
    attr: &pthread_rwlockattr_t,
) -> Result<(), Error> {
    // Register read-write lock if it is not yet registered.
    if let Entry::Vacant(entry) = RWLOCKS
        .lock()
        .entry(rwlock as *const pthread_rwlock_t as usize)
    {
        // Register read-write lock.
        entry.insert(Arc::new(Mutex::new(ReadWriteLockState::new(*attr))));
        Ok(())
    } else {
        let reason: &str = "read-write lock is already initialized";
        ::syslog::error!("pthread_rwlock_init(): {reason}");
        Err(Error::new(ErrorCode::InvalidArgument, reason))
    }
}

///
/// # Description
///
/// Destroys a read-write lock.
///
/// # Parameters
///
/// - `rwlock`: Read-write lock object.
///
/// # Return Value
///
/// On success, this function returns empty. On failure, it returns an error describing the reason
/// for the failure.
///
pub fn pthread_rwlock_destroy(rwlock: &mut pthread_rwlock_t) -> Result<(), Error> {
    let mut rwlocks: MutexGuard<'_, BTreeMap<usize, ReadWriteLock>> = RWLOCKS.lock();

    // Check if read-write lock was not registered.
    if let Some(runtime) = rwlocks.get(&(rwlock as *const pthread_rwlock_t as usize)) {
        let locked_runtime_rwlock: MutexGuard<'_, ReadWriteLockState> = runtime.lock();
        if locked_runtime_rwlock.readers != 0 || locked_runtime_rwlock.writer_active {
            let reason: &str = "read-write lock is busy";
            ::syslog::error!("pthread_rwlock_destroy(): {}", reason);
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }
    } else {
        // Not registered; accept destroy if statically initialized.
        if *rwlock == PTHREAD_RWLOCK_INITIALIZER {
            return Ok(());
        } else {
            let reason: &str = "read-write lock is not initialized";
            ::syslog::error!("pthread_rwlock_destroy(): {reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    // Remove read-write lock from the table of read-write locks.
    rwlocks.remove(&(rwlock as *const pthread_rwlock_t as usize));

    Ok(())
}

///
/// # Description
///
/// Acquires a read lock on a read-write lock.
///
/// # Parameters
///
/// - `rwlock`: Read-write lock object.
///
/// # Return Value
///
/// On success, this function returns empty. On failure, it returns an error describing the reason
/// for the failure.
///
pub fn pthread_rwlock_rdlock(rwlock: &mut pthread_rwlock_t) -> Result<(), Error> {
    let runtime_rwlock: ReadWriteLock = get_runtime_rwlock(rwlock)?;

    let (mutex_addr, readers_cond_addr): (MutexAddress, ConditionAddress) = {
        let locked_runtime_rwlock: MutexGuard<'_, ReadWriteLockState> = runtime_rwlock.lock();
        let mutex_addr: MutexAddress =
            MutexAddress::from(locked_runtime_rwlock.state_mutex as usize);
        let readers_cond_addr: ConditionAddress =
            ConditionAddress::from(locked_runtime_rwlock.readers_cond as usize);
        (mutex_addr, readers_cond_addr)
    };

    // Protect state.
    lock_mutex(mutex_addr, None)?;

    loop {
        let should_wait: bool = {
            let mut locked_runtime_rwlock: MutexGuard<'_, ReadWriteLockState> =
                runtime_rwlock.lock();
            if !locked_runtime_rwlock.writer_active && locked_runtime_rwlock.writers_waiting == 0 {
                locked_runtime_rwlock.readers += 1;
                false
            } else {
                true
            }
        };

        if !should_wait {
            unlock_mutex(mutex_addr)?;
            break Ok(());
        }
        wait_cond(readers_cond_addr, mutex_addr, None)?;
    }
}

///
/// # Description
///
/// Acquires a write lock on a read-write lock.
///
/// # Parameters
///
/// - `rwlock`: Read-write lock object.
///
/// # Return Value
///
/// On success, this function returns empty. On failure, it returns an error describing the reason
/// for the failure.
///
pub fn pthread_rwlock_wrlock(rwlock: &mut pthread_rwlock_t) -> Result<(), Error> {
    let runtime_rwlock: ReadWriteLock = get_runtime_rwlock(rwlock)?;

    let (mutex_addr, writers_cond_addr): (MutexAddress, ConditionAddress) = {
        let locked_runtime_rwlock: MutexGuard<'_, ReadWriteLockState> = runtime_rwlock.lock();
        let mutex_addr: MutexAddress =
            MutexAddress::from(locked_runtime_rwlock.state_mutex as usize);
        let writers_cond_addr: ConditionAddress =
            ConditionAddress::from(locked_runtime_rwlock.writers_cond as usize);
        (mutex_addr, writers_cond_addr)
    };

    lock_mutex(mutex_addr, None)?;

    {
        let mut runtime: MutexGuard<'_, ReadWriteLockState> = runtime_rwlock.lock();
        runtime.writers_waiting += 1;
    }

    loop {
        let mut acquired: bool = false;
        {
            let mut runtime: MutexGuard<'_, ReadWriteLockState> = runtime_rwlock.lock();
            if !runtime.writer_active && runtime.readers == 0 {
                runtime.writer_active = true;
                runtime.writers_waiting -= 1;
                acquired = true;
            }
        }

        if acquired {
            unlock_mutex(mutex_addr)?;
            break Ok(());
        }
        wait_cond(writers_cond_addr, mutex_addr, None)?;
    }
}

///
/// # Description
///
/// Releases a read or write lock held on a read-write lock.
///
/// # Parameters
///
/// - `rwlock`: Read-write lock object.
///
/// # Return Value
///
/// On success, this function returns empty. On failure, it returns an error describing the reason
/// for the failure.
///
pub fn pthread_rwlock_unlock(rwlock: &mut pthread_rwlock_t) -> Result<(), Error> {
    let runtime_rwlock: ReadWriteLock = get_runtime_rwlock(rwlock)?;

    let (mutex_addr, readers_cond_addr, writers_cond_addr): (
        MutexAddress,
        ConditionAddress,
        ConditionAddress,
    ) = {
        let locked_runtime_rwlock: MutexGuard<'_, ReadWriteLockState> = runtime_rwlock.lock();
        let mutex_addr: MutexAddress =
            MutexAddress::from(locked_runtime_rwlock.state_mutex as usize);
        let readers_cond_addr: ConditionAddress =
            ConditionAddress::from(locked_runtime_rwlock.readers_cond as usize);
        let writers_cond_addr: ConditionAddress =
            ConditionAddress::from(locked_runtime_rwlock.writers_cond as usize);
        (mutex_addr, readers_cond_addr, writers_cond_addr)
    };

    lock_mutex(mutex_addr, None)?;

    let (wake_readers, wake_writer): (bool, bool) = {
        let mut locked_runtime: MutexGuard<'_, ReadWriteLockState> = runtime_rwlock.lock();
        if locked_runtime.writer_active {
            locked_runtime.writer_active = false;
            if locked_runtime.writers_waiting > 0 {
                (false, true)
            } else {
                // No waiting writers; wake all readers.
                (true, false)
            }
        } else {
            // Must be a reader.
            if locked_runtime.readers == 0 {
                let reason: &str = "unlock on unlocked read-write lock";
                ::syslog::error!("pthread_rwlock_unlock(): {reason}");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
            locked_runtime.readers -= 1;
            if locked_runtime.readers == 0 && locked_runtime.writers_waiting > 0 {
                (false, true)
            } else {
                (false, false)
            }
        }
    };

    // Wake appropriate waiters while still holding protection mutex so they observe state.
    if wake_writer {
        let _ = signal_cond(writers_cond_addr, false);
    } else if wake_readers {
        let _ = signal_cond(readers_cond_addr, true); // broadcast to all readers
    }

    unlock_mutex(mutex_addr)
}

///
/// # Description
///
/// Gets the runtime structure for a read-write lock, registering the read-write lock in the table
/// of read-write locks if necessary.
///
/// # Parameters
///
/// - `rwlock`: Read-write lock object.
///
/// # Return Value
///
/// On success, this function returns the runtime structure for the read-write lock. On failure,
/// it returns an error describing the reason for the failure.
///
fn get_runtime_rwlock(rwlock: &pthread_rwlock_t) -> Result<ReadWriteLock, Error> {
    let mut rwlocks: MutexGuard<'_, BTreeMap<usize, ReadWriteLock>> = RWLOCKS.lock();
    let runtime_ptr: usize = rwlock as *const pthread_rwlock_t as usize;
    if let Entry::Vacant(entry) = rwlocks.entry(runtime_ptr) {
        if *rwlock == PTHREAD_RWLOCK_INITIALIZER {
            entry.insert(Arc::new(Mutex::new(ReadWriteLockState::new(
                pthread_rwlockattr_t::default(),
            ))));
        } else {
            let reason: &str = "read-write lock is not initialized";
            ::syslog::error!("lazy_register_rwlock(): {reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
    }

    // Borrow runtime (need to re-lock map only when reading state). We clone needed addresses.
    let runtime: &ReadWriteLock = rwlocks
        .get(&runtime_ptr)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "read-write lock runtime missing"))?;

    Ok(runtime.clone())
}
