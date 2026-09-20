// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
verus! {

impl AccessPermission {
    /// Concrete write-bit interpretation used by the virtual-memory contract adapter.
    pub closed spec fn spec_writable(&self) -> bool {
        self.write == WritePermission::Allow
    }
}

} // verus!
