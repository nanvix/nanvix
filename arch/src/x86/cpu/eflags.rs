// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::ring;

//==================================================================================================
// Carry Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the carry flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarryFlag {
    /// The carry flag is clear.
    Clear = 0,
    /// The carry flag is set.
    Set = (1 << Self::SHIFT),
}

impl CarryFlag {
    /// Bit shift of the carry flag.
    const SHIFT: u32 = 0;
    /// Bit mask of the carry flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => CarryFlag::Clear,
            _ => CarryFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Parity Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the parity flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParityFlag {
    /// The parity flag is clear.
    Clear = 0,
    /// The parity flag is set.
    Set = (1 << Self::SHIFT),
}

impl ParityFlag {
    /// Bit shift of the parity flag.
    const SHIFT: u32 = 2;
    /// Bit mask of the parity flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => ParityFlag::Clear,
            _ => ParityFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Auxiliary Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the auxiliary flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxiliaryFlag {
    /// The adjust flag is clear.
    Clear = 0,
    /// The adjust flag is set.
    Set = (1 << Self::SHIFT),
}

impl AuxiliaryFlag {
    /// Bit shift of the adjust flag.
    const SHIFT: u32 = 4;
    /// Bit mask of the adjust flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => AuxiliaryFlag::Clear,
            _ => AuxiliaryFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Zero Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the zero flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroFlag {
    /// The zero flag is clear.
    Clear = 0,
    /// The zero flag is set.
    Set = (1 << Self::SHIFT),
}

impl ZeroFlag {
    /// Bit shift of the zero flag.
    const SHIFT: u32 = 6;
    /// Bit mask of the zero flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => ZeroFlag::Clear,
            _ => ZeroFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Sign Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the sign flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignFlag {
    /// The sign flag is clear.
    Clear = 0,
    /// The sign flag is set.
    Set = (1 << Self::SHIFT),
}

impl SignFlag {
    /// Bit shift of the sign flag.
    const SHIFT: u32 = 7;
    /// Bit mask of the sign flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => SignFlag::Clear,
            _ => SignFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Trap Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the trap flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapFlag {
    /// The trap flag is clear.
    Clear = 0,
    /// The trap flag is set.
    Set = (1 << Self::SHIFT),
}

impl TrapFlag {
    /// Bit shift of the trap flag.
    const SHIFT: u32 = 8;
    /// Bit mask of the trap flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => TrapFlag::Clear,
            _ => TrapFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Interrupt Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the interrupt flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptFlag {
    /// The interrupt flag is clear.
    Clear = 0,
    /// The interrupt flag is set.
    Set = (1 << Self::SHIFT),
}

impl InterruptFlag {
    /// Bit shift of the interrupt flag.
    const SHIFT: u32 = 9;
    /// Bit mask of the interrupt flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => InterruptFlag::Clear,
            _ => InterruptFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Direction Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the direction flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionFlag {
    /// The direction flag is clear.
    Clear = 0,
    /// The direction flag is set.
    Set = (1 << Self::SHIFT),
}

impl DirectionFlag {
    /// Bit shift of the direction flag.
    const SHIFT: u32 = 10;
    /// Bit mask of the direction flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => DirectionFlag::Clear,
            _ => DirectionFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Overflow Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the overflow flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowFlag {
    /// The overflow flag is clear.
    Clear = 0,
    /// The overflow flag is set.
    Set = (1 << Self::SHIFT),
}

impl OverflowFlag {
    /// Bit shift of the overflow flag.
    const SHIFT: u32 = 11;
    /// Bit mask of the overflow flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => OverflowFlag::Clear,
            _ => OverflowFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// I/O Privilege Level Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the I/O privilege level of the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPrivilegeLevelFlag {
    IoPrivilegeLevel0 = (ring::PrivilegeLevel::Ring0 as u32) << Self::SHIFT,
    IoPrivilegeLevel1 = (ring::PrivilegeLevel::Ring1 as u32) << Self::SHIFT,
    IoPrivilegeLevel2 = (ring::PrivilegeLevel::Ring2 as u32) << Self::SHIFT,
    IoPrivilegeLevel3 = (ring::PrivilegeLevel::Ring3 as u32) << Self::SHIFT,
}

impl IoPrivilegeLevelFlag {
    /// Bit shift of the I/O privilege level flag.
    const SHIFT: u32 = 12;
    /// Bit mask of the I/O privilege level flag.
    const MASK: u32 = (0b11 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0x00000000 => IoPrivilegeLevelFlag::IoPrivilegeLevel0,
            0x00001000 => IoPrivilegeLevelFlag::IoPrivilegeLevel1,
            0x00002000 => IoPrivilegeLevelFlag::IoPrivilegeLevel2,
            _ => IoPrivilegeLevelFlag::IoPrivilegeLevel3,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Nested Task Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the nested task flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedTaskFlag {
    /// The nested task flag is clear.
    Clear = 0,
    /// The nested task flag is set.
    Set = (1 << Self::SHIFT),
}

impl NestedTaskFlag {
    /// Bit shift of the nested task flag.
    const SHIFT: u32 = 14;
    /// Bit mask of the nested task flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => NestedTaskFlag::Clear,
            _ => NestedTaskFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Resume Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the resume flag in the EFLAGS register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeFlag {
    /// The resume flag is clear.
    Clear = 0,
    /// The resume flag is set.
    Set = (1 << Self::SHIFT),
}

impl ResumeFlag {
    /// Bit shift of the resume flag.
    const SHIFT: u32 = 16;
    /// Bit mask of the resume flag.
    const MASK: u32 = (1 << Self::SHIFT);

    pub fn from_raw_value(value: u32) -> Self {
        match value & Self::MASK {
            0 => ResumeFlag::Clear,
            _ => ResumeFlag::Set,
        }
    }

    pub fn into_raw_value(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Eflags Register
//==================================================================================================

///
/// # Description
///
/// A type that represents the EFLAGS register.
///
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EflagsRegister {
    /// Carry flag.
    pub carry: CarryFlag,
    /// Parity flag.
    pub parity: ParityFlag,
    /// Auxiliary flag.
    pub auxiliary: AuxiliaryFlag,
    /// Zero flag.
    pub zero: ZeroFlag,
    /// Sign flag.
    pub sign: SignFlag,
    /// Trap flag.
    pub trap: TrapFlag,
    /// Interrupt flag.
    pub interrupt: InterruptFlag,
    /// Direction flag.
    pub direction: DirectionFlag,
    /// Overflow flag.
    pub overflow: OverflowFlag,
    /// I/O privilege level flag.
    pub io_privilege_level: IoPrivilegeLevelFlag,
    /// Nested task flag.
    pub nested_task: NestedTaskFlag,
    /// Resume flag.
    pub resume: ResumeFlag,
}

impl EflagsRegister {
    pub fn from_raw_value(value: u32) -> Self {
        Self {
            carry: CarryFlag::from_raw_value(value),
            parity: ParityFlag::from_raw_value(value),
            auxiliary: AuxiliaryFlag::from_raw_value(value),
            zero: ZeroFlag::from_raw_value(value),
            sign: SignFlag::from_raw_value(value),
            trap: TrapFlag::from_raw_value(value),
            interrupt: InterruptFlag::from_raw_value(value),
            direction: DirectionFlag::from_raw_value(value),
            overflow: OverflowFlag::from_raw_value(value),
            io_privilege_level: IoPrivilegeLevelFlag::from_raw_value(value),
            nested_task: NestedTaskFlag::from_raw_value(value),
            resume: ResumeFlag::from_raw_value(value),
        }
    }

    pub fn into_raw_value(self) -> u32 {
        let mut value: u32 = 0;

        value |= self.carry.into_raw_value();
        value |= self.parity.into_raw_value();
        value |= self.auxiliary.into_raw_value();
        value |= self.zero.into_raw_value();
        value |= self.sign.into_raw_value();
        value |= self.trap.into_raw_value();
        value |= self.interrupt.into_raw_value();
        value |= self.direction.into_raw_value();
        value |= self.overflow.into_raw_value();
        value |= self.io_privilege_level.into_raw_value();
        value |= self.nested_task.into_raw_value();
        value |= self.resume.into_raw_value();

        value
    }
}

impl Default for EflagsRegister {
    fn default() -> Self {
        Self {
            carry: CarryFlag::Clear,
            parity: ParityFlag::Clear,
            auxiliary: AuxiliaryFlag::Clear,
            zero: ZeroFlag::Clear,
            sign: SignFlag::Clear,
            trap: TrapFlag::Clear,
            interrupt: InterruptFlag::Clear,
            direction: DirectionFlag::Clear,
            overflow: OverflowFlag::Clear,
            io_privilege_level: IoPrivilegeLevelFlag::IoPrivilegeLevel0,
            nested_task: NestedTaskFlag::Clear,
            resume: ResumeFlag::Clear,
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests is carry flag works.
fn test_carry_flag() -> bool {
    let value: u32 = 0x00000001;

    if CarryFlag::from_raw_value(value) != CarryFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        carry: CarryFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == CarryFlag::Set.into_raw_value()
}

/// Tests if parity flag works.
fn test_parity_flag() -> bool {
    let value: u32 = 0x00000004;

    if ParityFlag::from_raw_value(value) != ParityFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        parity: ParityFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == ParityFlag::Set.into_raw_value()
}

/// Tests if auxiliary flag works.
fn test_auxiliary_flag() -> bool {
    let value: u32 = 0x00000010;

    if AuxiliaryFlag::from_raw_value(value) != AuxiliaryFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        auxiliary: AuxiliaryFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == AuxiliaryFlag::Set.into_raw_value()
}

/// Tests if zero flag works.
fn test_zero_flag() -> bool {
    let value: u32 = 0x00000040;

    if ZeroFlag::from_raw_value(value) != ZeroFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        zero: ZeroFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == ZeroFlag::Set.into_raw_value()
}

/// Tests if sign flag works.
fn test_sign_flag() -> bool {
    let value: u32 = 0x00000080;

    if SignFlag::from_raw_value(value) != SignFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        sign: SignFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == SignFlag::Set.into_raw_value()
}

/// Tests if trap flag works.
fn test_trap_flag() -> bool {
    let value: u32 = 0x00000100;

    if TrapFlag::from_raw_value(value) != TrapFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        trap: TrapFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == TrapFlag::Set.into_raw_value()
}

/// Tests if interrupt flag works.
fn test_interrupt_flag() -> bool {
    let value: u32 = 0x00000200;

    if InterruptFlag::from_raw_value(value) != InterruptFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        interrupt: InterruptFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == InterruptFlag::Set.into_raw_value()
}

/// Tests if direction flag works.
fn test_direction_flag() -> bool {
    let value: u32 = 0x00000400;

    if DirectionFlag::from_raw_value(value) != DirectionFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        direction: DirectionFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == DirectionFlag::Set.into_raw_value()
}

/// Tests if overflow flag works.
fn test_overflow_flag() -> bool {
    let value: u32 = 0x00000800;

    if OverflowFlag::from_raw_value(value) != OverflowFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        overflow: OverflowFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == OverflowFlag::Set.into_raw_value()
}

/// Tests if I/O privilege level flag 0 works.
fn test_io_privilege_level_flag_0() -> bool {
    let value: u32 = 0x00000000;

    if IoPrivilegeLevelFlag::from_raw_value(value) != IoPrivilegeLevelFlag::IoPrivilegeLevel0 {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        io_privilege_level: IoPrivilegeLevelFlag::IoPrivilegeLevel0,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == IoPrivilegeLevelFlag::IoPrivilegeLevel0.into_raw_value()
}

/// Tests if I/O privilege level flag 1 works.
fn test_io_privilege_level_flag_1() -> bool {
    let value: u32 = 0x00001000;

    if IoPrivilegeLevelFlag::from_raw_value(value) != IoPrivilegeLevelFlag::IoPrivilegeLevel1 {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        io_privilege_level: IoPrivilegeLevelFlag::IoPrivilegeLevel1,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == IoPrivilegeLevelFlag::IoPrivilegeLevel1.into_raw_value()
}

/// Tests if I/O privilege level flag 2 works.
fn test_io_privilege_level_flag_2() -> bool {
    let value: u32 = 0x00002000;

    if IoPrivilegeLevelFlag::from_raw_value(value) != IoPrivilegeLevelFlag::IoPrivilegeLevel2 {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        io_privilege_level: IoPrivilegeLevelFlag::IoPrivilegeLevel2,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == IoPrivilegeLevelFlag::IoPrivilegeLevel2.into_raw_value()
}

/// Tests if I/O privilege level flag 3 works.
fn test_io_privilege_level_flag_3() -> bool {
    let value: u32 = 0x00003000;

    if IoPrivilegeLevelFlag::from_raw_value(value) != IoPrivilegeLevelFlag::IoPrivilegeLevel3 {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        io_privilege_level: IoPrivilegeLevelFlag::IoPrivilegeLevel3,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == IoPrivilegeLevelFlag::IoPrivilegeLevel3.into_raw_value()
}

/// Tests if nested task flag works.
fn test_nested_task_flag() -> bool {
    let value: u32 = 0x00004000;

    if NestedTaskFlag::from_raw_value(value) != NestedTaskFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        nested_task: NestedTaskFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == NestedTaskFlag::Set.into_raw_value()
}

/// Tests if resume flag works.
fn test_resume_flag() -> bool {
    let value: u32 = 0x00010000;

    if ResumeFlag::from_raw_value(value) != ResumeFlag::Set {
        return false;
    }

    let eflags: EflagsRegister = EflagsRegister {
        resume: ResumeFlag::Set,
        ..EflagsRegister::default()
    };

    if eflags != EflagsRegister::from_raw_value(value) {
        return false;
    }

    if eflags.into_raw_value() != value {
        return false;
    }

    eflags.into_raw_value() == ResumeFlag::Set.into_raw_value()
}

// Runs all tests for this module.
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= test_carry_flag();
    passed &= test_parity_flag();
    passed &= test_auxiliary_flag();
    passed &= test_zero_flag();
    passed &= test_sign_flag();
    passed &= test_trap_flag();
    passed &= test_interrupt_flag();
    passed &= test_direction_flag();
    passed &= test_overflow_flag();
    passed &= test_io_privilege_level_flag_0();
    passed &= test_io_privilege_level_flag_1();
    passed &= test_io_privilege_level_flag_2();
    passed &= test_io_privilege_level_flag_3();
    passed &= test_nested_task_flag();
    passed &= test_resume_flag();

    passed
}
