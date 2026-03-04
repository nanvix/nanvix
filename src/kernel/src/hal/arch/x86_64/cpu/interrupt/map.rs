// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86_64::cpu::InterruptNumber;
use core::ops::Index;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a map of logical interrupt numbers to physical interrupt pins.
///
#[derive(Debug)]
pub struct InterruptMap([u8; Self::LENGTH]);

//==================================================================================================
// Implementations
//==================================================================================================

impl InterruptMap {
    /// Maximum length of the map.
    const LENGTH: usize = 256;

    ///
    /// # Description
    ///
    /// Instantiates an interrupt map with identity mapping.
    ///
    pub fn new() -> Self {
        let mut map = [0; Self::LENGTH];
        for (i, item) in map.iter_mut().enumerate() {
            *item = i as u8;
        }
        Self(map)
    }

    ///
    /// # Description
    ///
    /// Remaps an interrupt number.
    ///
    pub fn remap(&mut self, logical: u8, physical: u8) {
        self.0[logical as usize] = physical;
    }
}

impl Index<InterruptNumber> for InterruptMap {
    type Output = u8;

    fn index(&self, index: InterruptNumber) -> &Self::Output {
        &self.0[index as usize]
    }
}
