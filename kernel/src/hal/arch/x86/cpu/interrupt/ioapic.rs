// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::cpu::ioapic,
    error::{
        Error,
        ErrorCode,
    },
    hal::{
        io::IoMemoryRegion,
        mem::Address,
    },
};

//==================================================================================================
// Uninitialized I/O APIC
//==================================================================================================

///
/// # Description
///
/// A struct that represents an uninitialized I/O APIC.
///
pub struct UninitIoapic {
    /// Interrupt vector base.
    intvec_base: u8,
    /// I/O APIC ID.
    id: u8,
    /// I/O APIC base address.
    base: IoMemoryRegion,
    /// Global System Interrupt (GSI).
    gsi: u32,
}

impl UninitIoapic {
    ///
    /// # Description
    ///
    /// Instantiates an uninitialized I/O APIC.
    ///
    /// # Parameters
    ///
    /// - `intvec_base`: Interrupt vector base.
    /// - `id`: I/O APIC ID.
    /// - `addr`: I/O APIC base address.
    /// - `gsi`: Global System Interrupt (GSI).
    ///
    /// # Return Values
    ///
    /// A new instance of an uninitialized I/O APIC is returned.
    ///
    pub fn new(intvec_base: u8, id: u8, base: IoMemoryRegion, gsi: u32) -> UninitIoapic {
        UninitIoapic {
            intvec_base,
            id,
            base,
            gsi,
        }
    }

    ///
    /// # Description
    ///
    /// Initializes the target I/O APIC.
    ///
    /// # Return Values
    ///
    /// Upon success, an initialized I/O APIC is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn init(&mut self) -> Result<Ioapic, Error> {
        info!("initializing ioapic (id={}, addr={:?}, gsi={})", self.id, self.base, self.gsi);

        let mut ioapic: Ioapic = Ioapic {
            _base: self.base.clone(),
            intvec_base: self.intvec_base,
            ptr: ioapic::Ioapic::new(self.base.base().into_raw_value()),
        };

        // Check ID mismatch.
        if ioapic::IoapicId::id(ioapic.deref_mut()) != self.id {
            let reason: &str = "id mismatch";
            error!("init(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, &reason));
        }

        ioapic.print_info();

        let maxintr: u8 = ioapic::IoapicVersion::maxredirect(ioapic.deref_mut());

        // For all interrupts: set physical destination mode to APIC ID 0; set high
        // activate; set edge-triggered; set disabled; set fixed delivery mode;
        // identity map interrupts.
        for irq in 0..=maxintr {
            ioapic::IoapicRedirectionTable::write(
                ioapic.deref_mut(),
                irq as u32,
                0,
                ioapic::IoapicRedirectionTableLow::IOREDTBL_INTMASK_MASK
                    | (self.intvec_base + irq) as u32,
            );
        }

        Ok(ioapic)
    }
}

//==================================================================================================
// Initialized I/O APIC
//==================================================================================================

///
/// # Description
///
/// A struct that represents an initialized I/O APIC.
///
pub struct Ioapic {
    /// I/O APIC base address.
    _base: IoMemoryRegion, // NOTE: we must keep this here to avoid deallocation.
    /// Interrupt vector base.
    intvec_base: u8,
    /// Underlying I/O APIC.
    ptr: ioapic::Ioapic,
}

impl Ioapic {
    /// Enables an interrupt line.
    pub fn enable(&mut self, irq: u8, cpunum: u8) -> Result<(), Error> {
        info!("trace(): irq={}, cpunum={}", irq, cpunum);
        // When using physical destination mode, only the lower 4 bits of the
        // destination field are used. The specification is unclear about the
        // behavior of the upper bits. See 82093AA I/O ADVANCED PROGRAMMABLE
        // INTERRUPT CONTROLLER (IOAPIC) for details.
        const MAXIMUM_NUMBER_CPUS: u8 = 16;

        // Check IRQ lies in a valid range.
        if irq >= ioapic::IoapicVersion::maxredirect(self.deref_mut()) {
            let reason: &str = "invalid irq number";
            error!("enable(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, &reason));
        }

        // Check CPU number lies in a valid range.
        if cpunum > MAXIMUM_NUMBER_CPUS {
            let reason: &str = "invalid cpu number";
            error!("enable(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, &reason));
        }

        // Set physical destination mode to cpunum; set high activate; set
        // edge-triggered; set enabled; set fixed delivery mode; identity map
        // interrupt.
        // NOTE: The following cast is safe because "cpunum" is a 4-bit value.
        // NOTE: The following cast is safe because "irq" is an 8-bit value.
        let intvec_base: u8 = self.intvec_base;
        ioapic::IoapicRedirectionTable::write(
            self.deref_mut(),
            irq as u32,
            (cpunum as u32) << ioapic::IoapicRedirectionTableHigh::IOREDTBL_DEST_SHIFT,
            (intvec_base + irq) as u32,
        );

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Prints information about the target I/O APIC.
    ///
    pub fn print_info(&mut self) {
        info!("ioapic id: {}", ioapic::IoapicId::id(self.deref_mut()));
        info!("ioapic version: {}", ioapic::IoapicVersion::version(self.deref_mut()));
        info!(
            "ioapic max redirection entries: {}",
            ioapic::IoapicVersion::maxredirect(self.deref_mut())
        );
        info!("ioapic pointers: {:?}", self.ptr);
    }

    ///
    /// # Description
    ///
    /// Convenient function to obtain a mutable reference to the underlying I/O APIC.
    ///
    /// # Return Values
    ///
    /// A mutable reference to the I/O APIC.
    ///
    fn deref_mut(&mut self) -> &mut ioapic::Ioapic {
        &mut self.ptr
    }
}
