// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::io,
    hal::io::{
        pmio::info::IoPortInfo,
        IoPortType,
    },
};

//==================================================================================================
// Structures
//==================================================================================================
///
/// **Description**
/// An I/O port.
///
pub(super) struct IoPort {
    info: IoPortInfo,
    pub(super) typ: IoPortType,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl IoPort {
    ///
    /// **Description**
    /// Instantiates a read-only I/O port.
    ///
    /// **Parameters**
    /// - `info`: Information about the I/O port.
    ///
    /// **Returns**
    /// A read-only I/O port.
    ///
    pub(super) fn new_read_only(info: IoPortInfo) -> Self {
        Self {
            info,
            typ: IoPortType::ReadOnly,
        }
    }

    ///
    /// **Description**
    /// Instantiates a write-only I/O port.
    ///
    /// **Parameters**
    /// - `info`: Information about the I/O port.
    ///
    /// **Returns**
    /// A write-only I/O port.
    ///
    pub(super) fn new_write_only(info: IoPortInfo) -> Self {
        Self {
            info,
            typ: IoPortType::WriteOnly,
        }
    }

    ///
    /// **Description**
    /// Instantiates a read-write I/O port.
    ///
    /// **Parameters**
    /// - `info`: Information about the I/O port.
    ///
    /// **Returns**
    /// A read-write I/O port.
    ///
    pub(super) fn new_read_write(info: IoPortInfo) -> Self {
        Self {
            info,
            typ: IoPortType::ReadWrite,
        }
    }

    ///
    /// **Description**
    /// Gets the number of the target I/O port.
    ///
    /// **Returns**
    /// The number of the target I/O port.
    ///
    pub(super) fn number(&self) -> u16 {
        self.info.number()
    }

    ///
    /// **Description**
    /// Reads a byte from the target I/O port.
    ///
    /// **Returns**
    /// The byte read from the target I/O port.
    ///
    pub(super) fn read8(&self) -> u8 {
        unsafe { io::in8(self.info.number()) }
    }

    ///
    /// **Description**
    /// Reads a word from the target I/O port.
    ///
    /// **Returns**
    /// The word read from the target I/O port.
    ///
    pub(super) fn read16(&self) -> u16 {
        unsafe { io::in16(self.info.number()) }
    }

    ///
    /// **Description**
    /// Reads a double word from the target I/O port.
    ///
    /// **Returns**
    /// The double word read from the target I/O port.
    ///
    pub(super) fn read32(&self) -> u32 {
        unsafe { io::in32(self.info.number()) }
    }

    ///
    /// **Description**
    /// Writes a byte to the target I/O port.
    ///
    /// **Parameters**
    /// - `b`: Byte to write.
    ///
    pub(super) fn write8(&self, b: u8) {
        unsafe { io::out8(self.info.number(), b) }
    }

    ///
    /// **Description**
    /// Writes a word to the target I/O port.
    ///
    /// **Parameters**
    /// - `w`: Word to write.
    ///
    pub(super) fn write16(&self, w: u16) {
        unsafe { io::out16(self.info.number(), w) }
    }

    ///
    /// **Description**
    /// Writes a double word to the target I/O port.
    ///
    /// **Parameters**
    /// - `dw`: Double word to write.
    ///
    pub(super) fn write32(&self, dw: u32) {
        unsafe { io::out32(self.info.number(), dw) }
    }
}
