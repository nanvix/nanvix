// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    VirtualMemory,
    guest::Guest,
};
use ::anyhow::Result;
use ::std::sync::{
    Arc,
    Mutex as StdMutex,
    OnceLock,
    Weak,
};
use ::syslog::error;
use ::tokio::sync::Mutex;

// ==================================================================================================
// Globals
// ==================================================================================================

/// Global registry holding references to the guest and virtual memory manager.
static GLOBAL_HANDLES: OnceLock<GlobalHandles> = OnceLock::new();

//==================================================================================================
// Structure
//==================================================================================================

pub(super) struct GlobalHandles {
    inner: StdMutex<Handles>,
}

#[derive(Default)]
struct Handles {
    guest: Option<Weak<Mutex<Guest>>>,
    vmem: Option<Weak<Mutex<VirtualMemory>>>,
}

pub(super) struct GlobalRegistration {
    handles: &'static GlobalHandles,
    guest: Weak<Mutex<Guest>>,
    vmem: Weak<Mutex<VirtualMemory>>,
}

fn global_handles() -> &'static GlobalHandles {
    GLOBAL_HANDLES.get_or_init(|| GlobalHandles {
        inner: StdMutex::new(Handles::default()),
    })
}

impl GlobalHandles {
    pub(super) fn register(
        &self,
        guest: &Arc<Mutex<Guest>>,
        vmem: &Arc<Mutex<VirtualMemory>>,
    ) -> Result<()> {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(error) => {
                let reason: String = format!("failed to acquire global handles lock: {error}");
                error!("register(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        if inner.guest.is_some() || inner.vmem.is_some() {
            let reason: &str = "global handles already registered";
            error!("register(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        inner.guest = Some(Arc::downgrade(guest));
        inner.vmem = Some(Arc::downgrade(vmem));
        Ok(())
    }

    fn clear_if_matches(
        &self,
        expected_guest: &Weak<Mutex<Guest>>,
        expected_vmem: &Weak<Mutex<VirtualMemory>>,
    ) {
        let mut inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(error) => {
                error!("clear_if_matches(): failed to acquire global handles lock: {error}");
                return;
            },
        };

        let guest_matches: bool = inner
            .guest
            .as_ref()
            .map(|guest| guest.ptr_eq(expected_guest))
            .unwrap_or(false);
        let vmem_matches: bool = inner
            .vmem
            .as_ref()
            .map(|vmem| vmem.ptr_eq(expected_vmem))
            .unwrap_or(false);

        if guest_matches && vmem_matches {
            inner.guest = None;
            inner.vmem = None;
        }
    }

    fn upgrade_guest(&self) -> Option<Arc<Mutex<Guest>>> {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(error) => {
                error!("upgrade_guest(): failed to acquire global handles lock: {error}");
                return None;
            },
        };
        inner.guest.as_ref().and_then(|guest| guest.upgrade())
    }

    fn upgrade_vmem(&self) -> Option<Arc<Mutex<VirtualMemory>>> {
        let inner = match self.inner.lock() {
            Ok(guard) => guard,
            Err(error) => {
                error!("upgrade_vmem(): failed to acquire global handles lock: {error}");
                return None;
            },
        };
        inner.vmem.as_ref().and_then(|vmem| vmem.upgrade())
    }
}

impl GlobalRegistration {
    pub(super) fn register(
        guest: Arc<Mutex<Guest>>,
        vmem: Arc<Mutex<VirtualMemory>>,
    ) -> Result<Arc<GlobalRegistration>> {
        let handles: &'static GlobalHandles = global_handles();
        handles.register(&guest, &vmem)?;

        Ok(Arc::new(GlobalRegistration {
            handles,
            guest: Arc::downgrade(&guest),
            vmem: Arc::downgrade(&vmem),
        }))
    }
}

impl Drop for GlobalRegistration {
    fn drop(&mut self) {
        self.handles.clear_if_matches(&self.guest, &self.vmem);
    }
}

pub(crate) fn try_get_guest_handle() -> Option<Arc<Mutex<Guest>>> {
    global_handles().upgrade_guest()
}

pub(crate) fn try_get_vmem_handle() -> Option<Arc<Mutex<VirtualMemory>>> {
    global_handles().upgrade_vmem()
}
