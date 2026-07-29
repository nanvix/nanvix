// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use ::arch::cpu::excp::Exception;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores information about an exception.
///
/// On x86_64, all fields are 64-bit to match the hooks.S layout.
/// Total size: 4 * 8 = 32 bytes.
///
#[derive(Clone)]
#[repr(C, packed)]
pub struct ExceptionInformation {
    /// Exception number.
    num: u64,
    /// Error code.
    code: u64,
    /// Faulting address.
    addr: u64,
    /// Faulting instruction.
    instruction: u64,
}

// `ExceptionInformation` must be 32 bytes long. This must match low-level assembly dispatcher code.
::static_assert::assert_eq_size!(ExceptionInformation, 32);

//==================================================================================================
// Implementations
//==================================================================================================

impl ExceptionInformation {
    /// Byte offset of the exception number field within the structure.
    pub const EXCEPTION_NR: u32 = core::mem::offset_of!(Self, num) as u32;
    /// Byte offset of the error code field within the structure.
    pub const EXCEPTION_ERR: u32 = core::mem::offset_of!(Self, code) as u32;
    /// Byte offset of the faulting address field within the structure.
    pub const EXCEPTION_DATA: u32 = core::mem::offset_of!(Self, addr) as u32;
    /// Byte offset of the faulting instruction field within the structure.
    pub const EXCEPTION_CODE: u32 = core::mem::offset_of!(Self, instruction) as u32;

    /// Total size of the exception information structure (in bytes).
    pub const EXCEPTION_SIZE: u32 = core::mem::size_of::<Self>() as u32;

    pub fn num(&self) -> u32 {
        self.num as u32
    }

    #[allow(clippy::as_conversions)]
    pub fn code(&self) -> u32 {
        self.code as u32
    }

    pub fn addr(&self) -> u64 {
        self.addr
    }

    pub fn instruction(&self) -> u64 {
        self.instruction
    }
}

impl core::fmt::Debug for ExceptionInformation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Copy fields to local variables to avoid unaligned access on packed struct.
        let num: u64 = self.num;
        let code: u64 = self.code;
        let addr: u64 = self.addr;
        let instr: u64 = self.instruction;
        match Exception::try_from(num as u32) {
            Ok(excp) => write!(
                f,
                "{excp:?} (error code={code}, faulting addr={addr:#018x}, faulting \
                 instruction={instr:#018x})",
            ),
            Err(_) => write!(
                f,
                "unknown exception {num} (error code={code}, faulting addr={addr:#018x}, faulting \
                 instruction={instr:#018x})",
            ),
        }
    }
}
