// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::io::{
    IoPortAllocator,
    ReadWriteIoPort,
};
use ::sys::error::Error;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// CMOS memory
///
/// # References
///
///- https://www.stanislavs.org/helppc/cmos_ram.html
///
pub struct Cmos {
    /// CMOS data register.
    data: ReadWriteIoPort,
    /// CMOS index register.
    index: ReadWriteIoPort,
}

///
/// # Description
///
/// Values for the shutdown status byte.
///
#[allow(dead_code)]
pub enum ShutdownStatus {
    SoftReset = 0,
    MemorySizeDetermination = 1,
    MemoryTest = 2,
    MemoryError = 3,
    BootLoaderRequest = 4,
    JmpDwordRequestWithIntInit = 5,
    ProtectedModeTest7Passed = 6,
    ProtectedModeTest7Failed = 7,
    ProtectedModeTest1Failed = 8,
    BlockMoveShutdownRequest = 9,
    JmpDwordRequestWithoutIntInit = 10,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Cmos {
    /// I/O port for the CMOS data register.
    pub const DATA: u16 = 0x70;
    /// I/O port for the CMOS index register.
    pub const INDEX: u16 = 0x71;

    /// Shutdown Status byte
    const SHUTDOWN_STATUS: u8 = 0xF;

    ///
    /// # Description
    ///
    /// Initializes the CMOS.
    ///
    pub fn init(ioports: &mut IoPortAllocator) -> Result<Self, Error> {
        Ok(Self {
            data: ioports.allocate_read_write(Self::DATA)?,
            index: ioports.allocate_read_write(Self::INDEX)?,
        })
    }

    ///
    /// # Description
    ///
    /// Writes to the shutdown status byte in the CMOS.
    ///
    /// # Parameters
    ///
    /// - `status`: New value for the shutdown status byte.
    ///
    pub fn write_shutdown_status(&mut self, status: ShutdownStatus) {
        self.write(Self::SHUTDOWN_STATUS, status as u8);
    }

    ///
    /// # Description
    ///
    /// Writes a byte to the CMOS.
    ///
    /// # Parameters
    ///
    /// - `register`: Register index.
    /// - `data`: Data to write.
    ///
    fn write(&mut self, register: u8, data: u8) {
        if (data & (1 << 7)) != 0 {
            // FIXME: The 8th bit of the data register sets/clears NMI and we should handle that.
            unimplemented!("write(): nmi disable not implemented");
        }
        self.index.write8(register);
        self.data.write8(data);
    }
}
