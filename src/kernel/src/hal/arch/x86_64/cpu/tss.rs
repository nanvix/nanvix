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
    /// - `rsp0`: Stack pointer for ring 0.
    ///
    /// # Returns
    ///
    /// Upon success, a reference to the TSS is returned. Upon failure, an error code is returned.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it mutates global variables.
    ///
    pub unsafe fn new(rsp0: u64) -> Result<Self, Error> {
        info!("initializing tss (rsp0={:#018x})", rsp0);

        let tss: Pin<Rc<RefCell<Tss>>> = Pin::new(Rc::new(RefCell::new(Self::init(rsp0))));

        unsafe { TSS = Some(Self(tss.clone())) };

        Ok(Self(tss))
    }

    ///
    /// # Description
    ///
    /// Returns the address of the task state segment (TSS).
    ///
    pub unsafe fn address(&self) -> usize {
        self.0.as_ref().as_ptr() as usize
    }

    ///
    /// # Description
    ///
    /// Returns the size of the task state segment (TSS).
    ///
    pub fn size(&self) -> usize {
        mem::size_of::<Tss>()
    }

    #[inline(never)]
    pub unsafe fn load(&self, selector: u16) {
        info!("loading tss (selector={:x})", selector);
        arch::asm!("ltr %ax", in("ax") selector, options(nostack, att_syntax));
    }

    fn init(rsp0: u64) -> Tss {
        Tss {
            reserved0: 0,
            rsp0,
            rsp1: 0,
            rsp2: 0,
            reserved1: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved2: 0,
            reserved3: 0,
            iomap_base: 0,
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
pub unsafe fn get_curr() -> *const Tss {
    TSS.as_ref().unwrap().0.as_ref().as_ptr()
}
