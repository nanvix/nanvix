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
use ::sys::error::Error;
#[cfg(not(all(feature = "microvm", feature = "whp")))]
use ::sys::error::ErrorCode;

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
/// On non-WHP platforms the PIT operates as a periodic timer using channel 0. On WHP the PIT is
/// used exclusively as a calibration source for the LAPIC timer via channel 2.
///
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
pub struct Pit {
    /// PIT command register (port 0x43).
    ctrl: ReadWriteIoPort,
    /// Channel 0 data port (port 0x40). Used for periodic timer on non-WHP platforms.
    #[cfg(not(all(feature = "microvm", feature = "whp")))]
    data: ReadWriteIoPort,
    /// Channel 2 data port (port 0x42). Used for LAPIC timer calibration on WHP.
    #[cfg(all(feature = "microvm", feature = "whp"))]
    data_ch2: ReadWriteIoPort,
    /// Speaker gate port (port 0x61). Used for LAPIC timer calibration on WHP.
    #[cfg(all(feature = "microvm", feature = "whp"))]
    speaker_gate: ReadWriteIoPort,
}

#[cfg(not(all(feature = "microvm", feature = "whp")))]
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
impl Pit {
    ///
    /// # Description
    ///
    /// Initializes the PIT device as a periodic timer on channel 0.
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
            error!("invalid frequency for pit (freq={})", freq);
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

#[cfg(all(feature = "microvm", feature = "whp"))]
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
impl Pit {
    ///
    /// # Description
    ///
    /// Initializes the PIT device for LAPIC timer calibration using channel 2.
    ///
    /// Channel 0 is not programmed. The PIT is used only as a reference clock source to measure
    /// LAPIC timer ticks. The timer frequency is set from the requested value so that other
    /// kernel subsystems report the correct tick rate.
    ///
    /// # Parameters
    ///
    /// - `ioports`: I/O port allocator.
    /// - `freq`: Timer frequency in Hz (used by the LAPIC periodic timer, not the PIT).
    ///
    /// # Returns
    ///
    /// If successful, returns a `Pit` instance. Otherwise, returns an error.
    ///
    pub fn new(ioports: &mut IoPortAllocator, freq: u32) -> Result<Self, Error> {
        info!("initializing pit for lapic calibration...");

        // Validate frequency to prevent divide-by-zero in downstream time conversions.
        if freq == 0 {
            error!("invalid frequency for pit (freq=0)");
            return Err(Error::new(::sys::error::ErrorCode::InvalidArgument, "invalid frequency"));
        }

        // Allocate I/O ports for calibration.
        let ctrl: ReadWriteIoPort = ioports.allocate_read_write(pit::PIT_CTRL)?;
        let data_ch2: ReadWriteIoPort = ioports.allocate_read_write(pit::PIT_DATA_CH2)?;
        let speaker_gate: ReadWriteIoPort = ioports.allocate_read_write(pit::PIT_SPEAKER_GATE)?;

        // Set timer frequency. On WHP, the LAPIC timer is the actual timer source.
        TIMER_FREQUENCY.store(freq, Ordering::SeqCst);

        info!("pit calibration ports allocated (timer freq set to {} Hz)", freq);

        Ok(Self {
            ctrl,
            data_ch2,
            speaker_gate,
        })
    }

    /// Maximum supported one-shot duration in milliseconds (~54ms for 1.193 MHz PIT).
    pub const MAX_ONESHOT_MS: u32 = (0xFFFF_u64 * 1000 / pit::PIT_MAX_FREQUENCY as u64) as u32;

    ///
    /// # Description
    ///
    /// Arms PIT channel 2 in one-shot mode for the given duration. The countdown starts as soon
    /// as the reload value is written. Call [`Self::wait_oneshot()`] to block until the delay
    /// elapses.
    ///
    /// Durations of 0 ms are clamped to 1 ms (a PIT reload of 0 is interpreted as 65536).
    /// Durations exceeding [`Self::MAX_ONESHOT_MS`] (~54 ms) are clamped to that maximum.
    ///
    /// # Parameters
    ///
    /// - `ms`: Duration in milliseconds.
    ///
    pub fn arm_oneshot(&mut self, ms: u32) {
        // Clamp to valid PIT one-shot range. A reload of 0 is interpreted by the PIT
        // as 65536 (maximum interval), so treat ms=0 as 1ms to avoid surprising behavior.
        let clamped_ms: u32 = if ms == 0 {
            warn!("pit oneshot duration 0ms invalid, clamping to 1ms");
            1
        } else if ms > Self::MAX_ONESHOT_MS {
            warn!("pit oneshot duration {}ms exceeds max {}ms, clamping", ms, Self::MAX_ONESHOT_MS);
            Self::MAX_ONESHOT_MS
        } else {
            ms
        };
        let pit_reload: u16 =
            ((pit::PIT_MAX_FREQUENCY as u64 * clamped_ms as u64 / 1000) & 0xFFFF) as u16;

        // Enable speaker gate for channel 2 and clear output bit.
        let speaker: u8 = (self.speaker_gate.read8() & pit::PIT_SPEAKER_GATE_CH2_CLEAR)
            | pit::PIT_SPEAKER_GATE_CH2_ENABLE;
        self.speaker_gate.write8(speaker);

        // Channel 2, lobyte/hibyte, mode 0 (one-shot), binary.
        self.ctrl
            .write8(pit::PIT_SEL2 | pit::PIT_ACC_LOHI | pit::PIT_MODE_TCOUNT | pit::PIT_BINARY);

        // Write reload value — countdown starts here.
        self.data_ch2.write8((pit_reload & 0xFF) as u8);
        self.data_ch2.write8((pit_reload >> 8) as u8);
    }

    ///
    /// # Description
    ///
    /// Busy-waits until the PIT channel 2 one-shot completes. The method polls the OUT2 bit on
    /// the speaker gate port and returns when it asserts. A fixed iteration limit prevents an
    /// unbounded spin if the hardware never signals completion.
    ///
    pub fn wait_oneshot(&self) {
        const PIT_CALIBRATION_MAX_ITERS: u32 = 10_000_000;
        let mut pit_iters: u32 = 0;
        while (self.speaker_gate.read8() & pit::PIT_SPEAKER_GATE_OUT2) == 0 {
            core::hint::spin_loop();
            pit_iters = pit_iters.wrapping_add(1);
            if pit_iters >= PIT_CALIBRATION_MAX_ITERS {
                warn!(
                    "pit oneshot timeout: out2 did not assert after {} iterations",
                    PIT_CALIBRATION_MAX_ITERS
                );
                break;
            }
        }
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
