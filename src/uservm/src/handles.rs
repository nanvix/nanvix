// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    VirtualMemory,
    guest::Guest,
};
use ::std::sync::Arc;
use ::tokio::sync::{
    Mutex,
    MutexGuard,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Shared handles for guest and virtual memory manager.
///
#[derive(Clone)]
pub struct UserVmHandles {
    /// Handle for guest, set after initialization.
    guest_handle: Arc<Mutex<Option<Arc<Mutex<Guest>>>>>,
    /// Handle for virtual memory manager, set after initialization.
    vmem_handle: Arc<Mutex<Option<Arc<Mutex<VirtualMemory>>>>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Default for UserVmHandles {
    fn default() -> Self {
        Self::new()
    }
}

impl UserVmHandles {
    ///
    /// # Description
    ///
    /// Creates a new set of handles with all values initialized to None.
    ///
    pub fn new() -> Self {
        Self {
            guest_handle: Arc::new(Mutex::new(None)),
            vmem_handle: Arc::new(Mutex::new(None)),
        }
    }

    ///
    /// # Description
    ///
    /// Sets the guest handle.
    ///
    /// # Parameters
    ///
    /// - `guest`: Guest handle to be set.
    ///
    pub async fn set_guest_handle(&self, guest: Arc<Mutex<Guest>>) {
        let mut guard: MutexGuard<'_, Option<Arc<Mutex<Guest>>>> = self.guest_handle.lock().await;
        *guard = Some(guest);
    }

    ///
    /// # Description
    ///
    /// Sets the virtual memory manager handle.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory manager handle to be set.
    ///
    pub async fn set_vmem_handle(&self, vmem: Arc<Mutex<VirtualMemory>>) {
        let mut guard: MutexGuard<'_, Option<Arc<Mutex<VirtualMemory>>>> =
            self.vmem_handle.lock().await;
        *guard = Some(vmem);
    }

    ///
    /// # Description
    ///
    /// Returns a clone of the guest handle if it has been set.
    ///
    pub fn get_guest_handle(&self) -> Option<Arc<Mutex<Guest>>> {
        let guard: MutexGuard<'_, Option<Arc<Mutex<Guest>>>> = self.guest_handle.blocking_lock();
        guard.clone()
    }

    ///
    /// # Description
    ///
    /// Returns a clone of the virtual memory manager handle if it has been set.
    ///
    pub fn get_vmem_handle(&self) -> Option<Arc<Mutex<VirtualMemory>>> {
        let guard: MutexGuard<'_, Option<Arc<Mutex<VirtualMemory>>>> =
            self.vmem_handle.blocking_lock();
        guard.clone()
    }
}
