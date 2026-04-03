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
    #[cfg(target_arch = "x86")]
    PicXapic(Pic, Xapic),
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
        #[cfg(target_arch = "x86")] eoi_xapic: Option<Xapic>,
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
                    // page address. QEMU works with both, but it defaults to page address, so
                    // let's use that to keep consistency.
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
                    // page address. QEMU works with both, but it defaults to page address, so
                    // let's use that to keep consistency.
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
            // On x86, if an xAPIC EOI handle was provided (by the xAPIC timer init path),
            // use PIC for external IRQ routing and the xAPIC for EOI acknowledgement.
            #[cfg(target_arch = "x86")]
            if let Some(xapic_eoi) = eoi_xapic {
                info!("using pic with xapic for eoi");
                return Ok(Self {
                    intmap,
                    intctrl: InterruptControllerType::PicXapic(pic, xapic_eoi),
                });
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
            #[cfg(target_arch = "x86")]
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
            #[cfg(target_arch = "x86")]
            InterruptControllerType::PicXapic(ref mut pic, _) => {
                pic.unmask(intnum as u16);
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
            InterruptControllerType::Legacy(_) => {
                let reason: &str = "legacy pic does not support starting cores";
                error!("{reason}");
                Err(Error::new(ErrorCode::OperationNotSupported, reason))
            },
            #[cfg(target_arch = "x86")]
            InterruptControllerType::PicXapic(..) => {
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
            InterruptControllerType::Legacy(_) => intnum as u8,
            #[cfg(target_arch = "x86")]
            InterruptControllerType::PicXapic(..) => intnum as u8,
            InterruptControllerType::Xapic(_, _) => self.intmap[intnum],
        };
        unsafe { INTERRUPT_VECTOR[intnum as usize] = handler };
        Ok(())
    }

    pub fn get_handler(&self, intnum: InterruptNumber) -> Result<Option<InterruptHandler>, Error> {
        let intnum: u8 = match self.intctrl {
            InterruptControllerType::Legacy(_) => intnum as u8,
            #[cfg(target_arch = "x86")]
            InterruptControllerType::PicXapic(..) => intnum as u8,
            InterruptControllerType::Xapic(_, _) => self.intmap[intnum],
        };
        unsafe { Ok(INTERRUPT_VECTOR[intnum as usize]) }
    }
}
