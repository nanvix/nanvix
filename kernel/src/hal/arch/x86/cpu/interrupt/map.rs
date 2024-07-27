// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::cpu::InterruptNumber;
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
    /// Maximum length of the map. This is set to match the x86 I/O APIC specification.
    const LENGTH: usize = 256;

    ///
    /// # Description
    ///
    /// Instantiates an interrupt map with identity mapping.
    ///
    /// # Returns
    ///
    /// An interrupt map.
    ///
    pub fn new() -> Self {
        let mut map = [0; Self::LENGTH];
        for i in 0..Self::LENGTH {
            map[i] = i as u8;
        }
        Self(map)
    }

    ///
    /// # Description
    ///
    /// Remaps an interrupt number.
    ///
    /// # Parameters
    ///
    /// - `logical`: The logical interrupt number.
    /// - `physical`: The physical interrupt pin.
    ///
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
