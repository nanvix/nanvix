// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::fmt::Debug;

//==================================================================================================
// Structures
//==================================================================================================

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualEnvironmentIdentifier {
    id: u32,
}
::static_assert::assert_eq_size!(VirtualEnvironmentIdentifier, 4);

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualEnvironmentIdentifier {
    pub const NEW: VirtualEnvironmentIdentifier = Self { id: 0 };

    fn new(id: u32) -> Self {
        Self { id }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn next(&self) -> Self {
        Self::new(self.id + 1)
    }
}

impl From<u32> for VirtualEnvironmentIdentifier {
    fn from(id: u32) -> Self {
        Self::new(id)
    }
}

impl Debug for VirtualEnvironmentIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{{ id: {:?} }}", self.id)
    }
}

impl Default for VirtualEnvironmentIdentifier {
    fn default() -> Self {
        Self { id: 1 }
    }
}
