// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]

/// xAPIC Registers
pub const XAPIC_ID: u32 = 0x0020 / 4; // ID Register
pub const XAPIC_VER: u32 = 0x0030 / 4; // Version Register
pub const XAPIC_TPR: u32 = 0x0080 / 4; // Task Priority
pub const XAPIC_PPR: u32 = 0x00A0 / 4; // Processor Priority
pub const XAPIC_EOI: u32 = 0x00B0 / 4; // EOI Register
pub const XAPIC_LDR: u32 = 0x00D0 / 4; // Logical Destination Register
pub const XAPIC_SVR: u32 = 0x00F0 / 4; // Spurious Interrupt Vector Register
pub const XAPIC_ISR: u32 = 0x0100 / 4; // In-Service Register
pub const XAPIC_TMR: u32 = 0x0180 / 4; // Trigger Mode Register
pub const XAPIC_IRR: u32 = 0x0200 / 4; // Interrupt Request Register
pub const XAPIC_ESR: u32 = 0x0280 / 4; // Error Status Register
pub const XAPIC_CMCI: u32 = 0x02F0 / 4; // CMCI LVT Register
pub const XAPIC_ICRLO: u32 = 0x0300 / 4; // Interrupt Command Register
pub const XAPIC_ICRHI: u32 = 0x0310 / 4; // Interrupt Command Register
pub const XAPIC_TIMER: u32 = 0x0320 / 4; // Timer LVT Register
pub const XAPIC_THERM: u32 = 0x0330 / 4; // Thermal Sensor LVT Register
pub const XAPIC_PCINT: u32 = 0x0340 / 4; // Performance Counter LVT Register
pub const XAPIC_LINT0: u32 = 0x0350 / 4; // Local Interrupt 0 LVT Register
pub const XAPIC_LINT1: u32 = 0x0360 / 4; // Local Interrupt 1 LVT Register
pub const XAPIC_ERROR: u32 = 0x0370 / 4; // Error LVT Register
pub const XAPIC_TICR: u32 = 0x0380 / 4; // Timer Initial Count Register
pub const XAPIC_TCCR: u32 = 0x0390 / 4; // Timer Current Count Register
pub const XAPIC_TDCR: u32 = 0x03E0 / 4; // Timer Divide Configuration Register

#[repr(C)]
pub struct XapicId {
    bits: u32,
}

impl XapicId {
    fn new(id: u32) -> Self {
        Self { bits: id }
    }

    pub fn id(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn from_u32(id: u32) -> Self {
        Self { bits: id }
    }
}

#[repr(C)]
pub struct XapicVer {
    bits: u32,
}

impl XapicVer {
    pub fn new(bits: u32) -> Self {
        Self { bits }
    }

    pub fn version(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn max_lvt(&self) -> u32 {
        (self.bits >> 16) & 0xFF
    }

    pub fn eoi_broadcast_supported(&self) -> bool {
        ((self.bits >> 24) & 0x1) != 0
    }

    pub fn from_u32(bits: u32) -> Self {
        Self { bits }
    }
}

#[repr(C)]
pub struct XapicTpr {
    bits: u32,
}

impl XapicTpr {
    pub fn new(priority_subclass: u32, priority_class: u32) -> Self {
        Self {
            bits: (priority_subclass & 0xF) | ((priority_class & 0xF) << 4),
        }
    }

    pub fn priority_subclass(&self) -> u32 {
        self.bits & 0xF
    }

    pub fn priority_class(&self) -> u32 {
        (self.bits >> 4) & 0xF
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

#[repr(C)]
pub struct XapicSvr {
    bits: u32,
}

impl XapicSvr {
    pub fn new(vector: u32, apic_enabled: bool, focus_checking: bool, eoi_broadcast: bool) -> Self {
        Self {
            bits: (vector & 0xFF)
                | ((apic_enabled as u32) << 8)
                | ((focus_checking as u32) << 9)
                | ((eoi_broadcast as u32) << 12),
        }
    }

    pub fn vector(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn apic_enabled(&self) -> bool {
        ((self.bits >> 8) & 0x1) != 0
    }

    pub fn focus_checking(&self) -> bool {
        ((self.bits >> 9) & 0x1) != 0
    }

    pub fn eoi_broadcast(&self) -> bool {
        ((self.bits >> 12) & 0x1) != 0
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

#[repr(C)]
pub struct XapicEsr {
    bits: u32,
}

impl XapicEsr {
    pub fn new(
        send_checksum_error: bool,
        receive_checksum_error: bool,
        send_accept_error: bool,
        receive_accept_error: bool,
        redirectable: bool,
        send_illegal_vector: bool,
        received_illegal_vector: bool,
        illegal_register_address: bool,
    ) -> Self {
        Self {
            bits: (send_checksum_error as u32)
                | ((receive_checksum_error as u32) << 1)
                | ((send_accept_error as u32) << 2)
                | ((receive_accept_error as u32) << 3)
                | ((redirectable as u32) << 4)
                | ((send_illegal_vector as u32) << 5)
                | ((received_illegal_vector as u32) << 6)
                | ((illegal_register_address as u32) << 7),
        }
    }

    pub fn send_checksum_error(&self) -> bool {
        (self.bits & 0x1) != 0
    }

    pub fn receive_checksum_error(&self) -> bool {
        (self.bits >> 1 & 0x1) != 0
    }

    pub fn send_accept_error(&self) -> bool {
        (self.bits >> 2 & 0x1) != 0
    }

    pub fn receive_accept_error(&self) -> bool {
        (self.bits >> 3 & 0x1) != 0
    }

    pub fn redirectable(&self) -> bool {
        (self.bits >> 4 & 0x1) != 0
    }

    pub fn send_illegal_vector(&self) -> bool {
        (self.bits >> 5 & 0x1) != 0
    }

    pub fn received_illegal_vector(&self) -> bool {
        (self.bits >> 6 & 0x1) != 0
    }

    pub fn illegal_register_address(&self) -> bool {
        (self.bits >> 7 & 0x1) != 0
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

#[repr(C)]
pub struct XapicCmci {
    bits: u32,
}

impl XapicCmci {
    pub fn new(vector: u32, delivery_mode: u32, delivery_status: bool, mask: bool) -> Self {
        Self {
            bits: (vector & 0xFF)
                | ((delivery_mode & 0x7) << 8)
                | ((delivery_status as u32) << 12)
                | ((mask as u32) << 16),
        }
    }

    pub fn vector(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn delivery_mode(&self) -> u32 {
        (self.bits >> 8) & 0x7
    }

    pub fn delivery_status(&self) -> bool {
        ((self.bits >> 12) & 0x1) != 0
    }

    pub fn mask(&self) -> bool {
        ((self.bits >> 16) & 0x1) != 0
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

#[derive(Default)]
#[repr(C)]
pub struct XapicIcrLo {
    bits: u32,
}

impl XapicIcrLo {
    pub fn new(
        vector: u32,
        delivery_mode: u32,
        destination_mode: bool,
        level: bool,
        trigger_mode: bool,
        destination_shorthand: u32,
    ) -> Self {
        Self {
            bits: (vector & 0xFF)
                | ((delivery_mode & 0x7) << 8)
                | ((destination_mode as u32) << 11)
                | ((level as u32) << 14)
                | ((trigger_mode as u32) << 15)
                | ((destination_shorthand & 0x3) << 18),
        }
    }

    pub fn vector(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn delivery_mode(&self) -> u32 {
        (self.bits >> 8) & 0x7
    }

    pub fn destination_mode(&self) -> bool {
        ((self.bits >> 11) & 0x1) != 0
    }

    pub fn delivery_status(&self) -> bool {
        ((self.bits >> 12) & 0x1) != 0
    }

    pub fn level(&self) -> bool {
        ((self.bits >> 14) & 0x1) != 0
    }

    pub fn trigger_mode(&self) -> bool {
        ((self.bits >> 15) & 0x1) != 0
    }

    pub fn destination_shorthand(&self) -> u32 {
        (self.bits >> 18) & 0x3
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }

    pub fn from_u32(bits: u32) -> Self {
        Self { bits }
    }
}

#[repr(C)]
pub struct XapicIcrHi {
    bits: u32,
}

impl XapicIcrHi {
    pub fn new(destination: u32) -> Self {
        Self {
            bits: destination << 24,
        }
    }

    pub fn destination(&self) -> u32 {
        self.bits >> 24
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

#[repr(C)]
pub struct XapicTimer {
    bits: u32,
}

impl XapicTimer {
    pub fn new(vector: u32, delivery_status: bool, mask: bool, mode: u32) -> Self {
        Self {
            bits: (vector & 0xFF)
                | ((delivery_status as u32) << 12)
                | ((mask as u32) << 16)
                | ((mode & 0x3) << 17),
        }
    }

    pub fn vector(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn delivery_status(&self) -> bool {
        ((self.bits >> 12) & 0x1) != 0
    }

    pub fn mask(&self) -> bool {
        ((self.bits >> 16) & 0x1) != 0
    }

    pub fn mode(&self) -> u32 {
        (self.bits >> 17) & 0x3
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

#[repr(C)]
pub struct XapicThermal {
    bits: u32,
}

impl XapicThermal {
    pub fn new(vector: u32, delivery_mode: u32, delivery_status: bool, mask: bool) -> Self {
        Self {
            bits: (vector & 0xFF)
                | ((delivery_mode & 0x7) << 8)
                | ((delivery_status as u32) << 12)
                | ((mask as u32) << 16),
        }
    }

    pub fn vector(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn delivery_mode(&self) -> u32 {
        (self.bits >> 8) & 0x7
    }

    pub fn delivery_status(&self) -> bool {
        ((self.bits >> 12) & 0x1) != 0
    }

    pub fn mask(&self) -> bool {
        ((self.bits >> 16) & 0x1) != 0
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

#[repr(C)]
pub struct XapicPcint {
    bits: u32,
}

impl XapicPcint {
    pub fn new(vector: u32, delivery_mode: u32, delivery_status: bool, mask: bool) -> Self {
        Self {
            bits: (vector & 0xFF)
                | ((delivery_mode & 0x7) << 8)
                | ((delivery_status as u32) << 12)
                | ((mask as u32) << 16),
        }
    }

    pub fn vector(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn delivery_mode(&self) -> u32 {
        (self.bits >> 8) & 0x7
    }

    pub fn delivery_status(&self) -> bool {
        ((self.bits >> 12) & 0x1) != 0
    }

    pub fn mask(&self) -> bool {
        ((self.bits >> 16) & 0x1) != 0
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

#[repr(C)]
pub struct XapicLint {
    bits: u32,
}

impl XapicLint {
    pub fn new(
        vector: u32,
        delivery_mode: u32,
        delivery_status: bool,
        polarity: bool,
        remote_irr: bool,
        trigger_mode: bool,
        mask: bool,
    ) -> Self {
        Self {
            bits: (vector & 0xFF)
                | ((delivery_mode & 0x7) << 8)
                | ((delivery_status as u32) << 12)
                | ((polarity as u32) << 13)
                | ((remote_irr as u32) << 14)
                | ((trigger_mode as u32) << 15)
                | ((mask as u32) << 16),
        }
    }

    pub fn vector(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn delivery_mode(&self) -> u32 {
        (self.bits >> 8) & 0x7
    }

    pub fn delivery_status(&self) -> bool {
        ((self.bits >> 12) & 0x1) != 0
    }

    pub fn polarity(&self) -> bool {
        ((self.bits >> 13) & 0x1) != 0
    }

    pub fn remote_irr(&self) -> bool {
        ((self.bits >> 14) & 0x1) != 0
    }

    pub fn trigger_mode(&self) -> bool {
        ((self.bits >> 15) & 0x1) != 0
    }

    pub fn mask(&self) -> bool {
        ((self.bits >> 16) & 0x1) != 0
    }
}

#[repr(C)]
pub struct XapicError {
    bits: u32,
}

impl XapicError {
    pub fn new(vector: u32, delivery_status: bool, mask: bool) -> Self {
        Self {
            bits: (vector & 0xFF) | ((delivery_status as u32) << 12) | ((mask as u32) << 16),
        }
    }

    pub fn vector(&self) -> u32 {
        self.bits & 0xFF
    }

    pub fn delivery_status(&self) -> bool {
        ((self.bits >> 12) & 0x1) != 0
    }

    pub fn mask(&self) -> bool {
        ((self.bits >> 16) & 0x1) != 0
    }

    pub fn to_u32(&self) -> u32 {
        self.bits
    }
}

/// Delivery status for LVT registers.
#[repr(u8)]
pub enum XapicLvtDeliveryStatus {
    Idle = 0,        // Idle.
    SendPending = 1, // Send pending.
}

/// Delivery mode.
#[repr(u8)]
pub enum XapicIcrDeliveryMode {
    FixedDelivery = 0,  // Fixed delivery mode.
    LowestPriority = 1, // Lowest priority mode.
    Smi = 2,            // System management mode.
    Nmi = 4,            // Non-maskable interrupt.
    Init = 5,           // INIT mode.
    Startup = 6,        // Startup mode.
    Extint = 7,         // External interrupt.
}

/// Destination shorthand.
#[repr(u8)]
pub enum XapicIcrDestinationShorthand {
    NoShorthand = 0,      // No shorthand.
    OnlySelf = 1,         // Self.
    AllIncludingSelf = 2, // All including self.
    AllExcludingSelf = 3, // All excluding self.
}

/// Delivery status.
#[repr(u8)]
pub enum XapicIcrDeliveryStatus {
    Idle = 0,        // Idle.
    SendPending = 1, // Send pending.
}

/// xAPIC Interrupt Vectors.
#[repr(u8)]
pub enum XapicIntvec {
    Cmci = 247,     // CMCI interrupt vector.
    Thermal = 248,  // Thermal Sensor interrupt vector.
    Pcint = 249,    // Performance Counter interrupt vector.
    Timer = 250,    // Timer interrupt vector.
    Lint1 = 251,    // Local Interrupt 1 vector.
    Lint0 = 252,    // Local Interrupt 0 vector.
    Error = 253,    // Error interrupt vector.
    Ipi = 254,      // Inter-Processor Interrupt.
    Spurious = 255, // Spurious interrupt vector.
}

pub struct Xapic(*mut u32);

impl Xapic {
    pub fn new(base: *mut u32) -> Self {
        Self(base)
    }

    pub unsafe fn read(&self, reg: u32) -> u32 {
        self.0.add(reg as usize).read_volatile()
    }

    pub unsafe fn write(&self, reg: u32, value: u32) {
        self.0.add(reg as usize).write_volatile(value)
    }
}
