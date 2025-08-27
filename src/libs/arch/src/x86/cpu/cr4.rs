// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

// Error
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
compile_error!("Unsupported architecture");

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch::asm;

//==================================================================================================
// Virtual Mode Extensions Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the virtual mode extensions flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualModeExtensionsFlag {
    /// Virtual mode extensions are disabled.
    Disabled = 0,
    /// Virtual mode extensions are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl VirtualModeExtensionsFlag {
    /// Bit shift of the virtual mode extensions flag.
    const SHIFT: u32 = 0;
    /// Bit mask of the virtual mode extensions flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a virtual mode extensions flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The virtual mode extensions flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => VirtualModeExtensionsFlag::Disabled,
            _ => VirtualModeExtensionsFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the virtual mode extensions flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the virtual mode extensions flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Protected Mode Virtual Interrupts Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the protected mode virtual interrupts flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectedModeVirtualInterruptsFlag {
    /// Protected mode virtual interrupts are disabled.
    Disabled = 0,
    /// Protected mode virtual interrupts are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl ProtectedModeVirtualInterruptsFlag {
    /// Bit shift of the protected mode virtual interrupts flag.
    const SHIFT: u32 = 1;
    /// Bit mask of the protected mode virtual interrupts flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a protected mode virtual interrupts flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The protected mode virtual interrupts flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => ProtectedModeVirtualInterruptsFlag::Disabled,
            _ => ProtectedModeVirtualInterruptsFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the protected mode virtual interrupts flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the protected mode virtual interrupts flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Time Stamp Disable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the time stamp disable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeStampDisableFlag {
    /// Time stamp counter can be executed at any privilege level.
    Enabled = 0,
    /// Time stamp counter can only be executed at privilege level 0.
    Disabled = (1 << Self::SHIFT),
}

impl TimeStampDisableFlag {
    /// Bit shift of the time stamp disable flag.
    const SHIFT: u32 = 2;
    /// Bit mask of the time stamp disable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a time stamp disable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The time stamp disable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => TimeStampDisableFlag::Enabled,
            _ => TimeStampDisableFlag::Disabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the time stamp disable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the time stamp disable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Debugging Extensions Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the debugging extensions flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebuggingExtensionsFlag {
    /// Debugging extensions are disabled.
    Disabled = 0,
    /// Debugging extensions are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl DebuggingExtensionsFlag {
    /// Bit shift of the debugging extensions flag.
    const SHIFT: u32 = 3;
    /// Bit mask of the debugging extensions flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a debugging extensions flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The debugging extensions flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => DebuggingExtensionsFlag::Disabled,
            _ => DebuggingExtensionsFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the debugging extensions flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the debugging extensions flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Page Size Extensions Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the page size extensions flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSizeExtensionsFlag {
    /// Page size extensions are disabled (4KB pages only).
    Disabled = 0,
    /// Page size extensions are enabled (4MB pages supported).
    Enabled = (1 << Self::SHIFT),
}

impl PageSizeExtensionsFlag {
    /// Bit shift of the page size extensions flag.
    const SHIFT: u32 = 4;
    /// Bit mask of the page size extensions flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a page size extensions flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The page size extensions flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PageSizeExtensionsFlag::Disabled,
            _ => PageSizeExtensionsFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the page size extensions flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the page size extensions flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Physical Address Extension Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the physical address extension flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalAddressExtensionFlag {
    /// Physical address extension is disabled.
    Disabled = 0,
    /// Physical address extension is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl PhysicalAddressExtensionFlag {
    /// Bit shift of the physical address extension flag.
    const SHIFT: u32 = 5;
    /// Bit mask of the physical address extension flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a physical address extension flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The physical address extension flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PhysicalAddressExtensionFlag::Disabled,
            _ => PhysicalAddressExtensionFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the physical address extension flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the physical address extension flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Machine Check Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the machine check enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineCheckEnableFlag {
    /// Machine check exceptions are disabled.
    Disabled = 0,
    /// Machine check exceptions are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl MachineCheckEnableFlag {
    /// Bit shift of the machine check enable flag.
    const SHIFT: u32 = 6;
    /// Bit mask of the machine check enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a machine check enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The machine check enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => MachineCheckEnableFlag::Disabled,
            _ => MachineCheckEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the machine check enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the machine check enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Page Global Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the page global enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageGlobalEnableFlag {
    /// Global pages are disabled.
    Disabled = 0,
    /// Global pages are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl PageGlobalEnableFlag {
    /// Bit shift of the page global enable flag.
    const SHIFT: u32 = 7;
    /// Bit mask of the page global enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a page global enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The page global enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PageGlobalEnableFlag::Disabled,
            _ => PageGlobalEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the page global enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the page global enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Performance Counter Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the performance counter enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceCounterEnableFlag {
    /// Performance counters are restricted to privilege level 0.
    Disabled = 0,
    /// Performance counters are enabled at all privilege levels.
    Enabled = (1 << Self::SHIFT),
}

impl PerformanceCounterEnableFlag {
    /// Bit shift of the performance counter enable flag.
    const SHIFT: u32 = 8;
    /// Bit mask of the performance counter enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a performance counter enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The performance counter enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PerformanceCounterEnableFlag::Disabled,
            _ => PerformanceCounterEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the performance counter enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the performance counter enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Operating System Support for FXSAVE and FXRSTOR Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the OS support for FXSAVE and FXRSTOR flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsFxsaveFlag {
    /// OS does not support FXSAVE and FXRSTOR instructions.
    Disabled = 0,
    /// OS supports FXSAVE and FXRSTOR instructions.
    Enabled = (1 << Self::SHIFT),
}

impl OsFxsaveFlag {
    /// Bit shift of the OS FXSAVE flag.
    const SHIFT: u32 = 9;
    /// Bit mask of the OS FXSAVE flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an OS FXSAVE flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The OS FXSAVE flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => OsFxsaveFlag::Disabled,
            _ => OsFxsaveFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the OS FXSAVE flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the OS FXSAVE flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Operating System Support for Unmasked SIMD Floating Point Exceptions Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the OS support for unmasked SIMD floating point exceptions flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsSimdExceptionFlag {
    /// OS does not support unmasked SIMD floating point exceptions.
    Disabled = 0,
    /// OS supports unmasked SIMD floating point exceptions.
    Enabled = (1 << Self::SHIFT),
}

impl OsSimdExceptionFlag {
    /// Bit shift of the OS SIMD exception flag.
    const SHIFT: u32 = 10;
    /// Bit mask of the OS SIMD exception flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an OS SIMD exception flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The OS SIMD exception flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => OsSimdExceptionFlag::Disabled,
            _ => OsSimdExceptionFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the OS SIMD exception flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the OS SIMD exception flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// User Mode Instruction Prevention Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the user mode instruction prevention flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserModeInstructionPreventionFlag {
    /// User mode instruction prevention is disabled.
    Disabled = 0,
    /// User mode instruction prevention is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl UserModeInstructionPreventionFlag {
    /// Bit shift of the user mode instruction prevention flag.
    const SHIFT: u32 = 11;
    /// Bit mask of the user mode instruction prevention flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a user mode instruction prevention flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The user mode instruction prevention flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => UserModeInstructionPreventionFlag::Disabled,
            _ => UserModeInstructionPreventionFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the user mode instruction prevention flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the user mode instruction prevention flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// 57-bit Linear Address Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the 57-bit linear address flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearAddress57Flag {
    /// 4-level paging (48-bit linear addresses).
    Disabled = 0,
    /// 5-level paging (57-bit linear addresses).
    Enabled = (1 << Self::SHIFT),
}

impl LinearAddress57Flag {
    /// Bit shift of the 57-bit linear address flag.
    const SHIFT: u32 = 12;
    /// Bit mask of the 57-bit linear address flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a 57-bit linear address flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The 57-bit linear address flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => LinearAddress57Flag::Disabled,
            _ => LinearAddress57Flag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the 57-bit linear address flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the 57-bit linear address flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Virtual Machine Extensions Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the virtual machine extensions enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualMachineExtensionsEnableFlag {
    /// Virtual machine extensions are disabled.
    Disabled = 0,
    /// Virtual machine extensions are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl VirtualMachineExtensionsEnableFlag {
    /// Bit shift of the virtual machine extensions enable flag.
    const SHIFT: u32 = 13;
    /// Bit mask of the virtual machine extensions enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a virtual machine extensions enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The virtual machine extensions enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => VirtualMachineExtensionsEnableFlag::Disabled,
            _ => VirtualMachineExtensionsEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the virtual machine extensions enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the virtual machine extensions enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Safer Mode Extensions Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the safer mode extensions enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaferModeExtensionsEnableFlag {
    /// Safer mode extensions are disabled.
    Disabled = 0,
    /// Safer mode extensions are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl SaferModeExtensionsEnableFlag {
    /// Bit shift of the safer mode extensions enable flag.
    const SHIFT: u32 = 14;
    /// Bit mask of the safer mode extensions enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a safer mode extensions enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The safer mode extensions enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => SaferModeExtensionsEnableFlag::Disabled,
            _ => SaferModeExtensionsEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the safer mode extensions enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the safer mode extensions enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// FS/GS Base Instructions Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the FS/GS base instructions enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsGsBaseInstructionsEnableFlag {
    /// RDFSBASE, RDGSBASE, WRFSBASE, and WRGSBASE instructions are disabled.
    Disabled = 0,
    /// RDFSBASE, RDGSBASE, WRFSBASE, and WRGSBASE instructions are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl FsGsBaseInstructionsEnableFlag {
    /// Bit shift of the FS/GS base instructions enable flag.
    const SHIFT: u32 = 16;
    /// Bit mask of the FS/GS base instructions enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a FS/GS base instructions enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The FS/GS base instructions enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => FsGsBaseInstructionsEnableFlag::Disabled,
            _ => FsGsBaseInstructionsEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the FS/GS base instructions enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the FS/GS base instructions enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Process Context Identifier Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the process context identifier enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessContextIdentifierEnableFlag {
    /// Process context identifiers are disabled.
    Disabled = 0,
    /// Process context identifiers are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl ProcessContextIdentifierEnableFlag {
    /// Bit shift of the process context identifier enable flag.
    const SHIFT: u32 = 17;
    /// Bit mask of the process context identifier enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a process context identifier enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The process context identifier enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => ProcessContextIdentifierEnableFlag::Disabled,
            _ => ProcessContextIdentifierEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the process context identifier enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the process context identifier enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// XSAVE and Processor Extended States Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the XSAVE and processor extended states enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XsaveEnableFlag {
    /// XSAVE and processor extended states are disabled.
    Disabled = 0,
    /// XSAVE and processor extended states are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl XsaveEnableFlag {
    /// Bit shift of the XSAVE enable flag.
    const SHIFT: u32 = 18;
    /// Bit mask of the XSAVE enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an XSAVE enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The XSAVE enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => XsaveEnableFlag::Disabled,
            _ => XsaveEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the XSAVE enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the XSAVE enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Supervisor Mode Execution Protection Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the supervisor mode execution protection enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorModeExecutionProtectionEnableFlag {
    /// Supervisor mode execution protection is disabled.
    Disabled = 0,
    /// Supervisor mode execution protection is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl SupervisorModeExecutionProtectionEnableFlag {
    /// Bit shift of the supervisor mode execution protection enable flag.
    const SHIFT: u32 = 20;
    /// Bit mask of the supervisor mode execution protection enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a supervisor mode execution protection enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The supervisor mode execution protection enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => SupervisorModeExecutionProtectionEnableFlag::Disabled,
            _ => SupervisorModeExecutionProtectionEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the supervisor mode execution protection enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the supervisor mode execution protection enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Supervisor Mode Access Prevention Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the supervisor mode access prevention enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorModeAccessPreventionEnableFlag {
    /// Supervisor mode access prevention is disabled.
    Disabled = 0,
    /// Supervisor mode access prevention is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl SupervisorModeAccessPreventionEnableFlag {
    /// Bit shift of the supervisor mode access prevention enable flag.
    const SHIFT: u32 = 21;
    /// Bit mask of the supervisor mode access prevention enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a supervisor mode access prevention enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The supervisor mode access prevention enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => SupervisorModeAccessPreventionEnableFlag::Disabled,
            _ => SupervisorModeAccessPreventionEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the supervisor mode access prevention enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the supervisor mode access prevention enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Protection Key Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the protection key enable flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionKeyEnableFlag {
    /// Protection keys are disabled.
    Disabled = 0,
    /// Protection keys are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl ProtectionKeyEnableFlag {
    /// Bit shift of the protection key enable flag.
    const SHIFT: u32 = 22;
    /// Bit mask of the protection key enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a protection key enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The protection key enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => ProtectionKeyEnableFlag::Disabled,
            _ => ProtectionKeyEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the protection key enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the protection key enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Control-flow Enforcement Technology Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the control-flow enforcement technology flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlFlowEnforcementTechnologyFlag {
    /// Control-flow enforcement technology is disabled.
    Disabled = 0,
    /// Control-flow enforcement technology is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl ControlFlowEnforcementTechnologyFlag {
    /// Bit shift of the control-flow enforcement technology flag.
    const SHIFT: u32 = 23;
    /// Bit mask of the control-flow enforcement technology flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a control-flow enforcement technology flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The control-flow enforcement technology flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => ControlFlowEnforcementTechnologyFlag::Disabled,
            _ => ControlFlowEnforcementTechnologyFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the control-flow enforcement technology flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the control-flow enforcement technology flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Protection Keys for Supervisor-Mode Pages Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the protection keys for supervisor-mode pages flag in the CR4 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionKeysForSupervisorModeFlag {
    /// Protection keys for supervisor-mode pages are disabled.
    Disabled = 0,
    /// Protection keys for supervisor-mode pages are enabled.
    Enabled = (1 << Self::SHIFT),
}

impl ProtectionKeysForSupervisorModeFlag {
    /// Bit shift of the protection keys for supervisor-mode pages flag.
    const SHIFT: u32 = 24;
    /// Bit mask of the protection keys for supervisor-mode pages flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a protection keys for supervisor-mode pages flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The protection keys for supervisor-mode pages flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => ProtectionKeysForSupervisorModeFlag::Disabled,
            _ => ProtectionKeysForSupervisorModeFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the protection keys for supervisor-mode pages flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the protection keys for supervisor-mode pages flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Control Register Four (CR4)
//==================================================================================================

///
/// # Description
///
/// A type that represents the CR4 register.
///
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cr4Register {
    /// Virtual mode extensions flag.
    pub virtual_mode_extensions: VirtualModeExtensionsFlag,
    /// Protected mode virtual interrupts flag.
    pub protected_mode_virtual_interrupts: ProtectedModeVirtualInterruptsFlag,
    /// Time stamp disable flag.
    pub time_stamp_disable: TimeStampDisableFlag,
    /// Debugging extensions flag.
    pub debugging_extensions: DebuggingExtensionsFlag,
    /// Page size extensions flag.
    pub page_size_extensions: PageSizeExtensionsFlag,
    /// Physical address extension flag.
    pub physical_address_extension: PhysicalAddressExtensionFlag,
    /// Machine check enable flag.
    pub machine_check_enable: MachineCheckEnableFlag,
    /// Page global enable flag.
    pub page_global_enable: PageGlobalEnableFlag,
    /// Performance counter enable flag.
    pub performance_counter_enable: PerformanceCounterEnableFlag,
    /// OS support for FXSAVE and FXRSTOR flag.
    pub os_fxsave: OsFxsaveFlag,
    /// OS support for unmasked SIMD floating point exceptions flag.
    pub os_simd_exception: OsSimdExceptionFlag,
    /// User mode instruction prevention flag.
    pub user_mode_instruction_prevention: UserModeInstructionPreventionFlag,
    /// 57-bit linear address flag.
    pub linear_address_57: LinearAddress57Flag,
    /// Virtual machine extensions enable flag.
    pub virtual_machine_extensions_enable: VirtualMachineExtensionsEnableFlag,
    /// Safer mode extensions enable flag.
    pub safer_mode_extensions_enable: SaferModeExtensionsEnableFlag,
    /// FS/GS base instructions enable flag.
    pub fs_gs_base_instructions_enable: FsGsBaseInstructionsEnableFlag,
    /// Process context identifier enable flag.
    pub process_context_identifier_enable: ProcessContextIdentifierEnableFlag,
    /// XSAVE and processor extended states enable flag.
    pub xsave_enable: XsaveEnableFlag,
    /// Supervisor mode execution protection enable flag.
    pub supervisor_mode_execution_protection_enable: SupervisorModeExecutionProtectionEnableFlag,
    /// Supervisor mode access prevention enable flag.
    pub supervisor_mode_access_prevention_enable: SupervisorModeAccessPreventionEnableFlag,
    /// Protection key enable flag.
    pub protection_key_enable: ProtectionKeyEnableFlag,
    /// Control-flow enforcement technology flag.
    pub control_flow_enforcement_technology: ControlFlowEnforcementTechnologyFlag,
    /// Protection keys for supervisor-mode pages flag.
    pub protection_keys_for_supervisor_mode: ProtectionKeysForSupervisorModeFlag,
}

impl Cr4Register {
    ///
    /// # Description
    ///
    /// Creates a CR4 register from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the register state.
    ///
    /// # Return Value
    ///
    /// The CR4 register with all flags extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        Self {
            virtual_mode_extensions: VirtualModeExtensionsFlag::from_u32(value),
            protected_mode_virtual_interrupts: ProtectedModeVirtualInterruptsFlag::from_u32(value),
            time_stamp_disable: TimeStampDisableFlag::from_u32(value),
            debugging_extensions: DebuggingExtensionsFlag::from_u32(value),
            page_size_extensions: PageSizeExtensionsFlag::from_u32(value),
            physical_address_extension: PhysicalAddressExtensionFlag::from_u32(value),
            machine_check_enable: MachineCheckEnableFlag::from_u32(value),
            page_global_enable: PageGlobalEnableFlag::from_u32(value),
            performance_counter_enable: PerformanceCounterEnableFlag::from_u32(value),
            os_fxsave: OsFxsaveFlag::from_u32(value),
            os_simd_exception: OsSimdExceptionFlag::from_u32(value),
            user_mode_instruction_prevention: UserModeInstructionPreventionFlag::from_u32(value),
            linear_address_57: LinearAddress57Flag::from_u32(value),
            virtual_machine_extensions_enable: VirtualMachineExtensionsEnableFlag::from_u32(value),
            safer_mode_extensions_enable: SaferModeExtensionsEnableFlag::from_u32(value),
            fs_gs_base_instructions_enable: FsGsBaseInstructionsEnableFlag::from_u32(value),
            process_context_identifier_enable: ProcessContextIdentifierEnableFlag::from_u32(value),
            xsave_enable: XsaveEnableFlag::from_u32(value),
            supervisor_mode_execution_protection_enable:
                SupervisorModeExecutionProtectionEnableFlag::from_u32(value),
            supervisor_mode_access_prevention_enable:
                SupervisorModeAccessPreventionEnableFlag::from_u32(value),
            protection_key_enable: ProtectionKeyEnableFlag::from_u32(value),
            control_flow_enforcement_technology: ControlFlowEnforcementTechnologyFlag::from_u32(
                value,
            ),
            protection_keys_for_supervisor_mode: ProtectionKeysForSupervisorModeFlag::from_u32(
                value,
            ),
        }
    }

    ///
    /// # Description
    ///
    /// Converts the CR4 register to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the CR4 register with all flags combined.
    ///
    fn into_u32(self) -> u32 {
        let mut value: u32 = 0;

        value |= self.virtual_mode_extensions.into_u32();
        value |= self.protected_mode_virtual_interrupts.into_u32();
        value |= self.time_stamp_disable.into_u32();
        value |= self.debugging_extensions.into_u32();
        value |= self.page_size_extensions.into_u32();
        value |= self.physical_address_extension.into_u32();
        value |= self.machine_check_enable.into_u32();
        value |= self.page_global_enable.into_u32();
        value |= self.performance_counter_enable.into_u32();
        value |= self.os_fxsave.into_u32();
        value |= self.os_simd_exception.into_u32();
        value |= self.user_mode_instruction_prevention.into_u32();
        value |= self.linear_address_57.into_u32();
        value |= self.virtual_machine_extensions_enable.into_u32();
        value |= self.safer_mode_extensions_enable.into_u32();
        value |= self.fs_gs_base_instructions_enable.into_u32();
        value |= self.process_context_identifier_enable.into_u32();
        value |= self.xsave_enable.into_u32();
        value |= self.supervisor_mode_execution_protection_enable.into_u32();
        value |= self.supervisor_mode_access_prevention_enable.into_u32();
        value |= self.protection_key_enable.into_u32();
        value |= self.control_flow_enforcement_technology.into_u32();
        value |= self.protection_keys_for_supervisor_mode.into_u32();

        value
    }

    ///
    /// # Description
    ///
    /// Reads the value of the CR4 register.
    ///
    /// # Return Value
    ///
    /// The value of the CR4 register.
    ///
    /// # Safety
    ///
    /// It is unsafe to call this function because it executes privileged instructions.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - The caller runs at processor privilege level 0.
    ///
    pub unsafe fn read() -> Self {
        #[cfg(target_arch = "x86")]
        {
            let value: u32;
            asm!("mov {0:e}, cr4", out(reg) value);
            Self::from_u32(value)
        }

        #[cfg(target_arch = "x86_64")]
        {
            let value: u64;
            asm!("mov {0:r}, cr4", out(reg) value);
            let value32: u32 = (value & 0xffff_ffff) as u32;
            Self::from_u32(value32)
        }
    }

    ///
    /// # Description
    ///
    /// Writes a value to the CR4 register.
    ///
    /// # Safety
    ///
    /// It is unsafe to call this function because it executes privileged instructions.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - The caller runs at processor privilege level 0.
    ///
    pub unsafe fn write(&self) {
        #[cfg(target_arch = "x86")]
        {
            let value: u32 = self.into_u32();
            asm!("mov cr4, {0:e}", in(reg) value);
        }

        #[cfg(target_arch = "x86_64")]
        {
            let value64: u64 = self.into_u32() as u64;
            asm!("mov cr4, {0:r}", in(reg) value64);
        }
    }
}

impl Default for Cr4Register {
    fn default() -> Self {
        Self {
            virtual_mode_extensions: VirtualModeExtensionsFlag::Disabled,
            protected_mode_virtual_interrupts: ProtectedModeVirtualInterruptsFlag::Disabled,
            time_stamp_disable: TimeStampDisableFlag::Enabled,
            debugging_extensions: DebuggingExtensionsFlag::Disabled,
            page_size_extensions: PageSizeExtensionsFlag::Disabled,
            physical_address_extension: PhysicalAddressExtensionFlag::Disabled,
            machine_check_enable: MachineCheckEnableFlag::Disabled,
            page_global_enable: PageGlobalEnableFlag::Disabled,
            performance_counter_enable: PerformanceCounterEnableFlag::Disabled,
            os_fxsave: OsFxsaveFlag::Disabled,
            os_simd_exception: OsSimdExceptionFlag::Disabled,
            user_mode_instruction_prevention: UserModeInstructionPreventionFlag::Disabled,
            linear_address_57: LinearAddress57Flag::Disabled,
            virtual_machine_extensions_enable: VirtualMachineExtensionsEnableFlag::Disabled,
            safer_mode_extensions_enable: SaferModeExtensionsEnableFlag::Disabled,
            fs_gs_base_instructions_enable: FsGsBaseInstructionsEnableFlag::Disabled,
            process_context_identifier_enable: ProcessContextIdentifierEnableFlag::Disabled,
            xsave_enable: XsaveEnableFlag::Disabled,
            supervisor_mode_execution_protection_enable:
                SupervisorModeExecutionProtectionEnableFlag::Disabled,
            supervisor_mode_access_prevention_enable:
                SupervisorModeAccessPreventionEnableFlag::Disabled,
            protection_key_enable: ProtectionKeyEnableFlag::Disabled,
            control_flow_enforcement_technology: ControlFlowEnforcementTechnologyFlag::Disabled,
            protection_keys_for_supervisor_mode: ProtectionKeysForSupervisorModeFlag::Disabled,
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests if virtual mode extensions flag works.
fn test_virtual_mode_extensions_flag() -> bool {
    let value: u32 = 0x00000001;

    if VirtualModeExtensionsFlag::from_u32(value) != VirtualModeExtensionsFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        virtual_mode_extensions: VirtualModeExtensionsFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == VirtualModeExtensionsFlag::Enabled.into_u32()
}

/// Tests if protected mode virtual interrupts flag works.
fn test_protected_mode_virtual_interrupts_flag() -> bool {
    let value: u32 = 0x00000002;

    if ProtectedModeVirtualInterruptsFlag::from_u32(value)
        != ProtectedModeVirtualInterruptsFlag::Enabled
    {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        protected_mode_virtual_interrupts: ProtectedModeVirtualInterruptsFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == ProtectedModeVirtualInterruptsFlag::Enabled.into_u32()
}

/// Tests if time stamp disable flag works.
fn test_time_stamp_disable_flag() -> bool {
    let value: u32 = 0x00000004;

    if TimeStampDisableFlag::from_u32(value) != TimeStampDisableFlag::Disabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        time_stamp_disable: TimeStampDisableFlag::Disabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == TimeStampDisableFlag::Disabled.into_u32()
}

/// Tests if debugging extensions flag works.
fn test_debugging_extensions_flag() -> bool {
    let value: u32 = 0x00000008;

    if DebuggingExtensionsFlag::from_u32(value) != DebuggingExtensionsFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        debugging_extensions: DebuggingExtensionsFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == DebuggingExtensionsFlag::Enabled.into_u32()
}

/// Tests if page size extensions flag works.
fn test_page_size_extensions_flag() -> bool {
    let value: u32 = 0x00000010;

    if PageSizeExtensionsFlag::from_u32(value) != PageSizeExtensionsFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        page_size_extensions: PageSizeExtensionsFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == PageSizeExtensionsFlag::Enabled.into_u32()
}

/// Tests if physical address extension flag works.
fn test_physical_address_extension_flag() -> bool {
    let value: u32 = 0x00000020;

    if PhysicalAddressExtensionFlag::from_u32(value) != PhysicalAddressExtensionFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        physical_address_extension: PhysicalAddressExtensionFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == PhysicalAddressExtensionFlag::Enabled.into_u32()
}

/// Tests if machine check enable flag works.
fn test_machine_check_enable_flag() -> bool {
    let value: u32 = 0x00000040;

    if MachineCheckEnableFlag::from_u32(value) != MachineCheckEnableFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        machine_check_enable: MachineCheckEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == MachineCheckEnableFlag::Enabled.into_u32()
}

/// Tests if page global enable flag works.
fn test_page_global_enable_flag() -> bool {
    let value: u32 = 0x00000080;

    if PageGlobalEnableFlag::from_u32(value) != PageGlobalEnableFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        page_global_enable: PageGlobalEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == PageGlobalEnableFlag::Enabled.into_u32()
}

/// Tests if performance counter enable flag works.
fn test_performance_counter_enable_flag() -> bool {
    let value: u32 = 0x00000100;

    if PerformanceCounterEnableFlag::from_u32(value) != PerformanceCounterEnableFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        performance_counter_enable: PerformanceCounterEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == PerformanceCounterEnableFlag::Enabled.into_u32()
}

/// Tests if OS FXSAVE flag works.
fn test_os_fxsave_flag() -> bool {
    let value: u32 = 0x00000200;

    if OsFxsaveFlag::from_u32(value) != OsFxsaveFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        os_fxsave: OsFxsaveFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == OsFxsaveFlag::Enabled.into_u32()
}

/// Tests if OS SIMD exception flag works.
fn test_os_simd_exception_flag() -> bool {
    let value: u32 = 0x00000400;

    if OsSimdExceptionFlag::from_u32(value) != OsSimdExceptionFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        os_simd_exception: OsSimdExceptionFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == OsSimdExceptionFlag::Enabled.into_u32()
}

/// Tests if user mode instruction prevention flag works.
fn test_user_mode_instruction_prevention_flag() -> bool {
    let value: u32 = 0x00000800;

    if UserModeInstructionPreventionFlag::from_u32(value)
        != UserModeInstructionPreventionFlag::Enabled
    {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        user_mode_instruction_prevention: UserModeInstructionPreventionFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == UserModeInstructionPreventionFlag::Enabled.into_u32()
}

/// Tests if 57-bit linear address flag works.
fn test_linear_address_57_flag() -> bool {
    let value: u32 = 0x00001000;

    if LinearAddress57Flag::from_u32(value) != LinearAddress57Flag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        linear_address_57: LinearAddress57Flag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == LinearAddress57Flag::Enabled.into_u32()
}

/// Tests if virtual machine extensions enable flag works.
fn test_virtual_machine_extensions_enable_flag() -> bool {
    let value: u32 = 0x00002000;

    if VirtualMachineExtensionsEnableFlag::from_u32(value)
        != VirtualMachineExtensionsEnableFlag::Enabled
    {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        virtual_machine_extensions_enable: VirtualMachineExtensionsEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == VirtualMachineExtensionsEnableFlag::Enabled.into_u32()
}

/// Tests if safer mode extensions enable flag works.
fn test_safer_mode_extensions_enable_flag() -> bool {
    let value: u32 = 0x00004000;

    if SaferModeExtensionsEnableFlag::from_u32(value) != SaferModeExtensionsEnableFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        safer_mode_extensions_enable: SaferModeExtensionsEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == SaferModeExtensionsEnableFlag::Enabled.into_u32()
}

/// Tests if FS/GS base instructions enable flag works.
fn test_fs_gs_base_instructions_enable_flag() -> bool {
    let value: u32 = 0x00010000;

    if FsGsBaseInstructionsEnableFlag::from_u32(value) != FsGsBaseInstructionsEnableFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        fs_gs_base_instructions_enable: FsGsBaseInstructionsEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == FsGsBaseInstructionsEnableFlag::Enabled.into_u32()
}

/// Tests if process context identifier enable flag works.
fn test_process_context_identifier_enable_flag() -> bool {
    let value: u32 = 0x00020000;

    if ProcessContextIdentifierEnableFlag::from_u32(value)
        != ProcessContextIdentifierEnableFlag::Enabled
    {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        process_context_identifier_enable: ProcessContextIdentifierEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == ProcessContextIdentifierEnableFlag::Enabled.into_u32()
}

/// Tests if XSAVE enable flag works.
fn test_xsave_enable_flag() -> bool {
    let value: u32 = 0x00040000;

    if XsaveEnableFlag::from_u32(value) != XsaveEnableFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        xsave_enable: XsaveEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == XsaveEnableFlag::Enabled.into_u32()
}

/// Tests if supervisor mode execution protection enable flag works.
fn test_supervisor_mode_execution_protection_enable_flag() -> bool {
    let value: u32 = 0x00100000;

    if SupervisorModeExecutionProtectionEnableFlag::from_u32(value)
        != SupervisorModeExecutionProtectionEnableFlag::Enabled
    {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        supervisor_mode_execution_protection_enable:
            SupervisorModeExecutionProtectionEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == SupervisorModeExecutionProtectionEnableFlag::Enabled.into_u32()
}

/// Tests if supervisor mode access prevention enable flag works.
fn test_supervisor_mode_access_prevention_enable_flag() -> bool {
    let value: u32 = 0x00200000;

    if SupervisorModeAccessPreventionEnableFlag::from_u32(value)
        != SupervisorModeAccessPreventionEnableFlag::Enabled
    {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        supervisor_mode_access_prevention_enable: SupervisorModeAccessPreventionEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == SupervisorModeAccessPreventionEnableFlag::Enabled.into_u32()
}

/// Tests if protection key enable flag works.
fn test_protection_key_enable_flag() -> bool {
    let value: u32 = 0x00400000;

    if ProtectionKeyEnableFlag::from_u32(value) != ProtectionKeyEnableFlag::Enabled {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        protection_key_enable: ProtectionKeyEnableFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == ProtectionKeyEnableFlag::Enabled.into_u32()
}

/// Tests if control-flow enforcement technology flag works.
fn test_control_flow_enforcement_technology_flag() -> bool {
    let value: u32 = 0x00800000;

    if ControlFlowEnforcementTechnologyFlag::from_u32(value)
        != ControlFlowEnforcementTechnologyFlag::Enabled
    {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        control_flow_enforcement_technology: ControlFlowEnforcementTechnologyFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == ControlFlowEnforcementTechnologyFlag::Enabled.into_u32()
}

/// Tests if protection keys for supervisor-mode pages flag works.
fn test_protection_keys_for_supervisor_mode_flag() -> bool {
    let value: u32 = 0x01000000;

    if ProtectionKeysForSupervisorModeFlag::from_u32(value)
        != ProtectionKeysForSupervisorModeFlag::Enabled
    {
        return false;
    }

    let cr4: Cr4Register = Cr4Register {
        protection_keys_for_supervisor_mode: ProtectionKeysForSupervisorModeFlag::Enabled,
        ..Cr4Register::default()
    };

    if cr4 != Cr4Register::from_u32(value) {
        return false;
    }

    if cr4.into_u32() != value {
        return false;
    }

    cr4.into_u32() == ProtectionKeysForSupervisorModeFlag::Enabled.into_u32()
}

// Runs all tests for this module.
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= test_virtual_mode_extensions_flag();
    passed &= test_protected_mode_virtual_interrupts_flag();
    passed &= test_time_stamp_disable_flag();
    passed &= test_debugging_extensions_flag();
    passed &= test_page_size_extensions_flag();
    passed &= test_physical_address_extension_flag();
    passed &= test_machine_check_enable_flag();
    passed &= test_page_global_enable_flag();
    passed &= test_performance_counter_enable_flag();
    passed &= test_os_fxsave_flag();
    passed &= test_os_simd_exception_flag();
    passed &= test_user_mode_instruction_prevention_flag();
    passed &= test_linear_address_57_flag();
    passed &= test_virtual_machine_extensions_enable_flag();
    passed &= test_safer_mode_extensions_enable_flag();
    passed &= test_fs_gs_base_instructions_enable_flag();
    passed &= test_process_context_identifier_enable_flag();
    passed &= test_xsave_enable_flag();
    passed &= test_supervisor_mode_execution_protection_enable_flag();
    passed &= test_supervisor_mode_access_prevention_enable_flag();
    passed &= test_protection_key_enable_flag();
    passed &= test_control_flow_enforcement_technology_flag();
    passed &= test_protection_keys_for_supervisor_mode_flag();

    passed
}
