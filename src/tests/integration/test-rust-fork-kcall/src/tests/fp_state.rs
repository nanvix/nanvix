// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    arch::asm,
    sync::atomic::{
        AtomicU32,
        Ordering,
    },
};
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    kcall::{
        fork,
        ipc,
        pm,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

const INHERITED_PATTERN: u64 = 0x0123_4567_89ab_cdef;
const CHILD_PATTERN: u64 = 0xfedc_ba98_7654_3210;
const FPCR_RMODE_MASK: u64 = 0b11 << 22;
const INHERITED_RMODE: u64 = 0b01 << 22;
const CHILD_RMODE: u64 = 0b10 << 22;
const INHERITED_FPSR: u64 = 0x01;
const CHILD_FPSR: u64 = 0x02;
const CHILD_EXIT_OK: i32 = 0;
const CHILD_EXIT_FAIL: i32 = 1;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FpState {
    d8: u64,
    fpcr: u64,
    fpsr: u64,
}

//==================================================================================================
// Global State
//==================================================================================================

static PARENT_PID: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Verifies that `fork()` clones AArch64 FP state without sharing later updates.
pub fn run() -> Result<(), Error> {
    let original: FpState = read_fp_state();
    let result: Result<(), Error> = run_inner(original);
    write_fp_state(original);
    result
}

fn run_inner(original: FpState) -> Result<(), Error> {
    let parent_pid: ProcessIdentifier = pm::getpid_uncached()?;
    PARENT_PID.store(u32::try_from(parent_pid)?, Ordering::SeqCst);

    let inherited: FpState = FpState {
        d8: INHERITED_PATTERN,
        fpcr: (original.fpcr & !FPCR_RMODE_MASK) | INHERITED_RMODE,
        fpsr: INHERITED_FPSR,
    };
    write_fp_state(inherited);

    let child_pid: ProcessIdentifier = fork::__kcall_fork()?;
    if child_pid == ProcessIdentifier::from(0) {
        let status: i32 = match run_child(inherited) {
            Ok(()) => CHILD_EXIT_OK,
            Err(_) => CHILD_EXIT_FAIL,
        };
        pm::__kcall_exit(status)?;
    }

    let reply: Message = ipc::__kcall_recv()?;
    assert_eq!(reply.message_type, MessageType::Ipc, "expected FP state reply from child");
    assert_eq!(reply.payload[0], 1, "child did not inherit the parent's FP state");
    assert_eq!(read_fp_state(), inherited, "child FP update leaked into the parent");
    Ok(())
}

fn run_child(inherited: FpState) -> Result<(), Error> {
    let parent_pid: ProcessIdentifier =
        ProcessIdentifier::try_from(PARENT_PID.load(Ordering::SeqCst))?;
    let child_pid: ProcessIdentifier = pm::getpid_uncached()?;
    let inherited_ok: bool = read_fp_state() == inherited;
    let child_state: FpState = FpState {
        d8: CHILD_PATTERN,
        fpcr: (inherited.fpcr & !FPCR_RMODE_MASK) | CHILD_RMODE,
        fpsr: CHILD_FPSR,
    };
    write_fp_state(child_state);

    let mut payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];
    payload[0] = u8::from(inherited_ok && read_fp_state() == child_state);
    let reply: Message = Message::new(
        MessageSender::new(child_pid, ThreadIdentifier::NONE),
        MessageReceiver::new(parent_pid, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        payload,
    );
    ipc::__kcall_send(&reply)
}

fn read_fp_state() -> FpState {
    let d8: u64;
    let fpcr: u64;
    let fpsr: u64;
    unsafe {
        asm!(
            "fmov {d8}, d8",
            "mrs {fpcr}, fpcr",
            "mrs {fpsr}, fpsr",
            d8 = out(reg) d8,
            fpcr = out(reg) fpcr,
            fpsr = out(reg) fpsr,
            options(nomem, nostack),
        );
    }
    FpState { d8, fpcr, fpsr }
}

fn write_fp_state(state: FpState) {
    unsafe {
        asm!(
            "fmov d8, {d8}",
            "msr fpcr, {fpcr}",
            "msr fpsr, {fpsr}",
            d8 = in(reg) state.d8,
            fpcr = in(reg) state.fpcr,
            fpsr = in(reg) state.fpsr,
            options(nomem, nostack),
        );
    }
}
