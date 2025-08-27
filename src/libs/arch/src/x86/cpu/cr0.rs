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
// Protection Enable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the protection enable flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionEnableFlag {
    /// Protection mode is disabled (real mode).
    Disabled = 0,
    /// Protection mode is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl ProtectionEnableFlag {
    /// Bit shift of the protection enable flag.
    const SHIFT: u32 = 0;
    /// Bit mask of the protection enable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a protection enable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The protection enable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => ProtectionEnableFlag::Disabled,
            _ => ProtectionEnableFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the protection enable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the protection enable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Monitor Coprocessor Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the monitor coprocessor flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorCoprocessorFlag {
    /// Monitor coprocessor is disabled.
    Disabled = 0,
    /// Monitor coprocessor is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl MonitorCoprocessorFlag {
    /// Bit shift of the monitor coprocessor flag.
    const SHIFT: u32 = 1;
    /// Bit mask of the monitor coprocessor flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a monitor coprocessor flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The monitor coprocessor flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => MonitorCoprocessorFlag::Disabled,
            _ => MonitorCoprocessorFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the monitor coprocessor flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the monitor coprocessor flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Emulation Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the emulation flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmulationFlag {
    /// No FPU emulation.
    Disabled = 0,
    /// FPU emulation enabled.
    Enabled = (1 << Self::SHIFT),
}

impl EmulationFlag {
    /// Bit shift of the emulation flag.
    const SHIFT: u32 = 2;
    /// Bit mask of the emulation flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an emulation flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The emulation flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => EmulationFlag::Disabled,
            _ => EmulationFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the emulation flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the emulation flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Task Switched Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the task switched flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSwitchedFlag {
    /// No task switch has occurred.
    Clear = 0,
    /// A task switch has occurred.
    Set = (1 << Self::SHIFT),
}

impl TaskSwitchedFlag {
    /// Bit shift of the task switched flag.
    const SHIFT: u32 = 3;
    /// Bit mask of the task switched flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a task switched flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The task switched flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => TaskSwitchedFlag::Clear,
            _ => TaskSwitchedFlag::Set,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the task switched flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the task switched flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Extension Type Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the extension type flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionTypeFlag {
    /// 80387 DX math coprocessor.
    MathCoprocessor80387 = 0,
    /// 80287 math coprocessor.
    MathCoprocessor80287 = (1 << Self::SHIFT),
}

impl ExtensionTypeFlag {
    /// Bit shift of the extension type flag.
    const SHIFT: u32 = 4;
    /// Bit mask of the extension type flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an extension type flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The extension type flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => ExtensionTypeFlag::MathCoprocessor80387,
            _ => ExtensionTypeFlag::MathCoprocessor80287,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the extension type flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the extension type flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Numeric Error Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the numeric error flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericErrorFlag {
    /// Numeric errors are ignored.
    Ignored = 0,
    /// Numeric errors cause exceptions.
    Exception = (1 << Self::SHIFT),
}

impl NumericErrorFlag {
    /// Bit shift of the numeric error flag.
    const SHIFT: u32 = 5;
    /// Bit mask of the numeric error flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a numeric error flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The numeric error flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => NumericErrorFlag::Ignored,
            _ => NumericErrorFlag::Exception,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the numeric error flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the numeric error flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Write Protect Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the write protect flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteProtectFlag {
    /// Write protection is disabled.
    Disabled = 0,
    /// Write protection is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl WriteProtectFlag {
    /// Bit shift of the write protect flag.
    const SHIFT: u32 = 16;
    /// Bit mask of the write protect flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a write protect flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The write protect flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => WriteProtectFlag::Disabled,
            _ => WriteProtectFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the write protect flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the write protect flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Alignment Mask Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the alignment mask flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentMaskFlag {
    /// Alignment checking is disabled.
    Disabled = 0,
    /// Alignment checking is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl AlignmentMaskFlag {
    /// Bit shift of the alignment mask flag.
    const SHIFT: u32 = 18;
    /// Bit mask of the alignment mask flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an alignment mask flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The alignment mask flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => AlignmentMaskFlag::Disabled,
            _ => AlignmentMaskFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the alignment mask flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the alignment mask flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Not Write Through Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the not write through flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotWriteThroughFlag {
    /// Write-through caching is enabled.
    Enabled = 0,
    /// Write-through caching is disabled.
    Disabled = (1 << Self::SHIFT),
}

impl NotWriteThroughFlag {
    /// Bit shift of the not write through flag.
    const SHIFT: u32 = 29;
    /// Bit mask of the not write through flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a not write through flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The not write through flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => NotWriteThroughFlag::Enabled,
            _ => NotWriteThroughFlag::Disabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the not write through flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the not write through flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Cache Disable Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the cache disable flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheDisableFlag {
    /// Cache is enabled.
    Enabled = 0,
    /// Cache is disabled.
    Disabled = (1 << Self::SHIFT),
}

impl CacheDisableFlag {
    /// Bit shift of the cache disable flag.
    const SHIFT: u32 = 30;
    /// Bit mask of the cache disable flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a cache disable flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The cache disable flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => CacheDisableFlag::Enabled,
            _ => CacheDisableFlag::Disabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the cache disable flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the cache disable flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Paging Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the paging flag in the CR0 register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingFlag {
    /// Paging is disabled.
    Disabled = 0,
    /// Paging is enabled.
    Enabled = (1 << Self::SHIFT),
}

impl PagingFlag {
    /// Bit shift of the paging flag.
    const SHIFT: u32 = 31;
    /// Bit mask of the paging flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a paging flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The paging flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PagingFlag::Disabled,
            _ => PagingFlag::Enabled,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the paging flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the paging flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Control Register Zero (CR0)
//==================================================================================================

///
/// # Description
///
/// A type that represents the CR0 register.
///
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cr0Register {
    /// Protection enable flag.
    pub protection_enable: ProtectionEnableFlag,
    /// Monitor coprocessor flag.
    pub monitor_coprocessor: MonitorCoprocessorFlag,
    /// Emulation flag.
    pub emulation: EmulationFlag,
    /// Task switched flag.
    pub task_switched: TaskSwitchedFlag,
    /// Extension type flag.
    pub extension_type: ExtensionTypeFlag,
    /// Numeric error flag.
    pub numeric_error: NumericErrorFlag,
    /// Write protect flag.
    pub write_protect: WriteProtectFlag,
    /// Alignment mask flag.
    pub alignment_mask: AlignmentMaskFlag,
    /// Not write through flag.
    pub not_write_through: NotWriteThroughFlag,
    /// Cache disable flag.
    pub cache_disable: CacheDisableFlag,
    /// Paging flag.
    pub paging: PagingFlag,
}

impl Cr0Register {
    ///
    /// # Description
    ///
    /// Creates a CR0 register from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the register state.
    ///
    /// # Return Value
    ///
    /// The CR0 register with all flags extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        Self {
            protection_enable: ProtectionEnableFlag::from_u32(value),
            monitor_coprocessor: MonitorCoprocessorFlag::from_u32(value),
            emulation: EmulationFlag::from_u32(value),
            task_switched: TaskSwitchedFlag::from_u32(value),
            extension_type: ExtensionTypeFlag::from_u32(value),
            numeric_error: NumericErrorFlag::from_u32(value),
            write_protect: WriteProtectFlag::from_u32(value),
            alignment_mask: AlignmentMaskFlag::from_u32(value),
            not_write_through: NotWriteThroughFlag::from_u32(value),
            cache_disable: CacheDisableFlag::from_u32(value),
            paging: PagingFlag::from_u32(value),
        }
    }

    ///
    /// # Description
    ///
    /// Converts the CR0 register to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the CR0 register with all flags combined.
    ///
    fn into_u32(self) -> u32 {
        let mut value: u32 = 0;

        value |= self.protection_enable.into_u32();
        value |= self.monitor_coprocessor.into_u32();
        value |= self.emulation.into_u32();
        value |= self.task_switched.into_u32();
        value |= self.extension_type.into_u32();
        value |= self.numeric_error.into_u32();
        value |= self.write_protect.into_u32();
        value |= self.alignment_mask.into_u32();
        value |= self.not_write_through.into_u32();
        value |= self.cache_disable.into_u32();
        value |= self.paging.into_u32();

        value
    }

    ///
    /// # Description
    ///
    /// Reads the value of the CR0 register.
    ///
    /// # Return Value
    ///
    /// The value of the CR0 register.
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
            asm!("mov {0:e}, cr0", out(reg) value);
            Self::from_u32(value)
        }

        #[cfg(target_arch = "x86_64")]
        {
            let value: u64;
            asm!("mov {0:r}, cr0", out(reg) value);
            let value32: u32 = (value & 0xffff_ffff) as u32;
            Self::from_u32(value32)
        }
    }

    ///
    /// # Description
    ///
    /// Writes a value to the CR0 register.
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
            asm!("mov cr0, {0:e}", in(reg) value);
        }

        #[cfg(target_arch = "x86_64")]
        {
            let value64: u64 = self.into_u32() as u64;
            asm!("mov cr0, {0:r}", in(reg) value64);
        }
    }
}

impl Default for Cr0Register {
    fn default() -> Self {
        Self {
            protection_enable: ProtectionEnableFlag::Disabled,
            monitor_coprocessor: MonitorCoprocessorFlag::Disabled,
            emulation: EmulationFlag::Disabled,
            task_switched: TaskSwitchedFlag::Clear,
            extension_type: ExtensionTypeFlag::MathCoprocessor80387,
            numeric_error: NumericErrorFlag::Ignored,
            write_protect: WriteProtectFlag::Disabled,
            alignment_mask: AlignmentMaskFlag::Disabled,
            not_write_through: NotWriteThroughFlag::Enabled,
            cache_disable: CacheDisableFlag::Enabled,
            paging: PagingFlag::Disabled,
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests if protection enable flag works.
fn test_protection_enable_flag() -> bool {
    let value: u32 = 0x00000001;

    if ProtectionEnableFlag::from_u32(value) != ProtectionEnableFlag::Enabled {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        protection_enable: ProtectionEnableFlag::Enabled,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == ProtectionEnableFlag::Enabled.into_u32()
}

/// Tests if monitor coprocessor flag works.
fn test_monitor_coprocessor_flag() -> bool {
    let value: u32 = 0x00000002;

    if MonitorCoprocessorFlag::from_u32(value) != MonitorCoprocessorFlag::Enabled {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        monitor_coprocessor: MonitorCoprocessorFlag::Enabled,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == MonitorCoprocessorFlag::Enabled.into_u32()
}

/// Tests if emulation flag works.
fn test_emulation_flag() -> bool {
    let value: u32 = 0x00000004;

    if EmulationFlag::from_u32(value) != EmulationFlag::Enabled {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        emulation: EmulationFlag::Enabled,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == EmulationFlag::Enabled.into_u32()
}

/// Tests if task switched flag works.
fn test_task_switched_flag() -> bool {
    let value: u32 = 0x00000008;

    if TaskSwitchedFlag::from_u32(value) != TaskSwitchedFlag::Set {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        task_switched: TaskSwitchedFlag::Set,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == TaskSwitchedFlag::Set.into_u32()
}

/// Tests if extension type flag works.
fn test_extension_type_flag() -> bool {
    let value: u32 = 0x00000010;

    if ExtensionTypeFlag::from_u32(value) != ExtensionTypeFlag::MathCoprocessor80287 {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        extension_type: ExtensionTypeFlag::MathCoprocessor80287,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == ExtensionTypeFlag::MathCoprocessor80287.into_u32()
}

/// Tests if numeric error flag works.
fn test_numeric_error_flag() -> bool {
    let value: u32 = 0x00000020;

    if NumericErrorFlag::from_u32(value) != NumericErrorFlag::Exception {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        numeric_error: NumericErrorFlag::Exception,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == NumericErrorFlag::Exception.into_u32()
}

/// Tests if write protect flag works.
fn test_write_protect_flag() -> bool {
    let value: u32 = 0x00010000;

    if WriteProtectFlag::from_u32(value) != WriteProtectFlag::Enabled {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        write_protect: WriteProtectFlag::Enabled,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == WriteProtectFlag::Enabled.into_u32()
}

/// Tests if alignment mask flag works.
fn test_alignment_mask_flag() -> bool {
    let value: u32 = 0x00040000;

    if AlignmentMaskFlag::from_u32(value) != AlignmentMaskFlag::Enabled {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        alignment_mask: AlignmentMaskFlag::Enabled,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == AlignmentMaskFlag::Enabled.into_u32()
}

/// Tests if not write through flag works.
fn test_not_write_through_flag() -> bool {
    let value: u32 = 0x20000000;

    if NotWriteThroughFlag::from_u32(value) != NotWriteThroughFlag::Disabled {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        not_write_through: NotWriteThroughFlag::Disabled,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == NotWriteThroughFlag::Disabled.into_u32()
}

/// Tests if cache disable flag works.
fn test_cache_disable_flag() -> bool {
    let value: u32 = 0x40000000;

    if CacheDisableFlag::from_u32(value) != CacheDisableFlag::Disabled {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        cache_disable: CacheDisableFlag::Disabled,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == CacheDisableFlag::Disabled.into_u32()
}

/// Tests if paging flag works.
fn test_paging_flag() -> bool {
    let value: u32 = 0x80000000;

    if PagingFlag::from_u32(value) != PagingFlag::Enabled {
        return false;
    }

    let cr0: Cr0Register = Cr0Register {
        paging: PagingFlag::Enabled,
        ..Cr0Register::default()
    };

    if cr0 != Cr0Register::from_u32(value) {
        return false;
    }

    if cr0.into_u32() != value {
        return false;
    }

    cr0.into_u32() == PagingFlag::Enabled.into_u32()
}

// Runs all tests for this module.
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= test_protection_enable_flag();
    passed &= test_monitor_coprocessor_flag();
    passed &= test_emulation_flag();
    passed &= test_task_switched_flag();
    passed &= test_extension_type_flag();
    passed &= test_numeric_error_flag();
    passed &= test_write_protect_flag();
    passed &= test_alignment_mask_flag();
    passed &= test_not_write_through_flag();
    passed &= test_cache_disable_flag();
    passed &= test_paging_flag();

    passed
}
