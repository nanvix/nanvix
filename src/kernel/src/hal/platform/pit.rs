// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::io::{
    IoPortAllocator,
    ReadWriteIoPort,
};
use ::sys::{
    arch::cpu::pit,
    error::Error,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents the Programmable Interval Timer (PIT) device.
///
pub struct Pit {
    ctrl: ReadWriteIoPort,
    data: ReadWriteIoPort,
}

impl Pit {
    ///
    /// # Description
    ///
    /// Initializes the PIT device.
    ///
    /// # Parameters
    ///
    /// - `ioports`: I/O port allocator.
    /// - `freq`: Frequency in Hz.
    ///
    /// # Returns
    ///
    /// If successful, returns a `Pit` instance. Otherwise, returns an error.
    ///
    pub fn new(ioports: &mut IoPortAllocator, freq: u32) -> Result<Self, Error> {
        info!("initializing pit...");

        // Allocate I/O ports.
        let ctrl: ReadWriteIoPort = ioports.allocate_read_write(pit::PIT_CTRL)?;
        let data: ReadWriteIoPort = ioports.allocate_read_write(pit::PIT_DATA)?;

        let mut pit: Pit = Self { ctrl, data };

        // Reset the PIT.
        pit.ctrl
            .write8(pit::PIT_SEL0 | pit::PIT_ACC_LOHI | pit::PIT_MODE_WAVE | pit::PIT_BINARY);

        let freq_divisor: u32 = pit::PIT_MAX_FREQUENCY / freq;

        // Send data byte: divisor low and high bytes.
        pit.data.write8((freq_divisor & 0xff) as u8);
        pit.data.write8(((freq_divisor >> 8) & 0xff) as u8);

        // Set timer frequency.
        TIMER_FREQUENCY.store(freq, Ordering::SeqCst);

        Ok(pit)
    }
}

}
