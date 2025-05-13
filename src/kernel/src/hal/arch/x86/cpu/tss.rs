// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::rc::Rc;
use ::arch::cpu::tss::Tss;
use ::core::{
    arch,
    cell::RefCell,
    mem,
    pin::Pin,
};
use ::sys::error::Error;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that enables one to access the task state segment (TSS).
///
pub struct TssRef(Pin<Rc<RefCell<Tss>>>);

//==================================================================================================
// Global Variables
//==================================================================================================

pub static mut TSS: Option<TssRef> = None;

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
        info!("initializing tss (ss0={:#02x}, esp0={:#08x})", ss0, esp0);

        let tss: Pin<Rc<RefCell<Tss>>> = Pin::new(Rc::new(RefCell::new(Self::init(ss0, esp0))));

        unsafe { TSS = Some(Self(tss.clone())) };

        Ok(Self(tss))
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
    pub unsafe fn address(&self) -> usize {
        self.0.as_ref().as_ptr() as usize
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

    #[inline(never)]
    pub unsafe fn load(&self, selector: u16) {
        info!("loading tss (selector={:x})", selector);
        arch::asm!("ltr %ax", in("ax") selector, options(nostack, att_syntax));
    }

    fn init(ss0: u32, esp0: u32) -> Tss {
        Tss {
            link: 0,
            esp0,
            ss0,
            esp1: 0,
            ss1: 0,
            esp2: 0,
            ss2: 0,
            cr3: 0,
            eip: 0,
            eflags: 0,
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0,
            ebp: 0,
            esi: 0,
            edi: 0,
            es: 0,
            cs: 0,
            ss: 0,
            ds: 0,
            fs: 0,
            gs: 0,
            ldtr: 0,
            iomap: 0,
        }
    }
}

unsafe impl Send for TssRef {}
unsafe impl Sync for TssRef {}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns a pointer to the currently active task state segment (TSS).
///
/// # Returns
///
/// A pointer to the currently active task state segment (TSS).
///
pub unsafe fn get_curr() -> *const Tss {
    TSS.as_ref().unwrap().0.as_ref().as_ptr()
}
