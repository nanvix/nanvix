// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores information about an exception (64-bit).
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
    pub fn num(&self) -> u64 {
        self.num
    }

    pub fn code(&self) -> u64 {
        self.code
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
        let num: u64 = self.num;
        let code: u64 = self.code;
        let addr: u64 = self.addr;
        let instr: u64 = self.instruction;
        write!(
            f,
            "Exception #{num} (error code={code:#x}, faulting addr={addr:#018x}, faulting \
             instruction={instr:#018x})",
        )
    }
}
