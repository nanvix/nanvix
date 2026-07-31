// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::runtime::KernelThread;
use ::core::{
    arch::asm,
    sync::atomic::{
        AtomicU8,
        Ordering,
    },
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::sched::__kcall_sched_yield,
};

//==================================================================================================
// Constants
//==================================================================================================

const MAIN_PATTERN: u64 = 0x1122_3344_5566_7788;
const WORKER_PATTERN: u64 = 0x8877_6655_4433_2211;
const MAIN_FPSR: u64 = 0x01;
const WORKER_FPSR: u64 = 0x02;
const FPCR_RMODE_MASK: u64 = 0b11 << 22;
const MAIN_RMODE: u64 = 0b01 << 22;
const WORKER_RMODE: u64 = 0b10 << 22;
const SWITCH_ROUNDS: usize = 64;
const WAIT_ROUNDS: usize = 4096;
const TURN_WORKER_READY: u8 = 1;
const TURN_WORKER_RUN: u8 = 2;
const TURN_WORKER_DONE: u8 = 3;
const TURN_FAILED: u8 = u8::MAX;

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

static TURN: AtomicU8 = AtomicU8::new(0);

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Verifies that lazy FP ownership preserves AArch64 FP state across thread switches.
pub fn run() -> Result<(), Error> {
    let original: FpState = read_fp_state();
    let result: Result<(), Error> = run_inner(original);
    write_fp_state(original);
    result
}

fn run_inner(original: FpState) -> Result<(), Error> {
    TURN.store(0, Ordering::Relaxed);
    let worker: KernelThread = KernelThread::spawn(fp_worker, 0)?;
    let expected: FpState = FpState {
        d8: MAIN_PATTERN,
        fpcr: (original.fpcr & !FPCR_RMODE_MASK) | MAIN_RMODE,
        fpsr: MAIN_FPSR,
    };
    write_fp_state(expected);

    wait_for_turn(TURN_WORKER_READY)?;
    for round in 0..SWITCH_ROUNDS {
        assert_eq!(read_fp_state(), expected, "main FP state changed before handoff");
        TURN.store(TURN_WORKER_RUN, Ordering::Release);
        wait_for_turn(TURN_WORKER_DONE)?;
        assert_eq!(read_fp_state(), expected, "main FP state changed after handoff");
        if round + 1 < SWITCH_ROUNDS {
            TURN.store(TURN_WORKER_READY, Ordering::Release);
        }
    }

    let status: usize = worker.join()?;
    assert_eq!(status, 0, "worker FP state changed across a context switch");
    Ok(())
}

extern "C" fn fp_worker(_arg: usize) -> usize {
    let original: FpState = read_fp_state();
    let result: Result<(), Error> = fp_worker_inner(original);
    write_fp_state(original);
    if result.is_ok() {
        0
    } else {
        TURN.store(TURN_FAILED, Ordering::Release);
        1
    }
}

fn fp_worker_inner(original: FpState) -> Result<(), Error> {
    let expected: FpState = FpState {
        d8: WORKER_PATTERN,
        fpcr: (original.fpcr & !FPCR_RMODE_MASK) | WORKER_RMODE,
        fpsr: WORKER_FPSR,
    };
    write_fp_state(expected);
    TURN.store(TURN_WORKER_READY, Ordering::Release);

    for _ in 0..SWITCH_ROUNDS {
        wait_for_turn(TURN_WORKER_RUN)?;
        if read_fp_state() != expected {
            return Err(Error::new(ErrorCode::TryAgain, "worker FP state was not preserved"));
        }
        TURN.store(TURN_WORKER_DONE, Ordering::Release);
    }

    Ok(())
}

fn wait_for_turn(expected: u8) -> Result<(), Error> {
    for _ in 0..WAIT_ROUNDS {
        let turn: u8 = TURN.load(Ordering::Acquire);
        if turn == expected {
            return Ok(());
        }
        if turn == TURN_FAILED {
            return Err(Error::new(ErrorCode::TryAgain, "peer FP state check failed"));
        }
        __kcall_sched_yield()?;
    }

    Err(Error::new(ErrorCode::OperationTimedOut, "timed out waiting for FP context-switch peer"))
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
