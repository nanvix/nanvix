// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::sys::mm::{
    Address,
    VirtualAddress,
};

const SPSR_EL0T: u64 = 0;

#[repr(C)]
#[derive(Default)]
pub struct ContextInformation {
    pub(crate) x: [u64; 31],
    pub(crate) sp: u64,
    pub(crate) elr: u64,
    pub(crate) spsr: u64,
    pub(crate) ttbr0: u64,
    pub(crate) sp_el0: u64,
    pub(crate) tpidr_el0: u64,
    /// Keeps exception-stack allocations aligned to the AAPCS64 16-byte stack boundary.
    pub(crate) _padding: u64,
}

::static_assert::assert_eq!(core::mem::size_of::<ContextInformation>().is_multiple_of(16));

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct SignalCpuContext {
    pub ip: u64,
    pub sp: u64,
    pub flags: u64,
    pub ax: u64,
    pub bx: u64,
    pub cx: u64,
    pub dx: u64,
    pub si: u64,
    pub di: u64,
    pub x6: u64,
    pub x7: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub x16: u64,
    pub x17: u64,
    pub x18: u64,
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub bp: u64,
    pub lr: u64,
    pub cs: u64,
    pub ss: u64,
}

impl ContextInformation {
    pub fn new(ttbr0: usize, stack: usize, kernel_stack_top: usize) -> Self {
        let mut context: Self = Self {
            sp: stack as u64,
            ttbr0: ttbr0 as u64,
            sp_el0: kernel_stack_top as u64,
            ..Default::default()
        };
        // A newly forged kernel stack stores the first-resume function in its first word.
        context.x[30] = unsafe { core::ptr::read_unaligned(stack as *const u64) };
        context.sp = (stack + core::mem::size_of::<u64>()) as u64;
        context
    }

    pub fn to_signal_context(&self) -> SignalCpuContext {
        SignalCpuContext {
            ip: self.elr,
            sp: self.sp,
            flags: self.spsr,
            ax: self.x[0],
            bx: self.x[1],
            cx: self.x[2],
            dx: self.x[3],
            si: self.x[4],
            di: self.x[5],
            x6: self.x[6],
            x7: self.x[7],
            r8: self.x[8],
            r9: self.x[9],
            r10: self.x[10],
            r11: self.x[11],
            r12: self.x[12],
            r13: self.x[13],
            r14: self.x[14],
            r15: self.x[15],
            x16: self.x[16],
            x17: self.x[17],
            x18: self.x[18],
            x19: self.x[19],
            x20: self.x[20],
            x21: self.x[21],
            x22: self.x[22],
            x23: self.x[23],
            x24: self.x[24],
            x25: self.x[25],
            x26: self.x[26],
            x27: self.x[27],
            x28: self.x[28],
            bp: self.x[29],
            lr: self.x[30],
            cs: 0,
            ss: 0,
        }
    }

    pub fn redirect_to_signal_handler(
        &mut self,
        entry: usize,
        frame_top: usize,
        restorer: usize,
        signum: usize,
        info_ptr: usize,
        ctx_ptr: usize,
    ) {
        self.elr = entry as u64;
        self.sp = frame_top as u64;
        self.spsr = SPSR_EL0T;
        self.x[0] = signum as u64;
        self.x[1] = info_ptr as u64;
        self.x[2] = ctx_ptr as u64;
        // AArch64 returns through LR rather than consuming the on-stack return address. Use the
        // already validated restorer value instead of dereferencing user memory while PAN is set.
        self.x[30] = restorer as u64;
    }

    pub fn returns_to_user(&self) -> bool {
        self.spsr & 0xf == SPSR_EL0T
    }

    pub unsafe fn switch(
        from: *mut ContextInformation,
        to: *mut ContextInformation,
        user_tda: Option<VirtualAddress>,
    ) {
        unsafe extern "C" {
            fn __context_switch(
                from: *mut ContextInformation,
                to: *mut ContextInformation,
                tpidr_el0: u64,
            );
        }

        let tpidr_el0: u64 = user_tda.map_or(0, |addr| addr.into_raw_value() as u64);
        unsafe {
            crate::hal::arch::set_task_switched();
            __context_switch(from, to, tpidr_el0);
        }
    }
}

impl core::fmt::Debug for ContextInformation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "pc={:#018x}, sp={:#018x}, spsr={:#018x}, ttbr0={:#018x}",
            self.elr, self.sp, self.spsr, self.ttbr0
        )
    }
}

pub unsafe fn forge_user_stack(
    kernel_stack_top: *mut u8,
    user_stack_top: usize,
    user_fn: usize,
    arg0: usize,
    arg1: usize,
    kernel_func: usize,
    enable_interrupts: bool,
) -> *mut u8 {
    let mut stack: *mut u64 = kernel_stack_top.cast();
    stack = unsafe { stack.sub(6) };
    let spsr: u64 = if enable_interrupts { SPSR_EL0T } else { 1 << 7 };
    unsafe {
        stack.write(kernel_func as u64);
        stack.add(1).write(user_fn as u64);
        stack.add(2).write((user_stack_top & !0xf) as u64);
        stack.add(3).write(arg0 as u64);
        stack.add(4).write(arg1 as u64);
        stack.add(5).write(spsr);
    }
    stack.cast()
}
