// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::io::{
    IoPortAllocator,
    ReadWriteIoPort,
};
use ::arch::cpu::pit;
use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Frequency
//==================================================================================================

///
/// # Description
///
/// The frequency of the PIT device. The default value is overwritten after the PIT is initialized.
static TIMER_FREQUENCY: AtomicU32 = AtomicU32::new(pit::PIT_MAX_FREQUENCY);

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

        // Check if frequency is valid.
        if !(pit::PIT_MIN_FREQUENCY..=pit::PIT_MAX_FREQUENCY).contains(&freq) {
            error!("new(): invalid frequency for pit (freq={})", freq);
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid frequency"));
        }

        // Calculate reload value rounding it to the nearest integer.
        let freq_divisor: u32 = if pit::PIT_MAX_FREQUENCY % freq > pit::PIT_MAX_FREQUENCY / 2 {
            pit::PIT_MAX_FREQUENCY / freq + 1
        } else {
            pit::PIT_MAX_FREQUENCY / freq
        };

        // Compute actual frequency based on frequency divisor.
        let actual_freq: u32 = if pit::PIT_MAX_FREQUENCY % freq_divisor > pit::PIT_MAX_FREQUENCY / 2
        {
            pit::PIT_MAX_FREQUENCY / freq_divisor + 1
        } else {
            pit::PIT_MAX_FREQUENCY / freq_divisor
        };

        // Allocate I/O ports.
        let ctrl: ReadWriteIoPort = ioports.allocate_read_write(pit::PIT_CTRL)?;
        let data: ReadWriteIoPort = ioports.allocate_read_write(pit::PIT_DATA)?;

        let mut pit: Pit = Self { ctrl, data };

        // Reset the PIT.
        pit.ctrl
            .write8(pit::PIT_SEL0 | pit::PIT_ACC_LOHI | pit::PIT_MODE_WAVE | pit::PIT_BINARY);

        // Send data byte: divisor low and high bytes.
        pit.data.write8((freq_divisor & 0xff) as u8);
        pit.data.write8(((freq_divisor >> 8) & 0xff) as u8);

        // Set timer frequency.
        TIMER_FREQUENCY.store(actual_freq, Ordering::SeqCst);

        info!("pit set to {} Hz", actual_freq);

        Ok(pit)
    }
}

///
/// # Description
///
/// Returns the frequency of the timer.
///
/// # Returns
///
/// The frequency of the timer in Hz.
///
pub fn get_timer_frequency() -> u32 {
    TIMER_FREQUENCY.load(Ordering::SeqCst)
}
