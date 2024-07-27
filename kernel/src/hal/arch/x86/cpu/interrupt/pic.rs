// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::{
        cpu::pic,
        io,
    },
    error::{
        Error,
        ErrorCode,
    },
    hal::io::{
        IoPortAllocator,
        ReadWriteIoPort,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default mask. All interrupts disabled, expect IRQ 2 which is used for cascading.
const DEFAULT_MASK: u16 = 0xfffb;

//==================================================================================================
// Uninitialized PIC
//==================================================================================================

///
/// # Description
///
/// A structure that represents the resources allocated to an uninitialized Programmable Interrupt
/// Controller (PIC).
///
struct UninitPicResources {
    /// Master PIC Control Register.
    ctrl_master: ReadWriteIoPort,
    /// Master PIC Data Register
    data_master: ReadWriteIoPort,
    /// Slave PIC Control Register
    ctrl_slave: ReadWriteIoPort,
    /// Slave PIC Data Register
    data_slave: ReadWriteIoPort,
}

///
/// # Description
///
/// A struct that represents an uninitialized Programmable Interrupt Controller (PIC).
///
pub struct UninitPic {
    /// Offset.
    offset: u8,
    /// Underlying resources.
    resources: Option<UninitPicResources>,
}

impl UninitPic {
    ///
    /// # Description
    ///
    /// Instantiates an uninitialized PIC.
    ///
    /// # Parameters
    ///
    /// - `ioports`: I/O port allocator.
    /// - `offset`: Vector offset.
    ///
    /// # Return Values
    ///
    /// Upon success, a new instance of an uninitialized PIC is returned. The pic must be
    /// initialized before it can be used and initialization may be performed by calling
    /// [`Self::init()`]. Upon failure, an error is returned instead.
    ///
    pub fn new(ioports: &mut IoPortAllocator, offset: u8) -> Result<Self, Error> {
        let ctrl_master: ReadWriteIoPort =
            ioports.allocate_read_write(pic::PIC_CTRL_MASTER as u16)?;
        let data_master: ReadWriteIoPort =
            ioports.allocate_read_write(pic::PIC_DATA_MASTER as u16)?;
        let ctrl_slave: ReadWriteIoPort =
            ioports.allocate_read_write(pic::PIC_CTRL_SLAVE as u16)?;
        let data_slave: ReadWriteIoPort =
            ioports.allocate_read_write(pic::PIC_DATA_SLAVE as u16)?;

        Ok(Self {
            offset,
            resources: Some(UninitPicResources {
                ctrl_master,
                data_master,
                ctrl_slave,
                data_slave,
            }),
        })
    }

    ///
    /// # Description
    ///
    /// Initializes the target PIC.
    ///
    /// # Return Values
    ///
    /// An initialized PIC is returned.
    ///
    pub fn init(&mut self) -> Result<Pic, Error> {
        info!("initializing pic (offset={})", self.offset);

        let mut pic: Pic = {
            let UninitPicResources {
                ctrl_master,
                data_master,
                ctrl_slave,
                data_slave,
            } = match self.resources.take() {
                Some(resources) => resources,
                None => {
                    // NOTE: This error is unexpected, as resources should be available.
                    let reason: &str = "pic already initialized";
                    error!("init(): {}", reason);
                    return Err(Error::new(ErrorCode::TryAgain, reason));
                },
            };
            Pic {
                mask: DEFAULT_MASK,
                ctrl_master,
                data_master,
                ctrl_slave,
                data_slave,
            }
        };

        // Starts initialization sequence in cascade mode.
        pic.ctrl_master.write8(pic::icw1::Ic4::Needed as u8);
        io::wait();
        pic.ctrl_slave.write8(pic::icw1::Ic4::Needed as u8);
        io::wait();

        // Send new vector offset.
        pic.data_master.write8(self.offset);
        io::wait();
        pic.data_slave.write8(self.offset + pic::PIC_NUM_IRQS);
        io::wait();

        // Tell the master that there is a slave PIC at IRQ line 2 and
        // tell the slave PIC that it is cascaded.
        pic.data_master.write8(0x04);
        io::wait();
        pic.data_slave.write8(0x02);
        io::wait();

        // Set 8086 mode.
        pic.data_master
            .write8(pic::icw4::MicroprocessorMode::Mode8086 as u8);
        io::wait();
        pic.data_slave
            .write8(pic::icw4::MicroprocessorMode::Mode8086 as u8);
        io::wait();

        // Mask all interrupts.
        pic.disable();

        Ok(pic)
    }
}

//==================================================================================================
// Implementations
//==================================================================================================

///
/// # Description
///
/// A struct that represents an initialized Programmable Interrupt Controller (PIC).
///
pub struct Pic {
    /// Interrupt mask.
    mask: u16,
    /// Master PIC Control Register
    ctrl_master: ReadWriteIoPort,
    /// Master PIC Data Register
    data_master: ReadWriteIoPort,
    /// Slave PIC Control Register
    ctrl_slave: ReadWriteIoPort,
    /// Slave PIC Data Register
    data_slave: ReadWriteIoPort,
}

impl Pic {
    ///
    /// # Description
    ///
    /// Unmasks an interrupt in the target PIC.
    ///
    /// # Parameters
    ///
    /// - `mask`: Mask.
    ///
    pub fn unmask(&mut self, intnum: u16) {
        self.mask &= !(1 << intnum);
        self.data_master.write8(self.mask as u8);
        io::wait();
        self.data_slave.write8((self.mask >> 8) as u8);
        io::wait();
    }

    ///
    /// # Description
    ///
    /// Disables all interrupts in the target PIC.
    ///
    pub fn disable(&mut self) {
        self.mask = DEFAULT_MASK;
        self.data_master.write8(self.mask as u8);
        io::wait();
        self.data_slave.write8((self.mask >> 8) as u8);
        io::wait();
    }

    ///
    /// # Description
    ///
    /// Acknowledges an interrupt.
    ///
    /// # Parameters
    ///
    /// - `irq`: Interrupt request.
    ///
    pub fn ack(&mut self, irq: u32) {
        // Check for invalid interrupt request. IRQ 2 is reserved for the cascade.
        if irq == 2 || irq >= pic::PIC_NUM_IRQS as u32 {
            error!("invalid irq {}", irq);
            return;
        }

        // Check if EOI is managed by slave PIC.
        if irq >= pic::PIC_NUM_IRQS as u32 {
            // Send EOI to slave PIC.
            self.ctrl_slave.write8(pic::ocw2::Eoi::NonSpecific as u8);
        }

        // Send EOI to master PIC.
        self.ctrl_master.write8(pic::ocw2::Eoi::NonSpecific as u8);
    }
}
