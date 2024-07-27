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
static_assert_size!(ExceptionInformation, 16);

//==================================================================================================
// Implementations
//==================================================================================================

impl ExceptionInformation {
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
        let excp: Exception = num.into();
        write!(
            f,
            "{:?} (error code={}, faulting addr={:#010x}, faulting instruction={:#010x})",
            excp, code, addr, instr
        )
    }
}
