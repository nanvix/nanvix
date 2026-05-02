// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
mod boot_init;
#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
mod identity_map;
mod kpage;
mod manager;
#[cfg(feature = "platform-root-virtual-address-space-bootstrap")]
mod no_identity_map;
mod page_table_allocator;
#[cfg(feature = "platform-root-virtual-address-space-bootstrap")]
pub(in crate::mm) use page_table_allocator::PAGE_TABLE_ALLOCATOR;
mod vmem;

#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
use identity_map::init as identity_map_init;
#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
pub(crate) use identity_map::memcpy;
#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
use identity_map::memset;
#[cfg(feature = "platform-root-virtual-address-space-bootstrap")]
pub(crate) use no_identity_map::memcpy;
#[cfg(feature = "platform-root-virtual-address-space-bootstrap")]
use no_identity_map::memset;

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

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
pub use boot_init::init;
pub use kpage::KernelPage;
pub use manager::VirtMemoryManager;
pub use vmem::Vmem;

//==================================================================================================
// Structures and Enums
//==================================================================================================

pub enum PageTableStorage {
    /// Boot-time BSS-backed storage, allocated via `PAGE_TABLE_ALLOCATOR`.
    Bss(&'static mut [PteWord; PAGE_TABLE_LENGTH]),
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
}

impl Deref for PageTableStorage {
    type Target = [PteWord];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss(entries) => entries.as_slice(),
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
            Self::Bss(entries) => entries.as_mut_slice(),
            Self::KernelPage(page) => {
                let base: *mut PteWord = page.base().into_raw_value() as *mut PteWord;
                unsafe { core::slice::from_raw_parts_mut(base, PAGE_TABLE_LENGTH) }
            },
        }
    }
}

pub enum PageDirectoryStorage {
    /// Boot-time BSS-backed storage, allocated via `PAGE_TABLE_ALLOCATOR`.
    Bss(&'static mut [PteWord; PAGE_TABLE_LENGTH]),
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
}

impl Deref for PageDirectoryStorage {
    type Target = [PteWord];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Bss(entries) => entries.as_slice(),
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
            Self::Bss(entries) => entries.as_mut_slice(),
            Self::KernelPage(page) => {
                let base: *mut PteWord = page.base().into_raw_value() as *mut PteWord;
                unsafe { core::slice::from_raw_parts_mut(base, PAGE_TABLE_LENGTH) }
            },
        }
    }
}
