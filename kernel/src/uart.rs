// Copyright(c) 2The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// Imports
//==================================================================================================

use alloc::rc::Rc;
use core::cell::RefCell;

use crate::{
    error::Error,
    hal::io::{
        IoPortAllocator,
        ReadOnlyIoPort,
        ReadWriteIoPort,
    },
    stdout::Stdout,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Divisors for common baud rates.
pub enum BaudRate {
    /// Divisor for 115200 baud rate.
    Baud115200 = 1,
    /// Divisor for 57600 baud rate.
    Baud57600 = 2,
    /// Divisor for 38400 baud rate.
    Baud38400 = 3,
}

impl From<BaudRate> for u16 {
    fn from(val: BaudRate) -> Self {
        val as u16
    }
}

#[repr(u16)]
enum UartRegisterInterface {
    /// THR, RBR and IER.
    ThrRbrIer = 0x00,
    /// DLL and DLM.
    DllDlm = 0x01,
    /// IIR and FCR.
    IirFcr = 0x02,
    /// LCR.
    Lcr = 0x03,
    /// MCR.
    Mcr = 0x04,
    /// LSR.
    Lsr = 0x05,
    /// MSR.
    Msr = 0x06,
    /// SCR.
    Scr = 0x07,
}

impl From<UartRegisterInterface> for u16 {
    fn from(val: UartRegisterInterface) -> Self {
        val as u16
    }
}

// General Register Interface
const UART_DATA: u16 = 0x00; // Data Register (R/W)
const UART_IIR: u16 = 0x02; // Interrupt Identification Register (R)
const UART_FCR: u16 = 0x02; // FIFO Control Register (W)
const UART_LCR: u16 = 0x03; // Line Control Register (RW)
const UART_MCR: u16 = 0x04; // Modem Control Register (W)
const UART_LSR: u16 = 0x05; // Line Status Register (R)
const UART_MSR: u16 = 0x06; // Modem Status Register (R)
const UART_SCR: u16 = 0x07; // Scratch Register (RW)

// Register Interface (when DLA is Unset in LCR)
const UART_RBR: u16 = 0x00; // Receiver Buffer (R)
const UART_THR: u16 = 0x00; // Transmitter Holding Register (W)
const UART_IER: u16 = 0x01; // Interrupt Enable Register (RW)

// Register Interface (when DLA is Set in LCR)
const UART_DLL: u16 = 0x00; // Divisor Latch LSB (RW)
const UART_DLM: u16 = 0x01; // Divisor Latch MSB (RW)

//==================================================================================================
// Structures
//==================================================================================================

/// UART Ports.
#[repr(u16)]
enum UartPort {
    /// Port 0.
    Port0 = 0x3f8,
    /// Port 1.
    Port1 = 0x2f8,
    /// Port 2.
    Port2 = 0x3e8,
    /// Port 3.
    Port3 = 0x2e8,
    /// Port 4.
    Port4 = 0x5f8,
    /// Port 5.
    Port5 = 0x4f8,
    /// Port 6.
    Port6 = 0x5e8,
    /// Port 7.
    Port7 = 0x4e8,
}

impl From<UartPort> for u16 {
    fn from(val: UartPort) -> Self {
        val as u16
    }
}

struct Register<T> {
    bits: T,
}

#[derive(Clone)]
struct SharedIoPort {
    port: Rc<RefCell<ReadWriteIoPort>>,
}

impl SharedIoPort {
    fn new(port: ReadWriteIoPort) -> Self {
        Self {
            port: Rc::new(RefCell::new(port)),
        }
    }

    fn readb(&self) -> u8 {
        self.port.borrow().read8()
    }

    fn writeb(&mut self, value: u8) {
        self.port.borrow_mut().write8(value);
    }
}

pub struct Uart {
    thr: TransmitterHoldingBufferRegister,
    rbr: ReceiverBufferRegister,
    dll: DivisorLatchLsbRegister,
    ier: InterruptEnableRegister,
    dlm: DivisorLatchMsbRegister,
    iir: InterruptIdentificationRegister,
    fcr: FifoControlRegister,
    lcr: LineControlRegister,
    mcr: ModemControlRegister,
    lsr: LineStatusRegister,
    msr: ModemStatusRegister,
    scr: ScratchRegister,
}

/// Transmitter Holding Buffer (THR) Register.
struct TransmitterHoldingBufferRegister {
    port: SharedIoPort,
    register: Register<u8>,
}

/// Receiver Buffer Register.
struct ReceiverBufferRegister {
    port: SharedIoPort,
    register: Register<u8>,
}

/// Divisor Latch LSB Register.
struct DivisorLatchLsbRegister {
    port: SharedIoPort,
    register: Register<u8>,
}

/// Interrupt Enable Register.
struct InterruptEnableRegister {
    port: SharedIoPort,
    register: Register<u8>,
}

/// Divisor Latch MSB Register.
struct DivisorLatchMsbRegister {
    port: SharedIoPort,
    register: Register<u8>,
}

/// Interrupt Identification Register.
struct InterruptIdentificationRegister {
    port: SharedIoPort,
    register: Register<u8>,
}

/// FIFO Control Register.
struct FifoControlRegister {
    port: SharedIoPort,
    register: Register<u8>,
}

/// Line Control Register.
struct LineControlRegister {
    port: ReadWriteIoPort,
    register: Register<u8>,
}

/// Modem Control Register.
struct ModemControlRegister {
    port: ReadWriteIoPort,
    register: Register<u8>,
}

/// Line Status Register.
struct LineStatusRegister {
    port: ReadOnlyIoPort,
    register: Register<u8>,
}

/// Modem Status Register.
struct ModemStatusRegister {
    port: ReadOnlyIoPort,
    register: Register<u8>,
}

/// Scratch Register.
struct ScratchRegister {
    port: ReadWriteIoPort,
    register: Register<u8>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Register<u8> {
    fn new(bits: u8) -> Self {
        Self { bits }
    }

    fn read(&self) -> u8 {
        self.bits
    }

    fn write(&mut self, value: u8) {
        self.bits = value;
    }
}

impl TransmitterHoldingBufferRegister {
    fn new(port: SharedIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn write(&mut self, value: u8) {
        self.register.write(value);
        self.port.writeb(value);
    }
}

impl ReceiverBufferRegister {
    fn new(port: SharedIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.readb());
        self.register.read()
    }
}

impl DivisorLatchLsbRegister {
    fn new(port: SharedIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.readb());
        self.register.read()
    }

    fn write(&mut self, value: u8) {
        self.register.write(value);
        self.port.writeb(value);
    }
}

impl InterruptEnableRegister {
    /// Receiver Data Available Interrupt
    const UART_IER_RDAI: u8 = 1 << 0;
    /// Transmitter Holding Register Interrupt
    const UART_IER_THRI: u8 = 1 << 1;
    /// Receiver Line Status Interrupt
    const UART_IER_RLSI: u8 = 1 << 2;
    /// Modem Status Interrupt
    const UART_IER_MSI: u8 = 1 << 3;

    fn new(port: SharedIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.readb());
        self.register.read()
    }

    fn write(&mut self, value: u8) {
        self.register.write(value);
        self.port.writeb(value);
    }
}

impl DivisorLatchMsbRegister {
    fn new(port: SharedIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.readb());
        self.register.read()
    }

    fn write(&mut self, value: u8) {
        self.register.write(value);
        self.port.writeb(value);
    }
}

impl InterruptIdentificationRegister {
    /// Modem Status
    const UART_IIR_MS: u8 = 0x00;
    /// Transmitter Holding Register Empty
    const UART_IIR_THRE: u8 = 0x02;
    /// Receiver Data Available
    const UART_IIR_RDA: u8 = 0x04;
    /// Receiver Line Status
    const UART_IIR_RLS: u8 = 0x06;

    fn new(port: SharedIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.readb());
        self.register.read()
    }
}

impl FifoControlRegister {
    const UART_FCR_DISABLE_BIT: u8 = 1 << 0; // Disable FIFO
    const UART_FCR_CLRRECV_BIT: u8 = 1 << 1; // Clear Receiver FIFO
    const UART_FCR_CLRTMIT_BIT: u8 = 1 << 2; // Clear Transmitter FIFO
    const UART_FCR_DMA_SEL_BIT: u8 = 1 << 3; // DMA Select

    const UART_FCR_TRIG_1: u8 = 0x00; // Trigger Level 1 byte
    const UART_FCR_TRIG_4: u8 = 0x40; // Trigger Level 4 bytes
    const UART_FCR_TRIG_8: u8 = 0x80; // Trigger Level 8 bytes
    const UART_FCR_TRIG_14: u8 = 0xc0; // Trigger Level 14 bytes

    fn new(port: SharedIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn write(&mut self, value: u8) {
        self.register.write(value);
        self.port.writeb(value);
    }
}

impl LineControlRegister {
    const UART_LCR_DLA: u8 = 0x80; // Divisor Latch Access
    const UART_LCR_BPC_5: u8 = 0x00; // 5 Bits per Character
    const UART_LCR_BPC_6: u8 = 0x01; // 6 Bits per Character
    const UART_LCR_BPC_7: u8 = 0x02; // 7 Bits per Character
    const UART_LCR_BPC_8: u8 = 0x03; // 8 Bits per Character
    const UART_LCR_STOP_SINGLE: u8 = 0x00; // Single Stop Bit
    const UART_LCR_STOP_VARIABLE: u8 = 0x04; // Variable Stop Bits
    const UART_LCR_PARITY_NONE: u8 = 0x00; // No Parity
    const UART_LCR_PARITY_ODD: u8 = 0x08; // Odd Parity
    const UART_LCR_PARITY_EVEN: u8 = 0x0c; // Even Parity

    fn new(port: ReadWriteIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.read8());
        self.register.read()
    }

    fn write(&mut self, value: u8) {
        self.register.write(value);
        self.port.write8(value);
    }
}

impl ModemControlRegister {
    const UART_MCR_DTR: u8 = 1 << 0; // Data Terminal Ready
    const UART_MCR_RTS: u8 = 1 << 1; // Request Send Ping
    const UART_MCR_OUT1: u8 = 1 << 2; // Output Pin 1
    const UART_MCR_OUT2: u8 = 1 << 3; // Output Pin 2
    const UART_MCR_LOOP: u8 = 1 << 4; // Loopback Mode

    fn new(port: ReadWriteIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.read8());
        self.register.read()
    }

    fn write(&mut self, value: u8) {
        self.register.write(value);
        self.port.write8(value);
    }
}

impl LineStatusRegister {
    const UART_LSR_DR: u8 = 1 << 0; // Data Ready
    const UART_LSR_OE: u8 = 1 << 1; // Overrun Error
    const UART_LSR_PE: u8 = 1 << 2; // Parity Error
    const UART_LSR_FE: u8 = 1 << 3; // Framing Error
    const UART_LSR_BI: u8 = 1 << 4; // Break Indicator
    const UART_LSR_TFE: u8 = 1 << 5; // Transmitter FIFO Empty
    const UART_LSR_TEI: u8 = 1 << 6; // Transmitter Empty Indicator
    const UART_LSR_ERR: u8 = 1 << 7; // Erroneous Data in FIFO

    fn new(port: ReadOnlyIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.read8());
        self.register.read()
    }
}

impl ModemStatusRegister {
    const UART_MSR_CCTS: u8 = 1 << 0; // Change in CTS
    const UART_MSR_CDSR: u8 = 1 << 1; // Change in DSR
    const UART_MSR_TERI: u8 = 1 << 2; // Trailing Edge RI
    const UART_MSR_CDCD: u8 = 1 << 3; // Change in CD
    const UART_MSR_CTS: u8 = 1 << 4; // Clear to Send (CTS)
    const UART_MSR_DSR: u8 = 1 << 5; // Data Set Ready (DSR)
    const UART_MSR_RI: u8 = 1 << 6; // Ring Indicator (RI)
    const UART_MSR_CD: u8 = 1 << 7; // Carrier Detect CD

    fn new(port: ReadOnlyIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.read8());
        self.register.read()
    }
}

impl ScratchRegister {
    fn new(port: ReadWriteIoPort) -> Self {
        Self {
            port,
            register: Register::new(0),
        }
    }

    fn read(&mut self) -> u8 {
        self.register.write(self.port.read8());
        self.register.read()
    }

    fn write(&mut self, value: u8) {
        self.register.write(value);
        self.port.write8(value);
    }
}

impl Uart {
    pub fn new(ioports: &mut IoPortAllocator, baud_rate: BaudRate) -> Result<Self, Error> {
        // THR, RBR and IER share the same I/O port.
        let thr_rbr_ier_port: SharedIoPort =
            SharedIoPort::new(ioports.allocate_read_write(UartPort::Port0 as u16)?);

        // DLL and DLM share the same I/O port.
        let dll_dlm_port: SharedIoPort =
            SharedIoPort::new(ioports.allocate_read_write(UartPort::Port0 as u16 + 1)?);

        // IIR and FCR share the same I/O port.
        let irr_fcr_port: SharedIoPort =
            SharedIoPort::new(ioports.allocate_read_write(UartPort::Port0 as u16 + 2)?);

        let lcr_port: ReadWriteIoPort = ioports.allocate_read_write(UartPort::Port0 as u16 + 3)?;
        let mcr_port: ReadWriteIoPort = ioports.allocate_read_write(UartPort::Port0 as u16 + 4)?;
        let lsr_port: ReadOnlyIoPort = ioports.allocate_read_only(UartPort::Port0 as u16 + 5)?;
        let msr_port: ReadOnlyIoPort = ioports.allocate_read_only(UartPort::Port0 as u16 + 6)?;
        let scr_port: ReadWriteIoPort = ioports.allocate_read_write(UartPort::Port0 as u16 + 7)?;

        // THR, RBR and IER share the same I/O port.
        let thr: TransmitterHoldingBufferRegister =
            TransmitterHoldingBufferRegister::new(thr_rbr_ier_port.clone());
        let rbr: ReceiverBufferRegister = ReceiverBufferRegister::new(thr_rbr_ier_port.clone());
        let ier: InterruptEnableRegister = InterruptEnableRegister::new(thr_rbr_ier_port);

        // DLL and DLM share the same I/O port.
        let dll: DivisorLatchLsbRegister = DivisorLatchLsbRegister::new(dll_dlm_port.clone());
        let dlm: DivisorLatchMsbRegister = DivisorLatchMsbRegister::new(dll_dlm_port);

        // IIR and FCR share the same I/O port.
        let iir: InterruptIdentificationRegister =
            InterruptIdentificationRegister::new(irr_fcr_port.clone());
        let fcr: FifoControlRegister = FifoControlRegister::new(irr_fcr_port);

        let lcr: LineControlRegister = LineControlRegister::new(lcr_port);
        let mcr: ModemControlRegister = ModemControlRegister::new(mcr_port);
        let lsr: LineStatusRegister = LineStatusRegister::new(lsr_port);
        let msr: ModemStatusRegister = ModemStatusRegister::new(msr_port);
        let scr: ScratchRegister = ScratchRegister::new(scr_port);

        let mut uart: Uart = Uart {
            thr,
            rbr,
            dll,
            ier,
            dlm,
            iir,
            fcr,
            lcr,
            mcr,
            lsr,
            msr,
            scr,
        };

        uart.init(baud_rate);

        Ok(uart)
    }

    pub fn write(&mut self, s: &str) {
        for b in s.bytes() {
            self.wait();
            self.thr.write(b);
        }
    }

    fn init(&mut self, baud_rate: BaudRate) {
        // Disable all interrupts.
        self.disable_interrupts();

        // Set baud rate.
        self.set_baud_rate(baud_rate.into());

        // Set 8 bits per character, no parity and one stop bit.
        self.lcr.write(
            LineControlRegister::UART_LCR_BPC_8
                | LineControlRegister::UART_LCR_PARITY_NONE
                | LineControlRegister::UART_LCR_STOP_SINGLE,
        );

        // Enable FIFOs, clear tem, use 14-byte threshold.
        self.fcr.write(
            FifoControlRegister::UART_FCR_TRIG_14
                | FifoControlRegister::UART_FCR_CLRTMIT_BIT
                | FifoControlRegister::UART_FCR_CLRRECV_BIT,
        );

        // TODO: Add a test here to check if the output serial line is faulty.
    }

    fn disable_interrupts(&mut self) {
        self.ier.write(0);
    }

    fn wait(&mut self) {
        while (self.lsr.read() & LineStatusRegister::UART_LSR_TFE) == 0 {}
    }

    fn set_baud_rate(&mut self, divisor: u16) {
        // Enable divisor latch access bit.
        let mut lcr_bits: u8 = self.lcr.read();
        lcr_bits |= LineControlRegister::UART_LCR_DLA;
        self.lcr.write(lcr_bits);

        // Set divisor LSB.
        self.dll.write((divisor & 0xff) as u8);

        // Set divisor MSB.
        self.dlm.write((divisor >> 8) as u8);

        // Disable divisor latch access bit.
        lcr_bits &= !LineControlRegister::UART_LCR_DLA;
        self.lcr.write(lcr_bits);
    }
}

impl Stdout for Uart {
    fn puts(&mut self, s: &str) {
        self.write(s);
    }
}
