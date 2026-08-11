// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

verus! {

impl KernelFrame {
    pub open spec fn base_address(&self) -> int {
        self.base@
    }
}

} // verus!
