// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::kcall::Capability;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone, Copy)]
pub struct Capabilities(u8);

//==================================================================================================
// Implementations
//==================================================================================================

impl Capabilities {
    pub fn set(&mut self, capability: Capability) {
        self.0 |= 1 << capability as u8;
    }

    pub fn clear(&mut self, capability: Capability) {
        self.0 &= !(1 << capability as u8);
    }

    pub fn has(&self, capability: Capability) -> bool {
        (self.0 & (1 << capability as u8)) != 0
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self(0)
    }
}
