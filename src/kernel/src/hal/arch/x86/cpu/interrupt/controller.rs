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
    ) -> Result<Self, Error> {
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
            info!("using legacy pic");

            // On microvm/WHP the partition enables LAPIC emulation in
            // xAPIC mode. Enable the LAPIC software-enable bit and
            // configure the LAPIC periodic timer so timer interrupts
            // fire entirely inside the WHP LAPIC emulator — zero VM
            // exits for timer delivery. The LAPIC page at 0xFEE00000
            // is identity-mapped via the microvm platform init and
            // handled by the WHP LAPIC emulator (not guest RAM).
            #[cfg(all(feature = "microvm", feature = "whp"))]
            {
                use ::arch::cpu::{
                    pit,
                    xapic,
                };
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

                // PIT-based LAPIC timer calibration.
                //
                // We use PIT channel 2 in one-shot mode to measure how
                // many LAPIC timer ticks elapse in a known duration.
                // PIT I/O (ports 0x40-0x43, 0x61) causes PMIO VM exits,
                // which returns control to the VMM loop and allows
                // pvclock to advance during calibration. At runtime,
                // PIC EOI is skipped; pvclock is updated via guest-
                // driven refresh and a 100 Hz host timer fallback.
                const CALIBRATION_MS: u32 = 1;
                let pit_reload: u16 = ((pit::PIT_MAX_FREQUENCY as u64 * CALIBRATION_MS as u64
                    / 1000)
                    & 0xFFFF) as u16;

                // SAFETY: All I/O port accesses below target the PIT and
                // speaker gate, which are emulated by the VMM. The LAPIC
                // registers are accessed through the identity-mapped MMIO
                // page handled by the WHP LAPIC emulator.
                unsafe {
                    // 1. Mask the LAPIC timer during calibration.
                    lapic.write(
                        xapic::XAPIC_TIMER,
                        xapic::XapicTimer::new(0x20, false, true, 0).to_u32(),
                    );

                    // 2. Set LAPIC timer divide-by-128.
                    lapic.write(xapic::XAPIC_TDCR, 0x0A);

                    // 3. Program PIT channel 2 in one-shot mode.
                    // Enable speaker gate for channel 2 and clear output bit.
                    let speaker: u8 = (::arch::io::in8(0x61) & 0xFC) | 0x01;
                    ::arch::io::out8(0x61, speaker);
                    // Channel 2, lobyte/hibyte, mode 0 (one-shot), binary.
                    ::arch::io::out8(
                        pit::PIT_CTRL,
                        pit::PIT_SEL2 | pit::PIT_ACC_LOHI | pit::PIT_MODE_TCOUNT | pit::PIT_BINARY,
                    );
                    ::arch::io::out8(pit::PIT_DATA + 2, (pit_reload & 0xFF) as u8);
                    ::arch::io::out8(pit::PIT_DATA + 2, (pit_reload >> 8) as u8);

                    // 4. Start the LAPIC timer counting from max value.
                    lapic.write(xapic::XAPIC_TICR, 0xFFFF_FFFF);

                    // 5. Wait for PIT channel 2 output (bit 5 of port 0x61),
                    //    but avoid an unbounded busy-wait in case OUT2 never
                    //    transitions due to misconfigured PIT emulation.
                    const PIT_CALIBRATION_MAX_ITERS: u32 = 10_000_000;
                    let mut pit_iters: u32 = 0;
                    while (::arch::io::in8(0x61) & 0x20) == 0 {
                        core::hint::spin_loop();
                        pit_iters = pit_iters.wrapping_add(1);
                        if pit_iters >= PIT_CALIBRATION_MAX_ITERS {
                            warn!(
                                "PIT calibration timeout: OUT2 did not assert after {} iterations",
                                PIT_CALIBRATION_MAX_ITERS
                            );
                            break;
                        }
                    }

                    // 6. Read remaining LAPIC timer count.
                    let current_count: u32 = lapic.read(xapic::XAPIC_TCCR);
                    let elapsed_ticks: u32 = 0xFFFF_FFFF - current_count;
                    let mut ticks_per_ms: u32 = elapsed_ticks / CALIBRATION_MS;
                    if ticks_per_ms == 0 {
                        // Calibration underflow: use minimal non-zero fallback
                        // to avoid programming LAPIC TICR with 0.
                        warn!(
                            "lapic timer calibration underflow: elapsed_ticks={elapsed_ticks}, \
                             using fallback ticks_per_ms=1"
                        );
                        ticks_per_ms = 1;
                    }

                    info!(
                        "lapic timer calibration: elapsed_ticks={}, ticks_per_ms={}",
                        elapsed_ticks, ticks_per_ms
                    );

                    // 7. Program LAPIC timer in periodic mode with vector
                    //    0x20, initial count = ticks_per_ms (1kHz).
                    lapic.write(
                        xapic::XAPIC_TIMER,
                        xapic::XapicTimer::new(0x20, false, false, 1).to_u32(),
                    );
                    lapic.write(xapic::XAPIC_TICR, ticks_per_ms);

                    info!("lapic periodic timer started (vector=0x20, period=1ms)");
                }
            }

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
                // On microvm/WHP the LAPIC periodic timer delivers
                // interrupts through the LAPIC emulator.
                //
                // **LAPIC EOI** (MMIO write to 0xFEE000B0) runs every
                // tick to clear the ISR bit so the LAPIC can accept the
                // next periodic interrupt.
                //
                // **PIC EOI** (PMIO write to port 0x20) runs every
                // N-th tick to cause a VM exit for pvclock updates.
                // Between PIC EOI exits, the guest uses tick
                // interpolation in monotonic_time_ns() for zero-cost
                // clock::now() calls. N=10 gives ~100 Hz pvclock
                // refresh rate with ~1 ms interpolation accuracy.
                #[cfg(all(feature = "microvm", feature = "whp"))]
                {
                    use ::arch::cpu::xapic;
                    use ::core::sync::atomic::{
                        AtomicU32,
                        Ordering,
                    };

                    /// PIC EOI divisor: send PIC EOI every N-th tick.
                    const PIC_EOI_DIVISOR: u32 = 10;

                    static EOI_COUNTER: AtomicU32 = AtomicU32::new(0);

                    let lapic_base: usize = ::config::microvm::DEFAULT_LAPIC_BASE;
                    let lapic: xapic::Xapic = xapic::Xapic::new(lapic_base as *mut u32);
                    // SAFETY: LAPIC MMIO page is identity-mapped.
                    unsafe {
                        lapic.write(xapic::XAPIC_EOI, 0);
                    }

                    // Only throttle PIC EOI for timer interrupts. Non-timer
                    // IRQs (e.g., IKC) always send PIC EOI immediately so
                    // they do not skew the pvclock refresh cadence.
                    if intnum == InterruptNumber::Timer {
                        let tick: u32 = EOI_COUNTER.fetch_add(1, Ordering::Relaxed);
                        if tick % PIC_EOI_DIVISOR == 0 {
                            pic.ack(intnum as u32);
                        }
                    } else {
                        pic.ack(intnum as u32);
                    }
                    return Ok(());
                }
                #[cfg(not(all(feature = "microvm", feature = "whp")))]
                {
                    pic.ack(intnum as u32);
                    Ok(())
                }
            },
            InterruptControllerType::Xapic(ref mut xapic, _) => {
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
            InterruptControllerType::Legacy(_) => {
                let reason: &str = "legacy pic does not support starting cores";
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
            InterruptControllerType::Legacy(_) => intnum as u8,
            InterruptControllerType::Xapic(_, _) => self.intmap[intnum],
        };
        unsafe { INTERRUPT_VECTOR[intnum as usize] = handler };
        Ok(())
    }

    pub fn get_handler(&self, intnum: InterruptNumber) -> Result<Option<InterruptHandler>, Error> {
        let intnum: u8 = match self.intctrl {
            InterruptControllerType::Legacy(_) => intnum as u8,
            InterruptControllerType::Xapic(_, _) => self.intmap[intnum],
        };
        unsafe { Ok(INTERRUPT_VECTOR[intnum as usize]) }
    }
}
