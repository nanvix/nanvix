// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    ContextInformation,
    SignalCpuContext,
};

const CONTEXT_SIZE: usize = core::mem::size_of::<ContextInformation>();
const SPSR_NZCV_MASK: u64 = 0xf << 28;

unsafe fn context(esp0: usize) -> *mut ContextInformation {
    (esp0 - CONTEXT_SIZE) as *mut ContextInformation
}

pub unsafe fn returning_to_user(esp0: usize) -> bool {
    unsafe { (*context(esp0)).returns_to_user() }
}

pub unsafe fn read_user_sp(esp0: usize) -> usize {
    unsafe { (*context(esp0)).sp as usize }
}

pub fn join_kcall_result(ax: u64, _dx: u64) -> i64 {
    ax as i64
}

pub unsafe fn read_trap_context(esp0: usize, result: i64) -> SignalCpuContext {
    let mut cpu: SignalCpuContext = unsafe { (*context(esp0)).to_signal_context() };
    cpu.ax = result as u64;
    cpu
}

pub unsafe fn redirect_to_handler(
    esp0: usize,
    handler_ip: usize,
    frame_top: usize,
    restorer: usize,
    signum: usize,
    info_ptr: usize,
    ctx_ptr: usize,
) {
    unsafe {
        (*context(esp0))
            .redirect_to_signal_handler(handler_ip, frame_top, restorer, signum, info_ptr, ctx_ptr);
    }
}

pub unsafe fn restore_trap_context(esp0: usize, cpu: &SignalCpuContext) {
    let context: &mut ContextInformation = unsafe { &mut *context(esp0) };
    context.elr = cpu.ip;
    context.sp = cpu.sp;
    context.spsr = cpu.flags & SPSR_NZCV_MASK;
    context.x[0] = cpu.ax;
    context.x[1] = cpu.bx;
    context.x[2] = cpu.cx;
    context.x[3] = cpu.dx;
    context.x[4] = cpu.si;
    context.x[5] = cpu.di;
    context.x[6] = cpu.x6;
    context.x[7] = cpu.x7;
    context.x[8] = cpu.r8;
    context.x[9] = cpu.r9;
    context.x[10] = cpu.r10;
    context.x[11] = cpu.r11;
    context.x[12] = cpu.r12;
    context.x[13] = cpu.r13;
    context.x[14] = cpu.r14;
    context.x[15] = cpu.r15;
    context.x[16] = cpu.x16;
    context.x[17] = cpu.x17;
    context.x[18] = cpu.x18;
    context.x[19] = cpu.x19;
    context.x[20] = cpu.x20;
    context.x[21] = cpu.x21;
    context.x[22] = cpu.x22;
    context.x[23] = cpu.x23;
    context.x[24] = cpu.x24;
    context.x[25] = cpu.x25;
    context.x[26] = cpu.x26;
    context.x[27] = cpu.x27;
    context.x[28] = cpu.x28;
    context.x[29] = cpu.bp;
    context.x[30] = cpu.lr;
}

pub fn prepare_kcall_restart(cpu: &mut SignalCpuContext, number: u32, args: [u32; 4]) {
    cpu.ip = cpu.ip.wrapping_sub(4);
    cpu.r8 = u64::from(number);
    cpu.ax = u64::from(args[0]);
    cpu.bx = u64::from(args[1]);
    cpu.cx = u64::from(args[2]);
    cpu.dx = u64::from(args[3]);
}
