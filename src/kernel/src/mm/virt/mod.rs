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
mod page_table_allocator;
mod vmem;

#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
use identity_map::init as identity_map_init;
#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
pub(crate) use identity_map::memcpy;
#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
use identity_map::memset;

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
    #[cfg_attr(
        feature = "platform-root-virtual-address-space-bootstrap",
        allow(dead_code)
    )]
    Bss(&'static mut [PteWord; PAGE_TABLE_LENGTH]),
    /// Runtime storage backed by a kernel page from the page pool.
    KernelPage(KernelPage),
    /// Host-built page table inherited from a pre-existing address space.
    #[cfg_attr(
        not(feature = "platform-root-virtual-address-space-bootstrap"),
        allow(dead_code)
    )]
    Inherited(*mut PteWord),
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
            Self::Inherited(ptr) => unsafe { core::slice::from_raw_parts(*ptr, PAGE_TABLE_LENGTH) },
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
            Self::Inherited(ptr) => unsafe {
                core::slice::from_raw_parts_mut(*ptr, PAGE_TABLE_LENGTH)
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
