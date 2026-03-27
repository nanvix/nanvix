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
#[derive(Clone)]
#[repr(C, packed)]
pub struct ExceptionInformation {
    /// Exception number.
    num: u32,
    /// Error code.
    code: u32,
    /// Faulting address.
    addr: u32,
    /// Faulting instruction.
    instruction: u32,
}

// `ExceptionInformation` must be 16 bytes long. This must match low-level assembly dispatcher code.
::static_assert::assert_eq_size!(ExceptionInformation, 16);

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
        self.num
    }

    pub fn code(&self) -> u32 {
        self.code
    }

    pub fn addr(&self) -> u32 {
        self.addr
    }

    pub fn instruction(&self) -> u32 {
        self.instruction
    }
}

impl core::fmt::Debug for ExceptionInformation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Copy fields to local variables.
        let num: u32 = self.num;
        let code: u32 = self.code;
        let addr: u32 = self.addr;
        let instr: u32 = self.instruction;
        match Exception::try_from(num) {
            Ok(excp) => write!(
                f,
                "{excp:?} (error code={code}, faulting addr={addr:#010x}, faulting \
                 instruction={instr:#010x})",
            ),
            Err(_) => write!(
                f,
                "unknown exception {num} (error code={code}, faulting addr={addr:#010x}, faulting \
                 instruction={instr:#010x})",
            ),
        }
    }
}
