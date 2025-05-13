// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    AtomicUsize,
    Ordering,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A synchronization primitive that allows a thread to wait for a number of signals to be delivered.
///
pub struct Fence {
    /// Number of signals received.
    count: AtomicUsize,
    /// Total number of signals to wait for.
    total: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Fence {
    ///
    /// # Description
    ///
    /// Instantiates a new fence.
    ///
    /// # Parameters
    ///
    /// - `total`: Total number of signals to wait for.
    ///
    pub const fn new(total: usize) -> Self {
        Self {
            count: AtomicUsize::new(0),
            total,
        }
    }

    ///
    /// # Description
    ///
    /// Waits for all signals to be received in a spin loop.
    ///
    pub fn wait(&self) {
        while self.count.load(Ordering::Acquire) < self.total {
            ::arch::cpu::pause();
        }
    }

    ///
    /// # Description
    ///
    /// Signals the fence.
    ///
    pub fn signal(&self) {
        self.count.fetch_add(1, Ordering::Release);
    }
}
