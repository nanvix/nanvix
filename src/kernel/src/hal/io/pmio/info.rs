// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that describes information about an I/O port.
///
pub struct IoPortInfo {
    number: u16,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl IoPortInfo {
    ///
    /// # Description
    ///
    /// Instantiates an I/O port information structure.
    ///
    /// # Parameters
    ///
    /// - `number`: Number of the I/O port.
    ///
    /// # Returns
    ///
    /// The I/O port information structure.
    ///
    pub fn new(number: u16) -> Self {
        Self { number }
    }

    ///
    /// # Description
    ///
    /// Gets the number of the I/O port.
    ///
    /// # Returns
    ///
    /// The number of the I/O port.
    ///
    pub fn number(&self) -> u16 {
        self.number
    }
}
