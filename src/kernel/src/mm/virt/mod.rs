// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod boot_init;
mod identity_map;
mod kpage;
mod manager;
mod page_table_allocator;
mod vmem;

use identity_map::init as identity_map_init;
pub(in crate::mm) use identity_map::memset;
pub(crate) use identity_map::{
    identity_map_page,
    memcpy,
    sync_kernel_pdes,
};

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::{
    paging::PteWord,
    PAGE_TABLE_LENGTH,
};
use ::core::ops::{
    Deref,
    DerefMut,
};
use ::vstd::prelude::*;

//==================================================================================================
// Exports
//==================================================================================================

pub use boot_init::init;
pub use kpage::KernelPage;
pub use manager::VirtMemoryManager;
pub use vmem::Vmem;

//==================================================================================================
// Structures and Enums
//==================================================================================================

#[verus_verify]
pub enum PageTableStorage {
    /// Boot-time BSS-backed storage, allocated via `PAGE_TABLE_ALLOCATOR`.
    Bss {
        /// Page-table entries.
        entries: &'static mut [PteWord; PAGE_TABLE_LENGTH],
        /// Virtual base address used by Nanvix to access `entries`.
        #[cfg(verus_keep_ghost_body)]
        entries_base_address: Ghost<usize>,
        /// Physical base address encoded in a parent page-directory entry.
        #[cfg(verus_keep_ghost_body)]
        physical_base_address: Ghost<usize>,
    },
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
}

verus! {
    pub trait GetPageTableStorage {
        spec fn get_storage(&self) -> &PageTableStorage;
    }

    impl GetPageTableStorage for PageTableStorage {
        open spec fn get_storage(&self) -> &PageTableStorage {
            self
        }
    }
}

impl Deref for PageTableStorage {
    type Target = [PteWord];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss { entries, .. } => entries.as_slice(),
            Self::KernelPage(page) => {
                let base: *const PteWord = page.base().into_raw_value() as *const PteWord;
                unsafe { core::slice::from_raw_parts(base, PAGE_TABLE_LENGTH) }
            },
        }
    }
}

impl DerefMut for PageTableStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Bss { entries, .. } => entries.as_mut_slice(),
            Self::KernelPage(page) => {
                let base: *mut PteWord = page.base().into_raw_value() as *mut PteWord;
                unsafe { core::slice::from_raw_parts_mut(base, PAGE_TABLE_LENGTH) }
            },
        }
    }
}

#[verus_verify]
pub enum PageDirectoryStorage {
    /// Boot-time BSS-backed storage, allocated via `PAGE_TABLE_ALLOCATOR`.
    Bss {
        /// Page-directory entries.
        entries: &'static mut [PteWord; PAGE_TABLE_LENGTH],
        /// Base address of `entries`.
        #[cfg(verus_keep_ghost_body)]
        base_address: Ghost<usize>,
    },
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
}

verus! {
    pub trait GetPageDirectoryStorage {
        spec fn get_storage(&self) -> &PageDirectoryStorage;
    }

    impl GetPageDirectoryStorage for PageDirectoryStorage {
        open spec fn get_storage(&self) -> &PageDirectoryStorage {
            self
        }
    }
}

impl Deref for PageDirectoryStorage {
    type Target = [PteWord];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss { entries, .. } => entries.as_slice(),
            Self::KernelPage(page) => {
                let base: *const PteWord = page.base().into_raw_value() as *const PteWord;
                unsafe { core::slice::from_raw_parts(base, PAGE_TABLE_LENGTH) }
            },
        }
    }
}

impl DerefMut for PageDirectoryStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Bss { entries, .. } => entries.as_mut_slice(),
            Self::KernelPage(page) => {
                let base: *mut PteWord = page.base().into_raw_value() as *mut PteWord;
                unsafe { core::slice::from_raw_parts_mut(base, PAGE_TABLE_LENGTH) }
            },
        }
    }
}

include!("mod.spec.rs");

#[cfg(feature = "test")]
pub fn test() {
    assert!(identity_map::test::test());
}
