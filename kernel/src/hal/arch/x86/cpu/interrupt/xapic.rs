// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::cpu::xapic,
    hal::{
        io::IoMemoryRegion,
        mem::Address,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

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
            error!("init(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, &reason));
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
    /// # Return Values
    ///
    /// The base address of the target xAPIC.
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
}
