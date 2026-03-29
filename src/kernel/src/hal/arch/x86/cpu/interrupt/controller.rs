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

                // RDTSC-based LAPIC timer calibration.
                //
                // On WHP, every I/O port access (PIT polling) causes a
                // VM exit. During the very first WHvRunVirtualProcessor
                // call these exits are extremely expensive because WHP
                // lazily initialises internal partition state. Replacing
                // PIT-based calibration with an RDTSC spin loop
                // eliminates ~100 VM exits and saves significant time.
                //
                // The TSC frequency is obtained from CPUID leaf 0x16
                // (processor frequency information, EAX = base freq in
                // MHz). If the leaf is unavailable a 2 GHz default is
                // used; the correction step (target / actual) cancels
                // most of the error.

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

                    // 3. Obtain TSC frequency from CPUID leaf 0x16.
                    let cpuid16 = core::arch::x86::__cpuid(0x16);
                    let tsc_freq_mhz: u64 = if cpuid16.eax > 0 {
                        cpuid16.eax as u64
                    } else {
                        2_000 // 2 GHz fallback.
                    };
                    let tsc_ticks_per_ms: u64 = tsc_freq_mhz * 1_000;

                    // 4. Start the LAPIC timer counting from max value.
                    lapic.write(xapic::XAPIC_TICR, 0xFFFF_FFFF);

                    // 5. Spin for ~1 ms using RDTSC (zero VM exits).
                    let tsc_start: u64 = ::arch::cpu::rdtsc();
                    while (::arch::cpu::rdtsc() - tsc_start) < tsc_ticks_per_ms {
                        core::hint::spin_loop();
                    }

                    // 6. Read remaining LAPIC count and actual TSC delta
                    //    to correct for TSC frequency inaccuracy.
                    let current_count: u32 = lapic.read(xapic::XAPIC_TCCR);
                    let elapsed_ticks: u32 = 0xFFFF_FFFF_u32.wrapping_sub(current_count);
                    let tsc_elapsed: u64 = ::arch::cpu::rdtsc() - tsc_start;

                    // ticks_per_ms = elapsed × (target / actual) so the
                    // result is independent of TSC frequency errors.
                    let mut ticks_per_ms: u32 =
                        ((elapsed_ticks as u64 * tsc_ticks_per_ms) / tsc_elapsed) as u32;
                    if ticks_per_ms == 0 {
                        warn!(
                            "lapic timer calibration underflow: elapsed_ticks={elapsed_ticks}, \
                             using fallback ticks_per_ms=1"
                        );
                        ticks_per_ms = 1;
                    }

                    info!(
                        "lapic timer calibration (rdtsc): elapsed_ticks={}, ticks_per_ms={}, \
                         tsc_freq_mhz={}",
                        elapsed_ticks, ticks_per_ms, tsc_freq_mhz
                    );

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
