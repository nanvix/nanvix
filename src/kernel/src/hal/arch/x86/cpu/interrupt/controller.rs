// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::cpu::interrupt::{
    ioapic::{
        Ioapic,
        UninitIoapic,
    },
    map::InterruptMap,
    pic::{
        Pic,
        UninitPic,
    },
    xapic::{
        UninitXapic,
        Xapic,
    },
    InterruptNumber,
};
use ::arch::{
    self,
    cpu::msr,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

#[cfg(feature = "smp")]
use ::sys::mm::VirtualAddress;

//==================================================================================================
// Interrupt Vector
//==================================================================================================

/// Number of entries in the interrupt vector.
pub const INTERRUPT_VECTOR_LENGTH: usize = 256;

///
/// # Description
///
/// A type that represents an interrupt handler.
///
pub type InterruptHandler = unsafe fn(InterruptNumber);

#[unsafe(no_mangle)]
static mut INTERRUPT_VECTOR: [Option<InterruptHandler>; INTERRUPT_VECTOR_LENGTH] =
    [None; INTERRUPT_VECTOR_LENGTH];

//==================================================================================================
// Interrupt Controller
//==================================================================================================

enum InterruptControllerType {
    Legacy(Pic),
    Xapic(Xapic, Ioapic),
    PicXapic(Pic, Xapic),
    /// xAPIC-only mode: LAPIC handles timer delivery and EOI entirely
    /// in-kernel (via the WHP LAPIC emulator). No PIC initialization
    /// or routing is needed, eliminating ~47 VM exits from PIC I/O.
    XapicOnly(Xapic),
}

pub struct InterruptController {
    intmap: InterruptMap,
    intctrl: InterruptControllerType,
}

impl InterruptController {
    pub fn new(
        pic: Option<UninitPic>,
        xapic: Option<UninitXapic>,
        ioapic: Option<UninitIoapic>,
        intmap: InterruptMap,
        eoi_xapic: Option<Xapic>,
    ) -> Result<Self, Error> {
        // On WHP+microvm, when the xAPIC timer has already been
        // initialized (eoi_xapic is Some), skip PIC initialization
        // entirely. The WHP LAPIC emulator handles timer delivery
        // and EOI via MMIO — no VM exits. PIC ports (0x20/21/A0/A1)
        // are never accessed, eliminating ~47 exits per cold-start.
        #[cfg(all(feature = "microvm", feature = "whp"))]
        if let Some(xapic_eoi) = eoi_xapic {
            info!("using xapic-only mode (skipping pic init for whp)");
            return Ok(Self {
                intmap,
                intctrl: InterruptControllerType::XapicOnly(xapic_eoi),
            });
        }

        // If legacy PIC is available, initialize it.
        let pic: Option<Pic> = if let Some(mut pic) = pic {
            Some(pic.init()?)
        } else {
            None
        };

        // Check if xAPIC is available.
        if let Some(mut xapic) = xapic {
            // Check if IOAPIC is available.
            match ioapic {
                Some(ioapic) => {
                    info!("using xapic and ioapic");

                    // Enable APIC.
                    let apic_base: msr::ApicBase = msr::ApicBase::read();
                    info!("reading apic_base={:?}", apic_base);
                    // NOTE: check this in behavior in real hardware.
                    // Specification is unclear whether address is the full linear address or the
                    // page address. QEMU works with both, but it defaults to page address, so let's
                    // use that to keep consistency.
                    let apic_base: msr::ApicBase = msr::ApicBase::new(
                        (xapic.base() >> arch::mem::PAGE_SHIFT) as u64,
                        true,
                        true,
                    );
                    info!("writing apic_base={:?}", apic_base);
                    apic_base.write();

                    // Initialize xAPIC and I/O APIC.
                    let xapic: Xapic = xapic.init()?;
                    let ioapic: Ioapic = ioapic.init()?;

                    return Ok(Self {
                        intmap,
                        intctrl: InterruptControllerType::Xapic(xapic, ioapic),
                    });
                },
                None => {
                    // Disable APIC (no turning back).
                    let apic_base: msr::ApicBase = msr::ApicBase::read();
                    info!("reading apic_base={:?}", apic_base);
                    // NOTE: check this in behavior in real hardware.
                    // Specification is unclear whether address is the full linear address or the
                    // page address. QEMU works with both, but it defaults to page address, so let's
                    // use that to keep consistency.
                    let apic_base: msr::ApicBase = msr::ApicBase::new(
                        (xapic.base() >> arch::mem::PAGE_SHIFT) as u64,
                        true,
                        false,
                    );
                    info!("writing apic_base={:?}", apic_base);
                    apic_base.write();

                    warn!("ioapic not found, falling back to legacy pic");
                },
            }
        }

        // If legacy PIC is available, use it.
        if let Some(pic) = pic {
            // If an xAPIC EOI handle was provided (by the xAPIC timer init path),
            // use PIC for external IRQ routing and the xAPIC for EOI acknowledgement.
            if let Some(xapic_eoi) = eoi_xapic {
                info!("using pic with xapic for eoi");
                return Ok(Self {
                    intmap,
                    intctrl: InterruptControllerType::PicXapic(pic, xapic_eoi),
                });
            }

            // On microvm/WHP the partition enables LAPIC emulation in
            // xAPIC mode. Enable the LAPIC software-enable bit and
            // configure the LAPIC periodic timer so timer interrupts
            // fire entirely inside the WHP LAPIC emulator — zero VM
            // exits for timer delivery. The LAPIC page at 0xFEE00000
            // is identity-mapped via the microvm platform init and
            // handled by the WHP LAPIC emulator (not guest RAM).
            #[cfg(all(feature = "microvm", feature = "whp"))]
            {
                use ::arch::cpu::xapic;
                let lapic_base: usize = ::config::microvm::DEFAULT_LAPIC_BASE;
                let lapic: xapic::Xapic = xapic::Xapic::new(lapic_base as *mut u32);
                // SAFETY: The LAPIC MMIO page is identity-mapped during
                // microvm platform init. Writes go through the WHP LAPIC
                // emulator.
                unsafe {
                    lapic.write(xapic::XAPIC_SVR, 0x1FF);
                    lapic.write(xapic::XAPIC_TPR, 0);
                }
                info!("lapic svr enabled for whp interrupt delivery");

                // LAPIC timer calibration.
                //
                // When CPUID leaf 0x16 is available, an RDTSC-based spin
                // loop is used. This eliminates ~100 PIT-polling VM exits
                // that are extremely expensive during the first
                // WHvRunVirtualProcessor call (WHP lazily initialises
                // internal partition state).
                //
                // When leaf 0x16 is not available, we fall back to
                // PIT-based calibration with a reduced 1 ms window.

                // SAFETY: LAPIC registers go through the WHP emulator.
                // CPUID and RDTSC do not cause VM exits.
                unsafe {
                    // 1. Mask the LAPIC timer during calibration.
                    lapic.write(
                        xapic::XAPIC_TIMER,
                        xapic::XapicTimer::new(0x20, false, true, 0).to_u32(),
                    );

                    // 2. Set LAPIC timer divide-by-128.
                    lapic.write(xapic::XAPIC_TDCR, 0x0A);

                    // 3. Check CPUID leaf 0x16 for TSC frequency.
                    let base_freq: u32 = ::arch::cpu::cpuid::get_base_frequency_mhz();

                    let mut ticks_per_ms: u32 = if base_freq > 0 {
                        // RDTSC-based calibration (zero VM exits).
                        let tsc_freq_mhz: u64 = base_freq as u64;
                        let tsc_ticks_per_ms: u64 = tsc_freq_mhz * 1_000;

                        // 4a. Start the LAPIC timer counting from max value.
                        lapic.write(xapic::XAPIC_TICR, 0xFFFF_FFFF);

                        // 5a. Spin for ~1 ms using RDTSC (zero VM exits).
                        //     A max-iteration guard prevents a hang if TSC
                        //     does not advance (virtualisation quirk).
                        const RDTSC_MAX_ITERS: u64 = 1_000_000_000;
                        let tsc_start: u64 = ::arch::cpu::rdtsc();
                        let mut iters: u64 = 0;
                        while (::arch::cpu::rdtsc() - tsc_start) < tsc_ticks_per_ms {
                            core::hint::spin_loop();
                            iters += 1;
                            if iters >= RDTSC_MAX_ITERS {
                                warn!(
                                    "rdtsc calibration timeout after {} iterations",
                                    RDTSC_MAX_ITERS
                                );
                                break;
                            }
                        }

                        // 6a. Read remaining LAPIC count and actual TSC
                        //     delta to correct for overshoot.
                        let current_count: u32 = lapic.read(xapic::XAPIC_TCCR);
                        let elapsed_ticks: u32 = 0xFFFF_FFFF_u32.wrapping_sub(current_count);
                        let tsc_elapsed: u64 = ::arch::cpu::rdtsc() - tsc_start;

                        // ticks_per_ms = elapsed × (target / actual) so the
                        // result is independent of TSC frequency errors.
                        let tpm: u32 =
                            ((elapsed_ticks as u64 * tsc_ticks_per_ms) / tsc_elapsed) as u32;

                        info!(
                            "lapic timer calibration (rdtsc): elapsed_ticks={}, ticks_per_ms={}, \
                             tsc_freq_mhz={}",
                            elapsed_ticks, tpm, tsc_freq_mhz
                        );
                        tpm
                    } else {
                        // PIT-based fallback (reduced 1 ms window).
                        use ::arch::cpu::pit;
                        const CALIBRATION_MS: u32 = 1;
                        let pit_reload: u16 =
                            ((pit::PIT_MAX_FREQUENCY as u64 * CALIBRATION_MS as u64 / 1000)
                                & 0xFFFF) as u16;

                        warn!("cpuid leaf 0x16 unavailable, using pit-based calibration fallback");

                        // 4b. Program PIT channel 2 in one-shot mode.
                        let speaker: u8 = (::arch::io::in8(0x61) & 0xFC) | 0x01;
                        ::arch::io::out8(0x61, speaker);
                        ::arch::io::out8(
                            pit::PIT_CTRL,
                            pit::PIT_SEL2
                                | pit::PIT_ACC_LOHI
                                | pit::PIT_MODE_TCOUNT
                                | pit::PIT_BINARY,
                        );
                        ::arch::io::out8(pit::PIT_DATA + 2, (pit_reload & 0xFF) as u8);
                        ::arch::io::out8(pit::PIT_DATA + 2, (pit_reload >> 8) as u8);

                        // Start the LAPIC timer counting from max value.
                        lapic.write(xapic::XAPIC_TICR, 0xFFFF_FFFF);

                        // 5b. Wait for PIT channel 2 output (bit 5 of
                        //     port 0x61) with a bounded busy-wait.
                        const PIT_CALIBRATION_MAX_ITERS: u32 = 10_000_000;
                        let mut pit_iters: u32 = 0;
                        while (::arch::io::in8(0x61) & 0x20) == 0 {
                            core::hint::spin_loop();
                            pit_iters = pit_iters.wrapping_add(1);
                            if pit_iters >= PIT_CALIBRATION_MAX_ITERS {
                                warn!(
                                    "pit calibration timeout after {} iterations",
                                    PIT_CALIBRATION_MAX_ITERS
                                );
                                break;
                            }
                        }

                        // 6b. Read remaining LAPIC timer count.
                        let current_count: u32 = lapic.read(xapic::XAPIC_TCCR);
                        let elapsed_ticks: u32 = 0xFFFF_FFFF_u32.wrapping_sub(current_count);
                        let tpm: u32 = elapsed_ticks / CALIBRATION_MS;

                        info!(
                            "lapic timer calibration (pit fallback): elapsed_ticks={}, \
                             ticks_per_ms={}",
                            elapsed_ticks, tpm
                        );
                        tpm
                    };

                    if ticks_per_ms == 0 {
                        warn!("lapic timer calibration underflow: using fallback ticks_per_ms=1");
                        ticks_per_ms = 1;
                    }

                    // 7. Program LAPIC timer in periodic mode with vector
                    //    0x20, initial count = ticks_per_ms (1 kHz).
                    lapic.write(
                        xapic::XAPIC_TIMER,
                        xapic::XapicTimer::new(0x20, false, false, 1).to_u32(),
                    );
                    lapic.write(xapic::XAPIC_TICR, ticks_per_ms);

                    info!("lapic periodic timer started (vector=0x20, period=1ms)");
                }
            }

            info!("using legacy pic");
            return Ok(Self {
                intmap,
                intctrl: InterruptControllerType::Legacy(pic),
            });
        }

        let reason: &str = "no interrupt controller found";
        warn!("{reason}");
        Err(Error::new(ErrorCode::NoSuchDevice, reason))
    }

    pub fn ack(&mut self, intnum: InterruptNumber) -> Result<(), Error> {
        match self.intctrl {
            InterruptControllerType::Legacy(ref mut pic) => {
                pic.ack(intnum as u32);
                Ok(())
            },
            InterruptControllerType::Xapic(ref mut xapic, _) => {
                xapic.ack();
                Ok(())
            },
            InterruptControllerType::PicXapic(ref mut pic, ref mut xapic) => {
                // xAPIC EOI (MMIO write to 0xFEE000B0) runs every
                // tick to clear the ISR bit so the LAPIC can accept the
                // next periodic interrupt.
                xapic.ack();

                // PIC EOI (PMIO write to port 0x20) is throttled for
                // timer interrupts: only every N-th tick causes a VM
                // exit for pvclock updates. Between PIC EOI exits the
                // guest uses tick interpolation in monotonic_time_ns()
                // for zero-cost clock::now() calls. N=10 gives ~100 Hz
                // pvclock refresh rate with ~1 ms interpolation accuracy.
                // Non-timer IRQs (e.g., IKC) always send PIC EOI
                // immediately so they do not skew the pvclock refresh
                // cadence.
                if intnum == InterruptNumber::Timer {
                    use ::core::sync::atomic::{
                        AtomicU32,
                        Ordering,
                    };

                    /// PIC EOI divisor: send PIC EOI every N-th tick.
                    const PIC_EOI_DIVISOR: u32 = 10;

                    static EOI_COUNTER: AtomicU32 = AtomicU32::new(0);

                    let tick: u32 = EOI_COUNTER.fetch_add(1, Ordering::Relaxed);
                    if tick.is_multiple_of(PIC_EOI_DIVISOR) {
                        pic.ack(intnum as u32);
                    }
                } else {
                    pic.ack(intnum as u32);
                }
                Ok(())
            },
            InterruptControllerType::XapicOnly(ref mut xapic) => {
                xapic.ack();
                Ok(())
            },
        }
    }

    pub fn unmask(&mut self, intnum: InterruptNumber) -> Result<(), Error> {
        match self.intctrl {
            InterruptControllerType::Legacy(ref mut pic) => {
                pic.unmask(intnum as u16);
                Ok(())
            },
            // FIXME: enable interrupt on right CPU.
            InterruptControllerType::Xapic(_, ref mut ioapic) => {
                let intnum: u8 = self.intmap[intnum];
                ioapic.enable(intnum, 0)
            },
            InterruptControllerType::PicXapic(ref mut pic, _) => {
                pic.unmask(intnum as u16);
                Ok(())
            },
            InterruptControllerType::XapicOnly(_) => {
                // No PIC to unmask. LAPIC timer is already unmasked
                // during calibration; other interrupt sources (IKC)
                // are injected directly via the LAPIC by the VMM.
                Ok(())
            },
        }
    }

    ///
    /// # Description
    ///
    /// Starts up an application core.
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
        match self.intctrl {
            InterruptControllerType::Legacy(_)
            | InterruptControllerType::PicXapic(..)
            | InterruptControllerType::XapicOnly(_) => {
                let reason: &str = "pic does not support starting cores";
                error!("{reason}");
                Err(Error::new(ErrorCode::OperationNotSupported, reason))
            },
            InterruptControllerType::Xapic(ref mut xapic, _) => {
                xapic.start_core(coreid, entry, kstack)
            },
        }
    }

    pub fn set_handler(
        &mut self,
        intnum: InterruptNumber,
        handler: Option<InterruptHandler>,
    ) -> Result<(), Error> {
        let intnum: u8 = match self.intctrl {
            InterruptControllerType::Legacy(_)
            | InterruptControllerType::PicXapic(..)
            | InterruptControllerType::XapicOnly(_) => intnum as u8,
            InterruptControllerType::Xapic(_, _) => self.intmap[intnum],
        };
        unsafe { INTERRUPT_VECTOR[intnum as usize] = handler };
        Ok(())
    }

    pub fn get_handler(&self, intnum: InterruptNumber) -> Result<Option<InterruptHandler>, Error> {
        let intnum: u8 = match self.intctrl {
            InterruptControllerType::Legacy(_)
            | InterruptControllerType::PicXapic(..)
            | InterruptControllerType::XapicOnly(_) => intnum as u8,
            InterruptControllerType::Xapic(_, _) => self.intmap[intnum],
        };
        unsafe { Ok(INTERRUPT_VECTOR[intnum as usize]) }
    }
}
