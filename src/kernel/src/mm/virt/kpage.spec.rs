// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

verus! {

impl KernelPage {
    pub closed spec fn base_address(&self) -> int {
        self.kframe.base_address()
    }

    pub closed spec fn entries_base_address(&self) -> int {
        self.kframe.base_address()
    }

    pub closed spec fn physical_base_address(&self) -> int {
        self.kframe.base_address()
    }
}

} // verus!
