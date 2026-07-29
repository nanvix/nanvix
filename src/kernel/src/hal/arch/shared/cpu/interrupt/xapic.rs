// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    io::IoMemoryRegion,
    mem::Address,
};
use ::arch::cpu::xapic;
use ::sys::error::{
    Error,
    ErrorCode,
};

#[cfg(feature = "smp")]
#[path = ""]
mod smp_feature_imports {
    pub use crate::mm::kredzone;
    pub use ::sys::mm::VirtualAddress;
}

#[cfg(feature = "smp")]
use smp_feature_imports::*;

//==================================================================================================
// Uninitialized xAPIC
//==================================================================================================

///
/// # Description
///
/// A struct that represents an uninitialized advanced programmable interrupt controller (xAPIC).
///
pub struct UninitXapic {
    /// Local APIC ID.
    id: u8,
    /// Local APIC base address.
    base: IoMemoryRegion,
}

impl UninitXapic {
    ///
    /// # Description
    ///
    /// Instantiates an uninitialized xAPIC.
    ///
    pub fn new(id: u8, base: IoMemoryRegion) -> UninitXapic {
        UninitXapic { id, base }
    }

    ///
    /// # Description
    ///
    /// Initializes the target xAPIC.
    ///
    pub fn init(&mut self) -> Result<Xapic, Error> {
        info!("initializing xapic (id={}, base={:?})", self.id, self.base);

        let mut xapic: Xapic = Xapic {
            id: self.id,
            ptr: xapic::Xapic::new(self.base.base().into_raw_value() as *mut u32),
        };

        // Check ID matches the one in the APIC.
        let apic_id: xapic::XapicId = xapic::XapicId::from_u32(xapic.read(xapic::XAPIC_ID));
        if apic_id.id() != xapic.id as u32 {
            let reason: &str = "id mismatch";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        // Setup spurious interrupt vector.
        let svr: xapic::XapicSvr =
            xapic::XapicSvr::new(xapic::XapicIntvec::Spurious as u32, true, false, false);
        xapic.write(xapic::XAPIC_SVR, svr.to_u32());

        // Clear error status register (requires back-to-back writes).
        let esr: xapic::XapicEsr =
            xapic::XapicEsr::new(false, false, false, false, false, false, false, false);
        xapic.write(xapic::XAPIC_ESR, esr.to_u32());
        xapic.write(xapic::XAPIC_ESR, esr.to_u32());

        // Ack any outstanding interrupts.
        xapic.write(xapic::XAPIC_EOI, 0);

        // Send an Init Level De-Assert to synchronize arbitration ID's.
        let icrhi: xapic::XapicIcrHi = xapic::XapicIcrHi::new(0);
        xapic.write(xapic::XAPIC_ICRHI, icrhi.to_u32());
        let icrlo: xapic::XapicIcrLo =
            xapic::XapicIcrLo::from_u32(0x00080000 | 0x00000500 | 0x00008000);
        xapic.write(xapic::XAPIC_ICRLO, icrlo.to_u32());

        // Poll delivery status until it is set to zero.
        loop {
            let icrlo: xapic::XapicIcrLo =
                xapic::XapicIcrLo::from_u32(xapic.read(xapic::XAPIC_ICRLO));
            if icrlo.delivery_status() as u8 == xapic::XapicIcrDeliveryStatus::Idle as u8 {
                break;
            }
        }

        // Disable timer interrupt.
        let timer: xapic::XapicTimer = xapic::XapicTimer::new(
            xapic::XapicIntvec::Timer as u32,
            false,
            true,
            xapic::XapicIcrDeliveryMode::FixedDelivery as u32,
        );
        xapic.write(xapic::XAPIC_TIMER, timer.to_u32());

        // Read version number.
        let version: xapic::XapicVer = xapic::XapicVer::new(xapic.read(xapic::XAPIC_VER));

        // Check if performance counter register is supported
        if version.max_lvt() >= 4 {
            info!("performance counter interrupt supported");
            // It is, so disable performance counter interrupt.
            let perf: xapic::XapicPcint = xapic::XapicPcint::new(
                xapic::XapicIntvec::Pcint as u32,
                xapic::XapicIcrDeliveryMode::FixedDelivery as u32,
                false,
                true,
            );
            xapic.write(xapic::XAPIC_PCINT, perf.to_u32());
        }

        // Check if thermal register is supported.
        if version.max_lvt() >= 5 {
            info!("thermal interrupt supported");
            // It is, so disable thermal interrupt.
            let thermal: xapic::XapicThermal = xapic::XapicThermal::new(
                xapic::XapicIntvec::Thermal as u32,
                xapic::XapicIcrDeliveryMode::FixedDelivery as u32,
                false,
                true,
            );
            xapic.write(xapic::XAPIC_THERM, thermal.to_u32());
        }

        // Check if CMCI register is supported.
        if version.max_lvt() >= 6 {
            info!("cmci interrupt supported");
            // It is, so disable CMCI interrupt.
            let cmci: xapic::XapicCmci = xapic::XapicCmci::new(
                xapic::XapicIntvec::Cmci as u32,
                xapic::XapicIcrDeliveryMode::FixedDelivery as u32,
                false,
                true,
            );
            xapic.write(xapic::XAPIC_CMCI, cmci.to_u32());
        }

        // Disable local interrupt 0.
        let lint0: xapic::XapicPcint = xapic::XapicPcint::new(
            xapic::XapicIntvec::Lint0 as u32,
            xapic::XapicIcrDeliveryMode::FixedDelivery as u32,
            false,
            false,
        );
        xapic.write(xapic::XAPIC_LINT0, lint0.to_u32());

        // Disable local interrupt 1.
        let lint1: xapic::XapicPcint = xapic::XapicPcint::new(
            xapic::XapicIntvec::Lint1 as u32,
            xapic::XapicIcrDeliveryMode::FixedDelivery as u32,
            false,
            false,
        );
        xapic.write(xapic::XAPIC_LINT1, lint1.to_u32());

        // Disable error interrupt.
        let error: xapic::XapicPcint = xapic::XapicPcint::new(
            xapic::XapicIntvec::Error as u32,
            xapic::XapicIcrDeliveryMode::FixedDelivery as u32,
            false,
            true,
        );
        xapic.write(xapic::XAPIC_ERROR, error.to_u32());

        // Enable interrupts on the APIC (but not on the processor).
        let tpr: xapic::XapicTpr = xapic::XapicTpr::new(0, 0);
        xapic.write(xapic::XAPIC_TPR, tpr.to_u32());

        Ok(xapic)
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the target xAPIC.
    ///
    /// # Returns
    ///
    /// This function returns the base address of the target xAPIC.
    ///
    pub fn base(&self) -> usize {
        self.base.base().into_raw_value()
    }
}

//==================================================================================================
// Initialized xAPIC
//==================================================================================================

///
/// # Description
///
/// A struct that represents an initialized advanced programmable interrupt controller (xAPIC).
///
pub struct Xapic {
    /// Local APIC ID.
    id: u8,
    /// Local APIC pointer.
    ptr: xapic::Xapic,
}

impl Xapic {
    ///
    /// # Description
    ///
    /// Sends an End of Interrupt (EOI) to the target xAPIC.
    ///
    pub fn ack(&mut self) {
        unsafe { self.ptr.write(xapic::XAPIC_EOI, 0) };
    }

    ///
    /// # Description
    ///
    /// Starts up an application core using the "Universal Startup Algorithm".
    ///
    /// # Parameters
    ///
    /// - `coreid`: Core ID.
    /// - `entry`: Entry point.
    /// - `kstack`: Kernel stack.
    ///
    /// # Returns
    ///
    /// Upon success, empty result is returned. Otherwise, an error is returned.
    ///
    #[cfg(feature = "smp")]
    pub fn start_core(
        &mut self,
        coreid: u8,
        entry: VirtualAddress,
        kstack: *const u8,
    ) -> Result<(), Error> {
        use crate::hal::arch::x86::cpu::clock;

        // Maximum number of retries when waiting for the xAPIC to become idle.
        const RETRIES: usize = 1000;

        // Store the address of the kernel stack in the kernel red zone. When the application core
        // starts it will read this address from the kernel red zone. To setup its own stack.
        kredzone::store(0, kstack as usize)?;

        // Send INIT assert  interrupt to reset other core.
        let icrhi: xapic::XapicIcrHi = xapic::XapicIcrHi::new(coreid as u32);
        self.write(xapic::XAPIC_ICRHI, icrhi.to_u32());
        let icrlo: xapic::XapicIcrLo = xapic::XapicIcrLo::new(
            0,
            xapic::XapicIcrDeliveryMode::Init,
            xapic::XapicIcrDestinationMode::Physical,
            xapic::XapicIcrLevel::Assert,
            xapic::XapicIcrTriggerMode::Level,
            xapic::XapicIcrDestinationShorthand::NoShorthand,
        );
        self.write(xapic::XAPIC_ICRLO, icrlo.to_u32());
        self.wait(RETRIES)?;
        clock::microdelay(10000);

        // Send INIT de-assert to reset other core.
        let icrhi: xapic::XapicIcrHi = xapic::XapicIcrHi::new(coreid as u32);
        self.write(xapic::XAPIC_ICRHI, icrhi.to_u32());
        let icrlo: xapic::XapicIcrLo = xapic::XapicIcrLo::new(
            0,
            xapic::XapicIcrDeliveryMode::Init,
            xapic::XapicIcrDestinationMode::Physical,
            xapic::XapicIcrLevel::DeAssert,
            xapic::XapicIcrTriggerMode::Level,
            xapic::XapicIcrDestinationShorthand::NoShorthand,
        );
        self.write(xapic::XAPIC_ICRLO, icrlo.to_u32());
        self.wait(RETRIES)?;
        clock::microdelay(10000);

        // Send SIPI interrupt to reset other core.
        for _ in 0..2 {
            let icrhi: xapic::XapicIcrHi = xapic::XapicIcrHi::new(coreid as u32);
            self.write(xapic::XAPIC_ICRHI, icrhi.to_u32());
            let icrlo: xapic::XapicIcrLo = xapic::XapicIcrLo::new(
                (entry.into_raw_value() >> 12) as u32,
                xapic::XapicIcrDeliveryMode::Startup,
                xapic::XapicIcrDestinationMode::Physical,
                xapic::XapicIcrLevel::DeAssert,
                xapic::XapicIcrTriggerMode::Edge,
                xapic::XapicIcrDestinationShorthand::NoShorthand,
            );

            self.write(xapic::XAPIC_ICRLO, icrlo.to_u32());
            clock::microdelay(200);
            self.wait(RETRIES)?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Performs a safe write on the target xAPIC.
    ///
    /// # Parameters
    ///
    /// - `reg`: Register.
    /// - `value`: Value.
    ///
    fn write(&mut self, reg: u32, value: u32) {
        unsafe { self.ptr.write(reg, value) };
    }

    ///
    /// # Description
    ///
    /// Performs a safe read on the target xAPIC.
    ///
    /// # Parameters
    ///
    /// - `reg`: Register.
    ///
    /// # Return Values
    ///
    /// The value read.
    ///
    fn read(&mut self, reg: u32) -> u32 {
        unsafe { self.ptr.read(reg) }
    }

    ///
    /// # Description
    ///
    /// Polls the target xAPIC until it becomes idle.
    ///
    /// # Params
    ///
    /// - `retries`: Number of retries.
    ///
    /// # Returns
    ///
    /// Upon success, empty result is returned. Otherwise, an error is returned.
    ///
    #[cfg(feature = "smp")]
    fn wait(&mut self, retries: usize) -> Result<(), Error> {
        for _ in 0..retries {
            let bits: u32 = self.read(xapic::XAPIC_ICRLO);
            let icrlo: xapic::XapicIcrLo = xapic::XapicIcrLo::from_u32(bits);

            if icrlo.delivery_status() as u8 == xapic::XapicIcrDeliveryStatus::Idle as u8 {
                return Ok(());
            }

            ::arch::cpu::pause();
        }

        let reason: &str = "maximum number of retries exceeded";
        error!("{reason}");
        Err(Error::new(ErrorCode::TimerExpired, reason))
    }
}

//==================================================================================================
// Uninitialized xAPIC Timer (x86 only)
//==================================================================================================

///
/// # Description
///
/// An uninitialized xAPIC timer. Holds an allocated LAPIC MMIO region and can be initialized
/// into an [`XapicTimer`] via PIT-based calibration, following the same Uninit pattern used
/// by [`UninitPic`] and [`UninitXapic`].
///
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)]
pub struct UninitXapicTimer {
    /// LAPIC MMIO region handle.
    region: IoMemoryRegion,
    /// Low-level LAPIC MMIO handle.
    ptr: xapic::Xapic,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)]
impl UninitXapicTimer {
    ///
    /// # Description
    ///
    /// Creates a new uninitialized xAPIC timer from an allocated MMIO region.
    ///
    /// # Parameters
    ///
    /// - `region`: Allocated LAPIC MMIO region (must be identity-mapped).
    ///
    pub fn new(region: IoMemoryRegion) -> Self {
        let base: usize = region.base().into_raw_value();
        Self {
            region,
            ptr: xapic::Xapic::new(base as *mut u32),
        }
    }

    ///
    /// # Description
    ///
    /// Performs a safe write on the target xAPIC.
    ///
    /// # Parameters
    ///
    /// - `reg`: Register.
    /// - `value`: Value.
    ///
    fn write(&mut self, reg: u32, value: u32) {
        unsafe { self.ptr.write(reg, value) };
    }

    ///
    /// # Description
    ///
    /// Performs a safe read on the target xAPIC.
    ///
    /// # Parameters
    ///
    /// - `reg`: Register.
    ///
    /// # Return Values
    ///
    /// The value read.
    ///
    fn read(&mut self, reg: u32) -> u32 {
        unsafe { self.ptr.read(reg) }
    }

    ///
    /// # Description
    ///
    /// Calibrates the LAPIC timer using a PIT-provided delay and programs it in periodic mode.
    /// Consumes this uninitialized handle and returns an initialized [`XapicTimer`].
    ///
    /// # Parameters
    ///
    /// - `pit`: PIT handle with one-shot delay capability.
    /// - `calibration_ms`: Duration of the measurement window in milliseconds.
    ///
    /// # Returns
    ///
    /// An initialized [`XapicTimer`] with the LAPIC periodic timer running.
    ///
    /// # Safety
    ///
    /// The LAPIC MMIO page must be identity-mapped and accessible.
    ///
    #[cfg(all(feature = "pit", feature = "microvm", feature = "whp"))]
    pub fn init(
        mut self,
        pit: &mut crate::hal::platform::pit::Pit,
        calibration_ms: u32,
    ) -> XapicTimer {
        // Validate calibration_ms before touching PIT/LAPIC hardware. A zero value
        // would arm the PIT with reload 0 (interpreted as max interval by the PIT)
        // and cause a division by zero when computing ticks_per_ms.
        debug_assert!(
            calibration_ms != 0,
            "lapic timer calibration called with calibration_ms == 0"
        );
        let calibration_ms: u32 = if calibration_ms == 0 {
            warn!("lapic timer calibration called with calibration_ms=0, clamping to 1ms");
            1
        } else {
            calibration_ms
        };

        // Timer vector = IDT hardware interrupt base + Timer IRQ 0.
        let timer_vector: u32 = crate::hal::arch::x86::cpu::idt::INT_OFF as u32;

        // Mask the LAPIC timer during calibration.
        self.write(
            xapic::XAPIC_TIMER,
            xapic::XapicTimer::new(timer_vector, false, true, 0).to_u32(),
        );

        // Set LAPIC timer divide-by-128.
        self.write(xapic::XAPIC_TDCR, 0x0A);

        // LAPIC timer calibration.
        //
        // The platform provides the host TSC base frequency (in MHz). When
        // non-zero, an RDTSC-based spin loop is used. This eliminates ~100
        // PIT-polling VM exits that are expensive during the first
        // WHvRunVirtualProcessor call (WHP lazily initialises internal partition
        // state) and removes the dependency on CPUID leaf 0x16 (unavailable on
        // i686 guests).
        //
        // When the value is zero (platform does not provide it), we fall back
        // to PIT-based calibration.
        let base_freq: u32 = crate::hal::platform::tsc_base_frequency_mhz();

        // NOTE: the RDTSC path always calibrates for ~1 ms regardless of
        // `calibration_ms`; only the PIT fallback uses that parameter.
        let mut ticks_per_ms: u32 = if base_freq > 0 {
            // RDTSC-based calibration (zero VM exits).
            let tsc_freq_mhz: u64 = base_freq as u64;
            let tsc_ticks_per_ms: u64 = tsc_freq_mhz * 1_000;

            info!("calibrating lapic timer via rdtsc (tsc_freq={}mhz)", tsc_freq_mhz);

            // Start the LAPIC timer counting down from max value.
            self.write(xapic::XAPIC_TICR, 0xFFFF_FFFF);

            // Spin for ~1 ms using RDTSC (zero VM exits). A max-iteration
            // guard prevents a hang if TSC does not advance.
            const RDTSC_MAX_ITERS: u64 = 1_000_000_000;
            let tsc_start: u64 = ::arch::cpu::rdtsc();
            let mut iters: u64 = 0;
            while (::arch::cpu::rdtsc() - tsc_start) < tsc_ticks_per_ms {
                core::hint::spin_loop();
                iters += 1;
                if iters >= RDTSC_MAX_ITERS {
                    warn!("rdtsc calibration timeout after {} iterations", RDTSC_MAX_ITERS);
                    break;
                }
            }

            // Read remaining LAPIC count and actual TSC delta to correct for overshoot.
            let current_count: u32 = self.read(xapic::XAPIC_TCCR);
            let elapsed_ticks: u32 = 0xFFFF_FFFF_u32.wrapping_sub(current_count);
            let tsc_elapsed: u64 = ::arch::cpu::rdtsc() - tsc_start;

            // ticks_per_ms = elapsed × (target / actual) so the result is independent
            // of TSC frequency errors. Guard against tsc_elapsed == 0 (TSC did not
            // advance); the ticks_per_ms == 0 fallback below handles the result.
            let tpm: u32 = (elapsed_ticks as u64 * tsc_ticks_per_ms)
                .checked_div(tsc_elapsed)
                .unwrap_or(0) as u32;

            info!(
                "lapic timer calibration (rdtsc): elapsed_ticks={}, ticks_per_ms={}, \
                 tsc_freq_mhz={}",
                elapsed_ticks, tpm, tsc_freq_mhz
            );
            tpm
        } else {
            // PIT-based fallback.
            info!(
                "vmm tsc_freq_mhz register is zero, calibrating lapic timer via pit channel 2 \
                 ({}ms window)",
                calibration_ms
            );

            // Arm the PIT one-shot — countdown starts on return.
            pit.arm_oneshot(calibration_ms);

            // Start the LAPIC timer counting down from max value.
            self.write(xapic::XAPIC_TICR, 0xFFFF_FFFF);

            // Wait for PIT delay to elapse.
            pit.wait_oneshot();

            // Read remaining LAPIC timer count and compute ticks per millisecond.
            let current_count: u32 = self.read(xapic::XAPIC_TCCR);
            let elapsed_ticks: u32 = 0xFFFF_FFFF_u32.wrapping_sub(current_count);
            let tpm: u32 = elapsed_ticks / calibration_ms;

            info!(
                "lapic timer calibration (pit fallback): elapsed_ticks={}, ticks_per_ms={}",
                elapsed_ticks, tpm
            );
            tpm
        };

        if ticks_per_ms == 0 {
            warn!("lapic timer calibration underflow: using fallback ticks_per_ms=1");
            ticks_per_ms = 1;
        }

        // Enable LAPIC via spurious interrupt vector register.
        self.write(xapic::XAPIC_SVR, 0x1FF);
        self.write(xapic::XAPIC_TPR, 0);
        info!("lapic svr enabled for interrupt delivery");

        // Program LAPIC timer in periodic mode.
        self.write(
            xapic::XAPIC_TIMER,
            xapic::XapicTimer::new(timer_vector, false, false, 1).to_u32(),
        );
        self.write(xapic::XAPIC_TICR, ticks_per_ms);

        info!("lapic periodic timer started (vector={:#x}, period=1ms)", timer_vector);

        XapicTimer {
            region: self.region,
        }
    }
}

//==================================================================================================
// xAPIC Timer (x86 only)
//==================================================================================================

///
/// # Description
///
/// An initialized xAPIC periodic timer. Owns the LAPIC MMIO region and keeps the timer running.
/// The interrupt controller handles EOI via a separate [`Xapic`] handle obtained from
/// [`Self::create_eoi_handle()`].
///
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)]
pub struct XapicTimer {
    /// LAPIC MMIO region handle (kept alive to prevent reallocation).
    region: IoMemoryRegion,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)]
impl XapicTimer {
    ///
    /// # Description
    ///
    /// Creates an [`Xapic`] handle that shares this timer's LAPIC MMIO page. The returned handle
    /// is intended for EOI writes only and is passed to the interrupt controller.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the returned `Xapic` handle is only used on a single core and
    /// does not outlive this `XapicTimer`.
    ///
    pub unsafe fn create_eoi_handle(&self) -> Xapic {
        Xapic {
            id: 0,
            ptr: xapic::Xapic::new(self.region.base().into_raw_value() as *mut u32),
        }
    }
}
