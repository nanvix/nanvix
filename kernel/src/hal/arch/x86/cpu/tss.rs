// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::tss::Tss;
use ::core::{
    arch,
    mem,
    ops::{
        Deref,
        DerefMut,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Holds a reference to the task state segment (TSS).
pub struct TssRef(&'static mut Tss);

//==================================================================================================
// Global Variables
//==================================================================================================

/// Task state segment (TSS).
#[no_mangle]
pub static mut TSS: Tss = unsafe { mem::zeroed() };

/// Indicates if the TSS was initialized.
static mut TSS_REF: Option<TssRef> = Some(TssRef(unsafe { &mut TSS }));

//==================================================================================================
// Implementations
//==================================================================================================

impl TssRef {
    ///
    /// # Description
    ///
    /// Initializes the task state segment (TSS).
    ///
    /// # Parameters
    ///
    /// - `ss0`: Stack segment for ring 0.
    /// - `esp0`: Stack pointer for ring 0.
    ///
    /// # Returns
    ///
    /// Upon success, a reference to the TSS is returned. Upon failure, an error code is returned.
    ///
    /// # Safety
    ///
    /// This function is unsafe for the following reasons:
    /// - It mutates global variables.
    ///
    pub unsafe fn new(ss0: u32, esp0: u32) -> Result<Self, Error> {
        let mut tss = TSS_REF
            .take()
            .ok_or(Error::new(ErrorCode::OutOfMemory, "tss is already initialized"))?;

        // Initialize TSS.
        info!("initializing tss (ss0={:#02x}, esp0={:#08x})", ss0, esp0);

        tss.init(ss0, esp0);

        Ok(tss)
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the task state segment (TSS).
    ///
    /// # Returns
    ///
    /// A reference to the task state segment (TSS).
    ///
    pub fn address(&self) -> usize {
        self.0 as *const Tss as usize
    }

    ///
    /// # Description
    ///
    /// Returns the size of the task state segment (TSS).
    ///
    /// # Returns
    ///
    /// The size of the task state segment (TSS).
    ///
    pub fn size(&self) -> usize {
        mem::size_of::<Tss>()
    }

    pub unsafe fn load(&self, selector: u16) {
        info!("loading tss (selector={:x})", selector);
        arch::asm!("ltr %ax", in("ax") selector, options(nostack, att_syntax));
    }

    fn init(&mut self, ss0: u32, esp0: u32) {
        self.link = 0;
        self.esp0 = esp0;
        self.ss0 = ss0;
        self.esp1 = 0;
        self.ss1 = 0;
        self.esp2 = 0;
        self.ss2 = 0;
        self.cr3 = 0;
        self.eip = 0;
        self.eflags = 0;
        self.eax = 0;
        self.ecx = 0;
        self.edx = 0;
        self.ebx = 0;
        self.esp = 0;
        self.ebp = 0;
        self.esi = 0;
        self.edi = 0;
        self.es = 0;
        self.cs = 0;
        self.ss = 0;
        self.ds = 0;
        self.fs = 0;
        self.gs = 0;
        self.ldtr = 0;
        self.iomap = 0;
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl Deref for TssRef {
    type Target = Tss;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl DerefMut for TssRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
