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
// Invalid Operation Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the invalid operation flag in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidOperationFlag {
    /// No invalid operation occurred.
    Clear = 0,
    /// Invalid operation occurred.
    Set = (1 << Self::SHIFT),
}

impl InvalidOperationFlag {
    /// Bit shift of the invalid operation flag.
    const SHIFT: u32 = 0;
    /// Bit mask of the invalid operation flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an invalid operation flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The invalid operation flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => InvalidOperationFlag::Clear,
            _ => InvalidOperationFlag::Set,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the invalid operation flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the invalid operation flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Denormal Operation Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the denormal operation flag in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenormalOperationFlag {
    /// No denormal operation occurred.
    Clear = 0,
    /// Denormal operation occurred.
    Set = (1 << Self::SHIFT),
}

impl DenormalOperationFlag {
    /// Bit shift of the denormal operation flag.
    const SHIFT: u32 = 1;
    /// Bit mask of the denormal operation flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a denormal operation flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The denormal operation flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => DenormalOperationFlag::Clear,
            _ => DenormalOperationFlag::Set,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the denormal operation flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the denormal operation flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Divide By Zero Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the divide by zero flag in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivideByZeroFlag {
    /// No divide by zero occurred.
    Clear = 0,
    /// Divide by zero occurred.
    Set = (1 << Self::SHIFT),
}

impl DivideByZeroFlag {
    /// Bit shift of the divide by zero flag.
    const SHIFT: u32 = 2;
    /// Bit mask of the divide by zero flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a divide by zero flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The divide by zero flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => DivideByZeroFlag::Clear,
            _ => DivideByZeroFlag::Set,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the divide by zero flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the divide by zero flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Overflow Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the overflow flag in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowFlag {
    /// No overflow occurred.
    Clear = 0,
    /// Overflow occurred.
    Set = (1 << Self::SHIFT),
}

impl OverflowFlag {
    /// Bit shift of the overflow flag.
    const SHIFT: u32 = 3;
    /// Bit mask of the overflow flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an overflow flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The overflow flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => OverflowFlag::Clear,
            _ => OverflowFlag::Set,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the overflow flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the overflow flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Underflow Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the underflow flag in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderflowFlag {
    /// No underflow occurred.
    Clear = 0,
    /// Underflow occurred.
    Set = (1 << Self::SHIFT),
}

impl UnderflowFlag {
    /// Bit shift of the underflow flag.
    const SHIFT: u32 = 4;
    /// Bit mask of the underflow flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an underflow flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The underflow flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => UnderflowFlag::Clear,
            _ => UnderflowFlag::Set,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the underflow flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the underflow flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Precision Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the precision flag in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecisionFlag {
    /// No precision exception occurred.
    Clear = 0,
    /// Precision exception occurred.
    Set = (1 << Self::SHIFT),
}

impl PrecisionFlag {
    /// Bit shift of the precision flag.
    const SHIFT: u32 = 5;
    /// Bit mask of the precision flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a precision flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The precision flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PrecisionFlag::Clear,
            _ => PrecisionFlag::Set,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the precision flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the precision flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Denormals Are Zeros Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the denormals are zeros flag in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenormalsAreZerosFlag {
    /// Denormals are processed normally.
    Normal = 0,
    /// Denormals are treated as zeros.
    Zeros = (1 << Self::SHIFT),
}

impl DenormalsAreZerosFlag {
    /// Bit shift of the denormals are zeros flag.
    const SHIFT: u32 = 6;
    /// Bit mask of the denormals are zeros flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a denormals are zeros flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The denormals are zeros flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => DenormalsAreZerosFlag::Normal,
            _ => DenormalsAreZerosFlag::Zeros,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the denormals are zeros flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the denormals are zeros flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Invalid Operation Mask
//==================================================================================================

///
/// # Description
///
/// A type that represents the invalid operation mask in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidOperationMask {
    /// Invalid operation exceptions are enabled.
    Enabled = 0,
    /// Invalid operation exceptions are masked.
    Masked = (1 << Self::SHIFT),
}

impl InvalidOperationMask {
    /// Bit shift of the invalid operation mask.
    const SHIFT: u32 = 7;
    /// Bit mask of the invalid operation mask.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an invalid operation mask from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the mask.
    ///
    /// # Return Value
    ///
    /// The invalid operation mask extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => InvalidOperationMask::Enabled,
            _ => InvalidOperationMask::Masked,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the invalid operation mask to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the invalid operation mask.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Denormal Operation Mask
//==================================================================================================

///
/// # Description
///
/// A type that represents the denormal operation mask in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenormalOperationMask {
    /// Denormal operation exceptions are enabled.
    Enabled = 0,
    /// Denormal operation exceptions are masked.
    Masked = (1 << Self::SHIFT),
}

impl DenormalOperationMask {
    /// Bit shift of the denormal operation mask.
    const SHIFT: u32 = 8;
    /// Bit mask of the denormal operation mask.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a denormal operation mask from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the mask.
    ///
    /// # Return Value
    ///
    /// The denormal operation mask extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => DenormalOperationMask::Enabled,
            _ => DenormalOperationMask::Masked,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the denormal operation mask to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the denormal operation mask.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Divide By Zero Mask
//==================================================================================================

///
/// # Description
///
/// A type that represents the divide by zero mask in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivideByZeroMask {
    /// Divide by zero exceptions are enabled.
    Enabled = 0,
    /// Divide by zero exceptions are masked.
    Masked = (1 << Self::SHIFT),
}

impl DivideByZeroMask {
    /// Bit shift of the divide by zero mask.
    const SHIFT: u32 = 9;
    /// Bit mask of the divide by zero mask.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a divide by zero mask from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the mask.
    ///
    /// # Return Value
    ///
    /// The divide by zero mask extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => DivideByZeroMask::Enabled,
            _ => DivideByZeroMask::Masked,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the divide by zero mask to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the divide by zero mask.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Overflow Mask
//==================================================================================================

///
/// # Description
///
/// A type that represents the overflow mask in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowMask {
    /// Overflow exceptions are enabled.
    Enabled = 0,
    /// Overflow exceptions are masked.
    Masked = (1 << Self::SHIFT),
}

impl OverflowMask {
    /// Bit shift of the overflow mask.
    const SHIFT: u32 = 10;
    /// Bit mask of the overflow mask.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an overflow mask from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the mask.
    ///
    /// # Return Value
    ///
    /// The overflow mask extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => OverflowMask::Enabled,
            _ => OverflowMask::Masked,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the overflow mask to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the overflow mask.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Underflow Mask
//==================================================================================================

///
/// # Description
///
/// A type that represents the underflow mask in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderflowMask {
    /// Underflow exceptions are enabled.
    Enabled = 0,
    /// Underflow exceptions are masked.
    Masked = (1 << Self::SHIFT),
}

impl UnderflowMask {
    /// Bit shift of the underflow mask.
    const SHIFT: u32 = 11;
    /// Bit mask of the underflow mask.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates an underflow mask from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the mask.
    ///
    /// # Return Value
    ///
    /// The underflow mask extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => UnderflowMask::Enabled,
            _ => UnderflowMask::Masked,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the underflow mask to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the underflow mask.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Precision Mask
//==================================================================================================

///
/// # Description
///
/// A type that represents the precision mask in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecisionMask {
    /// Precision exceptions are enabled.
    Enabled = 0,
    /// Precision exceptions are masked.
    Masked = (1 << Self::SHIFT),
}

impl PrecisionMask {
    /// Bit shift of the precision mask.
    const SHIFT: u32 = 12;
    /// Bit mask of the precision mask.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a precision mask from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the mask.
    ///
    /// # Return Value
    ///
    /// The precision mask extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => PrecisionMask::Enabled,
            _ => PrecisionMask::Masked,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the precision mask to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the precision mask.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Rounding Control
//==================================================================================================

///
/// # Description
///
/// A type that represents the rounding control in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingControl {
    /// Round to nearest (even).
    RoundToNearest = 0,
    /// Round down (toward negative infinity).
    RoundDown = (1 << Self::SHIFT),
    /// Round up (toward positive infinity).
    RoundUp = (2 << Self::SHIFT),
    /// Round toward zero (truncate).
    RoundTowardZero = (3 << Self::SHIFT),
}

impl RoundingControl {
    /// Bit shift of the rounding control.
    const SHIFT: u32 = 13;
    /// Bit mask of the rounding control.
    const MASK: u32 = (3 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a rounding control from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the control.
    ///
    /// # Return Value
    ///
    /// The rounding control extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match (value & Self::MASK) >> Self::SHIFT {
            0 => RoundingControl::RoundToNearest,
            1 => RoundingControl::RoundDown,
            2 => RoundingControl::RoundUp,
            3 => RoundingControl::RoundTowardZero,
            _ => RoundingControl::RoundToNearest, // Default case
        }
    }

    ///
    /// # Description
    ///
    /// Converts the rounding control to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the rounding control.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// Flush To Zero Flag
//==================================================================================================

///
/// # Description
///
/// A type that represents the flush to zero flag in the MXCSR register.
///
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushToZeroFlag {
    /// Underflow results are processed normally.
    Normal = 0,
    /// Underflow results are flushed to zero.
    FlushToZero = (1 << Self::SHIFT),
}

impl FlushToZeroFlag {
    /// Bit shift of the flush to zero flag.
    const SHIFT: u32 = 15;
    /// Bit mask of the flush to zero flag.
    const MASK: u32 = (1 << Self::SHIFT);

    ///
    /// # Description
    ///
    /// Creates a flush to zero flag from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the flag.
    ///
    /// # Return Value
    ///
    /// The flush to zero flag extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        match value & Self::MASK {
            0 => FlushToZeroFlag::Normal,
            _ => FlushToZeroFlag::FlushToZero,
        }
    }

    ///
    /// # Description
    ///
    /// Converts the flush to zero flag to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the flush to zero flag.
    ///
    fn into_u32(self) -> u32 {
        self as u32
    }
}

//==================================================================================================
// MXCSR Register
//==================================================================================================

///
/// # Description
///
/// A type that represents the MXCSR register.
///
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MxcsrRegister {
    /// Invalid operation flag.
    pub invalid_operation: InvalidOperationFlag,
    /// Denormal operation flag.
    pub denormal_operation: DenormalOperationFlag,
    /// Divide by zero flag.
    pub divide_by_zero: DivideByZeroFlag,
    /// Overflow flag.
    pub overflow: OverflowFlag,
    /// Underflow flag.
    pub underflow: UnderflowFlag,
    /// Precision flag.
    pub precision: PrecisionFlag,
    /// Denormals are zeros flag.
    pub denormals_are_zeros: DenormalsAreZerosFlag,
    /// Invalid operation mask.
    pub invalid_operation_mask: InvalidOperationMask,
    /// Denormal operation mask.
    pub denormal_operation_mask: DenormalOperationMask,
    /// Divide by zero mask.
    pub divide_by_zero_mask: DivideByZeroMask,
    /// Overflow mask.
    pub overflow_mask: OverflowMask,
    /// Underflow mask.
    pub underflow_mask: UnderflowMask,
    /// Precision mask.
    pub precision_mask: PrecisionMask,
    /// Rounding control.
    pub rounding_control: RoundingControl,
    /// Flush to zero flag.
    pub flush_to_zero: FlushToZeroFlag,
}

impl MxcsrRegister {
    ///
    /// # Description
    ///
    /// Creates an MXCSR register from a raw 32-bit value.
    ///
    /// # Parameters
    ///
    /// - `value`: The raw 32-bit value containing the register state.
    ///
    /// # Return Value
    ///
    /// The MXCSR register with all flags extracted from the value.
    ///
    fn from_u32(value: u32) -> Self {
        Self {
            invalid_operation: InvalidOperationFlag::from_u32(value),
            denormal_operation: DenormalOperationFlag::from_u32(value),
            divide_by_zero: DivideByZeroFlag::from_u32(value),
            overflow: OverflowFlag::from_u32(value),
            underflow: UnderflowFlag::from_u32(value),
            precision: PrecisionFlag::from_u32(value),
            denormals_are_zeros: DenormalsAreZerosFlag::from_u32(value),
            invalid_operation_mask: InvalidOperationMask::from_u32(value),
            denormal_operation_mask: DenormalOperationMask::from_u32(value),
            divide_by_zero_mask: DivideByZeroMask::from_u32(value),
            overflow_mask: OverflowMask::from_u32(value),
            underflow_mask: UnderflowMask::from_u32(value),
            precision_mask: PrecisionMask::from_u32(value),
            rounding_control: RoundingControl::from_u32(value),
            flush_to_zero: FlushToZeroFlag::from_u32(value),
        }
    }

    ///
    /// # Description
    ///
    /// Converts the MXCSR register to a 32-bit value.
    ///
    /// # Return Value
    ///
    /// The 32-bit representation of the MXCSR register with all flags combined.
    ///
    fn into_u32(self) -> u32 {
        let mut value: u32 = 0;

        value |= self.invalid_operation.into_u32();
        value |= self.denormal_operation.into_u32();
        value |= self.divide_by_zero.into_u32();
        value |= self.overflow.into_u32();
        value |= self.underflow.into_u32();
        value |= self.precision.into_u32();
        value |= self.denormals_are_zeros.into_u32();
        value |= self.invalid_operation_mask.into_u32();
        value |= self.denormal_operation_mask.into_u32();
        value |= self.divide_by_zero_mask.into_u32();
        value |= self.overflow_mask.into_u32();
        value |= self.underflow_mask.into_u32();
        value |= self.precision_mask.into_u32();
        value |= self.rounding_control.into_u32();
        value |= self.flush_to_zero.into_u32();

        value
    }

    ///
    /// # Description
    ///
    /// Reads the value of the MXCSR register.
    ///
    /// # Return Value
    ///
    /// The value of the MXCSR register.
    ///
    /// # Safety
    ///
    /// It is unsafe to call this function because they require SSE support from the CPU and OS.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - The processor supports SSE instructions.
    /// - The operating system supports the use of SSE instructions.
    ///
    pub unsafe fn read() -> Self {
        let mut value: u32 = 0;
        #[cfg(target_arch = "x86")]
        {
            asm!(
                "stmxcsr [{0}]",
                in(reg) &mut value,
                options(nostack, preserves_flags)
            );
        }
        #[cfg(target_arch = "x86_64")]
        {
            asm!(
                "stmxcsr [{0}]",
                in(reg) &mut value,
                options(nostack, preserves_flags)
            );
        }
        Self::from_u32(value)
    }

    ///
    /// # Description
    ///
    /// Writes a value to the MXCSR register.
    ///
    /// # Safety
    ///
    /// It is unsafe to call this function because they require SSE support from the CPU and OS.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - The processor supports SSE instructions.
    /// - The operating system supports the use of SSE instructions.
    ///
    pub unsafe fn write(&self) {
        let value: u32 = self.into_u32();
        #[cfg(target_arch = "x86")]
        {
            asm!(
                "ldmxcsr [{0}]",
                in(reg) &value,
                options(nostack, preserves_flags)
            );
        }
        #[cfg(target_arch = "x86_64")]
        {
            asm!(
                "ldmxcsr [{0}]",
                in(reg) &value,
                options(nostack, preserves_flags)
            );
        }
    }
}

impl Default for MxcsrRegister {
    fn default() -> Self {
        Self {
            invalid_operation: InvalidOperationFlag::Clear,
            denormal_operation: DenormalOperationFlag::Clear,
            divide_by_zero: DivideByZeroFlag::Clear,
            overflow: OverflowFlag::Clear,
            underflow: UnderflowFlag::Clear,
            precision: PrecisionFlag::Clear,
            denormals_are_zeros: DenormalsAreZerosFlag::Normal,
            invalid_operation_mask: InvalidOperationMask::Masked,
            denormal_operation_mask: DenormalOperationMask::Masked,
            divide_by_zero_mask: DivideByZeroMask::Masked,
            overflow_mask: OverflowMask::Masked,
            underflow_mask: UnderflowMask::Masked,
            precision_mask: PrecisionMask::Masked,
            rounding_control: RoundingControl::RoundToNearest,
            flush_to_zero: FlushToZeroFlag::Normal,
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests if invalid operation flag works.
fn test_invalid_operation_flag() -> bool {
    let value: u32 = 0x00000001;

    if InvalidOperationFlag::from_u32(value) != InvalidOperationFlag::Set {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        invalid_operation: InvalidOperationFlag::Set,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == InvalidOperationFlag::Set.into_u32()
}

/// Tests if denormal operation flag works.
fn test_denormal_operation_flag() -> bool {
    let value: u32 = 0x00000002;

    if DenormalOperationFlag::from_u32(value) != DenormalOperationFlag::Set {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        denormal_operation: DenormalOperationFlag::Set,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == DenormalOperationFlag::Set.into_u32()
}

/// Tests if divide by zero flag works.
fn test_divide_by_zero_flag() -> bool {
    let value: u32 = 0x00000004;

    if DivideByZeroFlag::from_u32(value) != DivideByZeroFlag::Set {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        divide_by_zero: DivideByZeroFlag::Set,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == DivideByZeroFlag::Set.into_u32()
}

/// Tests if overflow flag works.
fn test_overflow_flag() -> bool {
    let value: u32 = 0x00000008;

    if OverflowFlag::from_u32(value) != OverflowFlag::Set {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        overflow: OverflowFlag::Set,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == OverflowFlag::Set.into_u32()
}

/// Tests if underflow flag works.
fn test_underflow_flag() -> bool {
    let value: u32 = 0x00000010;

    if UnderflowFlag::from_u32(value) != UnderflowFlag::Set {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        underflow: UnderflowFlag::Set,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == UnderflowFlag::Set.into_u32()
}

/// Tests if precision flag works.
fn test_precision_flag() -> bool {
    let value: u32 = 0x00000020;

    if PrecisionFlag::from_u32(value) != PrecisionFlag::Set {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        precision: PrecisionFlag::Set,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == PrecisionFlag::Set.into_u32()
}

/// Tests if denormals are zeros flag works.
fn test_denormals_are_zeros_flag() -> bool {
    let value: u32 = 0x00000040;

    if DenormalsAreZerosFlag::from_u32(value) != DenormalsAreZerosFlag::Zeros {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        denormals_are_zeros: DenormalsAreZerosFlag::Zeros,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == DenormalsAreZerosFlag::Zeros.into_u32()
}

/// Tests if invalid operation mask works.
fn test_invalid_operation_mask() -> bool {
    let value: u32 = 0x00000080;

    if InvalidOperationMask::from_u32(value) != InvalidOperationMask::Masked {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        invalid_operation_mask: InvalidOperationMask::Masked,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == InvalidOperationMask::Masked.into_u32()
}

/// Tests if denormal operation mask works.
fn test_denormal_operation_mask() -> bool {
    let value: u32 = 0x00000100;

    if DenormalOperationMask::from_u32(value) != DenormalOperationMask::Masked {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        denormal_operation_mask: DenormalOperationMask::Masked,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == DenormalOperationMask::Masked.into_u32()
}

/// Tests if divide by zero mask works.
fn test_divide_by_zero_mask() -> bool {
    let value: u32 = 0x00000200;

    if DivideByZeroMask::from_u32(value) != DivideByZeroMask::Masked {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        divide_by_zero_mask: DivideByZeroMask::Masked,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == DivideByZeroMask::Masked.into_u32()
}

/// Tests if overflow mask works.
fn test_overflow_mask() -> bool {
    let value: u32 = 0x00000400;

    if OverflowMask::from_u32(value) != OverflowMask::Masked {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        overflow_mask: OverflowMask::Masked,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == OverflowMask::Masked.into_u32()
}

/// Tests if underflow mask works.
fn test_underflow_mask() -> bool {
    let value: u32 = 0x00000800;

    if UnderflowMask::from_u32(value) != UnderflowMask::Masked {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        underflow_mask: UnderflowMask::Masked,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == UnderflowMask::Masked.into_u32()
}

/// Tests if precision mask works.
fn test_precision_mask() -> bool {
    let value: u32 = 0x00001000;

    if PrecisionMask::from_u32(value) != PrecisionMask::Masked {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        precision_mask: PrecisionMask::Masked,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == PrecisionMask::Masked.into_u32()
}

/// Tests if rounding control works.
fn test_rounding_control() -> bool {
    let value: u32 = 0x00006000; // RoundTowardZero (bits 13-14 = 11)

    if RoundingControl::from_u32(value) != RoundingControl::RoundTowardZero {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        rounding_control: RoundingControl::RoundTowardZero,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == RoundingControl::RoundTowardZero.into_u32()
}

/// Tests if flush to zero flag works.
fn test_flush_to_zero_flag() -> bool {
    let value: u32 = 0x00008000;

    if FlushToZeroFlag::from_u32(value) != FlushToZeroFlag::FlushToZero {
        return false;
    }

    let mxcsr: MxcsrRegister = MxcsrRegister {
        flush_to_zero: FlushToZeroFlag::FlushToZero,
        ..MxcsrRegister::default()
    };

    if mxcsr != MxcsrRegister::from_u32(value) {
        return false;
    }

    if mxcsr.into_u32() != value {
        return false;
    }

    mxcsr.into_u32() == FlushToZeroFlag::FlushToZero.into_u32()
}

// Runs all tests for this module.
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= test_invalid_operation_flag();
    passed &= test_denormal_operation_flag();
    passed &= test_divide_by_zero_flag();
    passed &= test_overflow_flag();
    passed &= test_underflow_flag();
    passed &= test_precision_flag();
    passed &= test_denormals_are_zeros_flag();
    passed &= test_invalid_operation_mask();
    passed &= test_denormal_operation_mask();
    passed &= test_divide_by_zero_mask();
    passed &= test_overflow_mask();
    passed &= test_underflow_mask();
    passed &= test_precision_mask();
    passed &= test_rounding_control();
    passed &= test_flush_to_zero_flag();

    passed
}
