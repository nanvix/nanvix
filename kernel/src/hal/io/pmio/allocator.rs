// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::io::{
    pmio::{
        info::IoPortInfo,
        ioport::IoPort,
    },
    IoPortType,
    IoPortWidth,
};
use ::alloc::{
    collections::LinkedList,
    rc::Rc,
};
use ::core::cell::RefCell;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

struct IoPortAllocatorEntry {
    port: IoPort,
}

///
/// **Description**
/// An I/O port allocator.
///
pub struct IoPortAllocator {
    /// Registered I/O ports.
    ports: LinkedList<Rc<RefCell<IoPortAllocatorEntry>>>,
}

///
/// **Description**
/// A read-only I/O port.
///
pub struct ReadOnlyIoPort {
    port: Rc<RefCell<IoPortAllocatorEntry>>,
}

///
/// **Description**
/// A write-only I/O port.
///
#[allow(dead_code)]
pub struct WriteOnlyIoPort {
    port: Rc<RefCell<IoPortAllocatorEntry>>,
}

///
/// **Description**
/// A read-write I/O port.
///
pub struct ReadWriteIoPort {
    port: Rc<RefCell<IoPortAllocatorEntry>>,
}

///
/// # Description
///
/// A type that represents either a [ReadOnlyIoPort], [WriteOnlyIoPort], or [ReadWriteIoPort].
///
pub enum AnyIoPort {
    ReadOnly(ReadOnlyIoPort),
    WriteOnly(WriteOnlyIoPort),
    ReadWrite(ReadWriteIoPort),
}

//==================================================================================================
// Implementations
//==================================================================================================

impl IoPortAllocator {
    pub fn new() -> Self {
        Self {
            ports: LinkedList::new(),
        }
    }

    ///
    /// **Description**
    /// Registers a read-only I/O port with the allocator.
    ///
    /// **Parameters**
    /// - `number`: Number of the I/O port.
    ///
    /// **Returns**
    /// - `Ok(())` on success.
    /// - `Err(Error::ENOMEM)` if the allocator is full.
    /// - `Err(Error::EADDRINUSE)` if the I/O port is already registered.
    ///
    pub fn register_read_only(&mut self, number: u16) -> Result<(), Error> {
        let info: IoPortInfo = IoPortInfo::new(number);
        let entry: IoPort = IoPort::new_read_only(info);
        self.register(entry)
    }

    ///
    /// **Description**
    /// Registers a write-only I/O port with the allocator.
    ///
    /// **Parameters**
    /// - `number`: Number of the I/O port.
    ///
    /// **Returns**
    /// - `Ok(())` on success.
    /// - `Err(Error::ENOMEM)` if the allocator is full.
    /// - `Err(Error::EADDRINUSE)` if the I/O port is already registered.
    ///
    #[allow(dead_code)]
    pub fn register_write_only(&mut self, number: u16) -> Result<(), Error> {
        let info: IoPortInfo = IoPortInfo::new(number);
        let entry: IoPort = IoPort::new_write_only(info);
        self.register(entry)
    }

    ///
    /// **Description**
    /// Registers a read-write I/O port with the allocator.
    ///
    /// **Parameters**
    /// - `number`: Number of the I/O port.
    ///
    /// **Returns**
    /// - `Ok(())` on success.
    /// - `Err(Error::ENOMEM)` if the allocator is full.
    /// - `Err(Error::EADDRINUSE)` if the I/O port is already registered.
    ///
    pub fn register_read_write(&mut self, number: u16) -> Result<(), Error> {
        let info: IoPortInfo = IoPortInfo::new(number);
        let entry: IoPort = IoPort::new_read_write(info);
        self.register(entry)
    }

    ///
    /// **Description**
    /// Registers an I/O port with the allocator.
    ///
    /// **Parameters**
    /// - `port`: I/O port to register.
    ///
    /// **Returns**
    /// - `Ok(())` on success.
    /// - `Err(Error::ENOMEM)` if the allocator is full.
    /// - `Err(Error::EADDRINUSE)` if the I/O port is already registered.
    ///
    fn register(&mut self, port: IoPort) -> Result<(), Error> {
        // Check if an I/O port with the same number is already registered.
        for entry in self.ports.iter() {
            if let Ok(e) = entry.try_borrow() {
                if e.port.number() == port.number() {
                    return Err(Error::new(
                        ErrorCode::AddressInUse,
                        "io port is already registered",
                    ));
                }
            }
        }

        // Register the I/O port.
        let entry: Rc<RefCell<IoPortAllocatorEntry>> =
            Rc::new(RefCell::new(IoPortAllocatorEntry { port }));
        self.ports.push_back(entry);

        Ok(())
    }

    ///
    /// **Description**
    /// Allocates a read-only I/O port.
    ///
    /// **Parameters**
    /// - `number`: Number of the I/O port.
    ///
    /// **Returns**
    /// - `Ok(ReadOnlyIoPort)` on success.
    /// - `Err(Error::EBUSY)` if the I/O port is already allocated.
    /// - `Err(Error::ENOENT)` if the I/O port is not registered.
    ///
    pub fn allocate_read_only(&mut self, number: u16) -> Result<ReadOnlyIoPort, Error> {
        self.allocate_io_port(number, IoPortType::ReadOnly)
            .map(|entry| ReadOnlyIoPort { port: entry })
    }

    //
    // **Description**
    // Allocates a write-only I/O port.
    //
    // **Parameters**
    // - `number`: Number of the I/O port.
    //
    // **Returns**
    // - `Some(WriteOnlyIoPort)` on success.
    // - `Err(Error::EBUSY)` if the I/O port is already allocated.
    // - `Err(Error::ENOENT)` if the I/O port is not registered.
    //
    #[allow(dead_code)]
    pub fn allocate_write_only(&mut self, number: u16) -> Result<WriteOnlyIoPort, Error> {
        self.allocate_io_port(number, IoPortType::WriteOnly)
            .map(|entry| WriteOnlyIoPort { port: entry })
    }

    //
    // **Description**
    // Allocates a read-write I/O port.
    //
    // **Parameters**
    // - `number`: Number of the I/O port.
    //
    // **Returns**
    // - `Ok(ReadWriteIoPort)` on success.
    // - `Err(Error::EBUSY)` if the I/O port is already allocated.
    // - `Err(Error::ENOENT)` if the I/O port is not registered.
    //
    pub fn allocate_read_write(&mut self, number: u16) -> Result<ReadWriteIoPort, Error> {
        self.allocate_io_port(number, IoPortType::ReadWrite)
            .map(|entry| ReadWriteIoPort { port: entry })
    }

    fn allocate_io_port(
        &mut self,
        number: u16,
        typ: IoPortType,
    ) -> Result<Rc<RefCell<IoPortAllocatorEntry>>, Error> {
        // Search for the I/O port.
        for entry in self.ports.iter_mut() {
            // Check if the I/O port is allocated.
            if Rc::<RefCell<IoPortAllocatorEntry>>::strong_count(entry) > 1 {
                // Skip ports that are already allocated.
                continue;
            }

            if let Ok(e) = entry.try_borrow() {
                // Check if the I/O port is the target one.
                if e.port.number() == number {
                    // Check if the I/O port is the target type.
                    if e.port.typ == typ {
                        // It is, so we can allocate it.
                        return Ok(entry.clone());
                    }
                }
            }
        }

        // The I/O port is not registered.
        Err(Error::new(ErrorCode::NoSuchEntry, "io port is not registered"))
    }
}

impl ReadOnlyIoPort {
    pub fn read8(&self) -> u8 {
        self.port.borrow().port.read8()
    }

    #[allow(dead_code)]
    pub fn read16(&self) -> u16 {
        self.port.borrow().port.read16()
    }

    pub fn read32(&self) -> u32 {
        self.port.borrow().port.read32()
    }
}

#[allow(dead_code)]
impl WriteOnlyIoPort {
    pub fn write8(&mut self, value: u8) {
        self.port.borrow_mut().port.write8(value)
    }

    pub fn write16(&mut self, value: u16) {
        self.port.borrow_mut().port.write16(value)
    }

    pub fn write32(&mut self, value: u32) {
        self.port.borrow_mut().port.write32(value)
    }
}

impl ReadWriteIoPort {
    pub fn read8(&self) -> u8 {
        self.port.borrow().port.read8()
    }

    #[allow(dead_code)]
    pub fn read16(&self) -> u16 {
        self.port.borrow().port.read16()
    }

    pub fn read32(&self) -> u32 {
        self.port.borrow().port.read32()
    }

    pub fn write8(&mut self, b: u8) {
        self.port.borrow_mut().port.write8(b)
    }

    pub fn write16(&mut self, w: u16) {
        self.port.borrow_mut().port.write16(w)
    }

    pub fn write32(&mut self, dw: u32) {
        self.port.borrow_mut().port.write32(dw)
    }
}

impl AnyIoPort {
    pub fn number(&self) -> u16 {
        match self {
            AnyIoPort::ReadOnly(port) => port.port.borrow().port.number(),
            AnyIoPort::WriteOnly(port) => port.port.borrow().port.number(),
            AnyIoPort::ReadWrite(port) => port.port.borrow().port.number(),
        }
    }
}

impl AnyIoPort {
    pub fn read(&self, port_width: IoPortWidth) -> Result<u32, Error> {
        match self {
            AnyIoPort::ReadOnly(port) => match port_width {
                IoPortWidth::Bits8 => Ok(port.read8() as u32),
                IoPortWidth::Bits16 => Ok(port.read16() as u32),
                IoPortWidth::Bits32 => Ok(port.read32()),
            },
            AnyIoPort::WriteOnly(_) => {
                let reason: &'static str = "write-only io port";
                error!("read(): {:?}", reason);
                Err(Error::new(ErrorCode::OperationNotSupported, &reason))
            },
            AnyIoPort::ReadWrite(port) => match port_width {
                IoPortWidth::Bits8 => Ok(port.read8() as u32),
                IoPortWidth::Bits16 => Ok(port.read16() as u32),
                IoPortWidth::Bits32 => Ok(port.read32()),
            },
        }
    }

    pub fn write(&mut self, port_width: IoPortWidth, value: u32) -> Result<(), Error> {
        match self {
            AnyIoPort::ReadOnly(_) => {
                let reason: &'static str = "read-only io port";
                error!("write(): {:?}", reason);
                return Err(Error::new(ErrorCode::OperationNotSupported, &reason));
            },
            AnyIoPort::WriteOnly(port) => match port_width {
                IoPortWidth::Bits8 => port.write8(value as u8),
                IoPortWidth::Bits16 => port.write16(value as u16),
                IoPortWidth::Bits32 => port.write32(value),
            },
            AnyIoPort::ReadWrite(port) => match port_width {
                IoPortWidth::Bits8 => port.write8(value as u8),
                IoPortWidth::Bits16 => port.write16(value as u16),
                IoPortWidth::Bits32 => port.write32(value),
            },
        }
        Ok(())
    }
}
