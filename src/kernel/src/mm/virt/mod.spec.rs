// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

verus! {

impl PageTableStorage {
    pub open spec fn entries_base_address(&self) -> int {
        match self {
            Self::Bss {
                entries_base_address,
                ..
            } => entries_base_address@ as int,
            Self::KernelPage(page) => page.entries_base_address(),
        }
    }

    pub open spec fn physical_base_address(&self) -> int {
        match self {
            Self::Bss {
                physical_base_address,
                ..
            } => physical_base_address@ as int,
            Self::KernelPage(page) => page.physical_base_address(),
        }
    }
}

impl PageDirectoryStorage {
    pub open spec fn base_address(&self) -> int {
        match self {
            Self::Bss { base_address, .. } => base_address@ as int,
            Self::KernelPage(page) => page.base_address(),
        }
    }
}

} // verus!
