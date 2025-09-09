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
                Some(mut ioapic) => {
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
