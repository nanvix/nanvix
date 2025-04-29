// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::NanvixUserland;
use ::alloc::boxed::Box;
use ::core::{
    panic,
    pin::Pin,
    sync::atomic::{
        AtomicU32,
        Ordering::SeqCst,
    },
    time::Duration,
};
use ::litebox::{
    platform,
    platform::{
        ImmediatelyWokenUp,
        RawMutexProvider,
        UnblockedOrTimedOut,
    },
};
use ::posix::nvx::{
    pm::{
        ConditionAddress,
        MutexAddress,
    },
    sys::{
        error::Error,
        kcall::pm::{
            lock_mutex,
            signal_cond,
            unlock_mutex,
            wait_cond,
        },
    },
};

//==================================================================================================
// Raw Mutex Inner
//==================================================================================================

/// A wrapper structure for the inner state of a raw mutex. Used for pinning.
#[derive(Default)]
struct RawMutexInnerWrapper {
    mutex: usize,
    cond: usize,
}

/// A structure representing the inner state of a raw mutex.
struct RawMutexInner {
    inner: Pin<Box<RawMutexInnerWrapper>>,
}

impl RawMutexInner {
    /// Locks the target raw mutex.
    fn lock(&self) -> Result<(), Error> {
        lock_mutex(MutexAddress::from(&self.inner.mutex as *const usize as usize), None)
    }

    /// Unlocks the target raw mutex.
    fn unlock(&self) -> Result<(), Error> {
        unlock_mutex(MutexAddress::from(&self.inner.mutex as *const usize as usize))
    }

    /// Waits for a conditional on the target raw mutex.
    fn wait(&self) -> Result<(), Error> {
        wait_cond(
            ConditionAddress::from(&self.inner.cond as *const usize as usize),
            MutexAddress::from(&self.inner.mutex as *const usize as usize),
            None,
        )
    }

    /// Wakes up all threads waiting for a conditional on the target raw mutex.
    fn wakeup_all(&self) -> Result<usize, Error> {
        signal_cond(ConditionAddress::from(&self.inner.cond as *const usize as usize), true)
    }
}

impl Default for RawMutexInner {
    fn default() -> Self {
        Self {
            inner: Box::pin(RawMutexInnerWrapper::default()),
        }
    }
}

//==================================================================================================
// Raw Mutex
//==================================================================================================

/// A raw mutex in user land.
pub struct RawMutex {
    inner: RawMutexInner,
    value: AtomicU32,
}

impl RawMutexProvider for NanvixUserland {
    type RawMutex = RawMutex;

    ///
    /// # Description
    ///
    /// Allocates a new raw mutex.
    ///
    /// # Returns
    ///
    /// A new raw mutex.
    ///
    fn new_raw_mutex(&self) -> Self::RawMutex {
        let inner: RawMutexInner = RawMutexInner::default();
        let value: AtomicU32 = AtomicU32::new(0);

        RawMutex { inner, value }
    }
}

impl platform::RawMutex for RawMutex {
    ///
    /// # Description
    ///
    /// Returns the underlying atomic value of the mutex.
    ///
    /// # Returns
    ///
    /// The underlying atomic value of the mutex.
    ///
    fn underlying_atomic(&self) -> &AtomicU32 {
        &self.value
    }

    ///
    /// # Description
    ///
    /// Wakes up `n` threads waiting for a condition on the mutex.
    ///
    /// # Parameters
    ///
    /// - `n`: The number of threads to wake up.
    ///
    /// # Returns
    ///
    /// The actual number of threads that were woken up.
    ///
    fn wake_many(&self, _n: usize) -> usize {
        match self.inner.wakeup_all() {
            Ok(n) => n,
            Err(error) => {
                panic!("wakeup_all(): failed to wakeup threads (error={:?})", error);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Blocks the current thread until the target raw mutex has a value different than `val`.
    ///
    /// # Parameters
    ///
    /// - `val`: The value to wait for.
    ///
    /// # Returns
    ///
    /// If the target raw mutex has a value different than `val`, returns empty. Otherwise, returns
    /// an error.
    ///
    fn block(&self, val: u32) -> Result<(), ImmediatelyWokenUp> {
        if self.value.load(SeqCst) != val {
            return Err(ImmediatelyWokenUp);
        }

        if let Err(error) = self.inner.lock() {
            panic!("block(): failed to lock mutex (error={:?})", error);
        }

        loop {
            if self.value.load(SeqCst) != val {
                break;
            }

            if let Err(error) = self.inner.wait() {
                panic!("block(): failed to wait on condition variable (error={:?})", error);
            }
        }

        if let Err(error) = self.inner.unlock() {
            panic!("block(): failed to unlock mutex (error={:?})", error);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Blocks the current thread until the target raw mutex has a value different than `val` or
    /// until the timeout expires.
    ///
    /// # Parameters
    ///
    /// - `val`: The value to wait for.
    /// - `timeout`: The timeout to wait for.
    ///
    /// # Returns
    ///
    /// If the target raw mutex has a value different than `val`, returns empty. Otherwise, returns
    /// an error.
    ///
    ///
    fn block_or_timeout(
        &self,
        val: u32,
        _timeout: Duration,
    ) -> Result<UnblockedOrTimedOut, ImmediatelyWokenUp> {
        // TODO: implement timeout semantics.
        match self.block(val) {
            Ok(()) => Ok(UnblockedOrTimedOut::Unblocked),
            Err(ImmediatelyWokenUp) => Err(ImmediatelyWokenUp),
        }
    }
}
