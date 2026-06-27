// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Asynchronous Signal Delivery
//!
//! Glue between the process manager and the architecture-neutral [`sigframe`] logic. At the
//! return-to-user boundary the kernel checks the running process for a deliverable caught signal
//! and, if one is found, redirects the interrupted thread through its user-space handler by:
//!
//! 1. reading the interrupted user context off the top of the thread's kernel stack (the hardware
//!    trap frame plus the callee-saved registers preserved by the kernel-call entry stub),
//! 2. building a signal frame on the thread's user stack that saves that context, the FPU state,
//!    and the blocked mask, and
//! 3. rewriting the trap frame so the kernel-call return path resumes in the handler.
//!
//! `sigreturn()` reverses the process: it validates the on-stack frame, restores the saved context,
//! FPU state, and mask, and resumes the interrupted instruction stream.
//!
//! [`sigframe`]: crate::pm::process::state::sigframe
//!

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    r#unsafe::FPU_OWNER_TID,
    ProcessManager,
};
use crate::{
    hal::{
        arch::{
            capture_fpu,
            install_fpu,
            join_kcall_result,
            prepare_kcall_restart,
            read_trap_context,
            read_user_sp,
            redirect_to_handler,
            restore_trap_context,
            returning_to_user,
        },
        mem::Address,
    },
    mm::Vmem,
    pm::{
        process::state::{
            sigframe::{
                self,
                build_frame,
                frame_layout,
                next_blocked,
                save_area_offset_from_sigreturn_sp,
                validate_and_restore,
                FrameLayout,
                SigFrame,
                SignalCpuContext,
                FPU_AREA_SIZE,
                RETADDR_SIZE,
            },
            signal::{
                SignalDisposition,
                UNBLOCKABLE,
            },
        },
        KcallRestart,
        ORDER,
    },
};
use ::sys::{
    error::Error,
    mm::VirtualAddress,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
        SA_NODEFER,
        SA_RESETHAND,
        SA_RESTART,
        SA_SIGINFO,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Outcome of evaluating signal delivery at a return-to-user checkpoint.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalDeliveryOutcome {
    /// No deliverable signal was pending; the thread resumes normally.
    None,
    /// A signal frame was built and the thread was redirected to its handler.
    Delivered,
    /// A frame could not be built safely; the caller must take the signal's default action and
    /// terminate the process.
    Escalate,
}

///
/// # Description
///
/// Reason a `sigreturn()` request failed.
///
// `Unsupported` is returned only on architectures without a delivery path (whose trap-frame
// accessors are inert placeholders); `Forged` only where a real frame is built and then fails
// validation. The `sigreturn()` handler matches both.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SigReturnFailure {
    /// The on-stack frame was corrupt or forged; the process must be terminated.
    Forged,
    /// `sigreturn()` is not implemented on this architecture.
    Unsupported,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessManager {
    ///
    /// # Description
    ///
    /// Records the user-space signal-return trampoline (restorer) for a process.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process.
    /// - `restorer`: Address of the restorer trampoline in the process's address space.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Upon failure, an error is returned instead.
    ///
    pub fn set_signal_restorer(
        &mut self,
        pid: ProcessIdentifier,
        restorer: VirtualAddress,
    ) -> Result<(), Error> {
        self.find_process_mut(pid)?
            .state_mut()
            .signals_mut()
            .set_restorer(Some(restorer));
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Records, on the running thread, a blocking kernel call that a deliverable caught signal just
    /// interrupted. The asynchronous-delivery checkpoint consumes this record to transparently
    /// restart the call when the interrupting handler is installed with `SA_RESTART`.
    ///
    /// # Parameters
    ///
    /// - `restart`: The interrupted call's number and arguments.
    ///
    pub fn set_running_thread_restart(&mut self, restart: KcallRestart) {
        let tid: ThreadIdentifier = self.get_tid();
        if let Some(mut thread) = self.get_running_mut().find_thread_mut(tid) {
            thread.thread_state_mut().set_restart(restart);
        }
    }

    ///
    /// # Description
    ///
    /// Evaluates and, if warranted, performs delivery of a pending caught signal to the running
    /// thread at the kernel-call return-to-user boundary.
    ///
    /// # Parameters
    ///
    /// - `result`: The return value the interrupted kernel call will deliver to user space (saved
    ///   as the interrupted accumulator so it survives the handler).
    ///
    /// # Returns
    ///
    /// A [`SignalDeliveryOutcome`] describing what happened.
    ///
    pub fn try_deliver_signal(&mut self, result: i64) -> SignalDeliveryOutcome {
        let pid: ProcessIdentifier = self.get_pid();
        let tid: ThreadIdentifier = self.get_tid();

        // Locate the running thread's kernel stack and read its blocked mask.
        let owner_tid: ThreadIdentifier = ThreadIdentifier::from(FPU_OWNER_TID.load(ORDER));
        let (esp0, blocked, restart): (usize, u64, Option<KcallRestart>) = {
            let mut thread = match self.get_running_mut().find_thread_mut(tid) {
                Some(thread) => thread,
                None => return SignalDeliveryOutcome::None,
            };
            let state = thread.thread_state_mut();
            // Consume any restart record now so it never lingers past this kernel-call boundary,
            // even if no signal turns out to be deliverable below.
            let restart: Option<KcallRestart> = state.take_restart();
            let blocked: u64 = state.blocked();
            match state.kernel_stack_top() {
                Some(esp0) => (esp0.into_raw_value(), blocked, restart),
                None => return SignalDeliveryOutcome::None,
            }
        };

        // Deliver only when the kernel-call return resumes in user mode.
        if !unsafe { returning_to_user(esp0) } {
            return SignalDeliveryOutcome::None;
        }

        // Select the lowest-numbered deliverable, caught signal and capture its handler. Pending
        // signals whose disposition is not a handler (e.g. job-control stop/continue that `kill()`
        // records for a later phase) are left pending and skipped, so they are neither discarded nor
        // allowed to mask a higher-numbered caught signal in the same set.
        let (signum, entry, sa_mask, sa_flags): (usize, usize, u64, i32) = {
            let signals = self.get_running_mut().state_mut().signals_mut();
            let mut deliverable: u64 = signals.pending() & !blocked;
            loop {
                if deliverable == 0 {
                    return SignalDeliveryOutcome::None;
                }
                let signum: usize = (deliverable.trailing_zeros() as usize) + 1;
                if let Some(SignalDisposition::Handler(handler)) = signals.disposition(signum) {
                    break (signum, handler.entry.into_raw_value(), handler.mask, handler.flags);
                }
                // Not a caught signal: leave it pending for its own phase and consider the next.
                deliverable &= deliverable - 1;
            }
        };

        // The handler return address must be a registered restorer trampoline.
        let restorer: usize = match self.get_running_mut().state_mut().signals_mut().restorer() {
            Some(restorer) => restorer.into_raw_value(),
            None => {
                error!("no signal restorer registered (pid={pid:?}, signum={signum})");
                self.get_running_mut()
                    .state_mut()
                    .signals_mut()
                    .clear_pending(signum);
                return SignalDeliveryOutcome::None;
            },
        };

        // Snapshot the interrupted CPU context and place the frame on the user stack.
        let mut cpu: SignalCpuContext = unsafe { read_trap_context(esp0, result) };

        // If a blocking call was interrupted by this signal and the handler that is about to run is
        // installed with SA_RESTART, rewind the saved context to the kernel-call trap and reload the
        // original argument registers, so the call transparently re-executes after the handler
        // returns. Without SA_RESTART the recorded EINTR result (already in the saved accumulator)
        // stands and the record is simply discarded.
        if let Some(restart) = restart {
            if (sa_flags & SA_RESTART) != 0 {
                prepare_kcall_restart(&mut cpu, restart.number, restart.args);
            }
        }

        let user_sp: usize = cpu.sp as usize;
        let layout: FrameLayout = match frame_layout(user_sp) {
            Some(layout) => layout,
            None => return SignalDeliveryOutcome::Escalate,
        };
        let frame_size: usize =
            RETADDR_SIZE + save_area_offset_from_sigreturn_sp() + core::mem::size_of::<SigFrame>();
        // The target stack address derives from user-controlled state, so confirm the whole frame
        // region is mapped and writable before writing it. An unmapped or read-only page escalates
        // to the signal's default action instead of faulting the kernel's physical-alias write
        // path while building the frame.
        let vmem: &Vmem = self.get_running().state().vmem();
        if !vmem.is_user_region_writable(VirtualAddress::new(layout.frame_top), frame_size) {
            return SignalDeliveryOutcome::Escalate;
        }

        // Capture the FPU state of the interrupted thread.
        let fpu: [u8; FPU_AREA_SIZE] = self.snapshot_thread_fpu(tid, owner_tid == tid);

        // Build the frame, saving the mask that was in effect before delivery.
        let has_siginfo: bool = (sa_flags & SA_SIGINFO) != 0;
        let frame: SigFrame = build_frame(cpu, blocked, fpu, signum, has_siginfo);

        // Copy the save area to user space.
        if self
            .vmcopy_to_user(
                pid,
                VirtualAddress::new(layout.save_area_base),
                VirtualAddress::new(&frame as *const SigFrame as usize),
                core::mem::size_of::<SigFrame>(),
            )
            .is_err()
        {
            return SignalDeliveryOutcome::Escalate;
        }

        // Copy the return address and the on-stack handler arguments below the save area.
        let (info_ptr, ctx_ptr): (usize, usize) = if has_siginfo {
            (
                layout.save_area_base + sigframe::siginfo_offset(),
                layout.save_area_base + sigframe::ctx_offset(),
            )
        } else {
            (0, 0)
        };
        let args: [usize; 4] = [restorer, signum, info_ptr, ctx_ptr];
        if self
            .vmcopy_to_user(
                pid,
                VirtualAddress::new(layout.frame_top),
                VirtualAddress::new(args.as_ptr() as usize),
                core::mem::size_of::<[usize; 4]>(),
            )
            .is_err()
        {
            return SignalDeliveryOutcome::Escalate;
        }

        // Commit the signal-state changes now that the frame is in place.
        let nodefer: bool = (sa_flags & SA_NODEFER) != 0;
        let new_blocked: u64 = next_blocked(blocked, sa_mask, signum, nodefer) & !UNBLOCKABLE;
        {
            if let Some(mut thread) = self.get_running_mut().find_thread_mut(tid) {
                thread.thread_state_mut().set_blocked(new_blocked);
            }
        }
        {
            let signals = self.get_running_mut().state_mut().signals_mut();
            signals.clear_pending(signum);
            if (sa_flags & SA_RESETHAND) != 0 {
                signals.set_disposition(signum, SignalDisposition::Default);
            }
        }

        // Redirect the interrupted thread into its handler.
        unsafe { redirect_to_handler(esp0, entry, layout.frame_top) };

        SignalDeliveryOutcome::Delivered
    }

    ///
    /// # Description
    ///
    /// Restores the interrupted context saved in the signal frame on the running thread's user
    /// stack, completing a `sigreturn()` kernel call.
    ///
    /// # Returns
    ///
    /// On success, the value to leave in the return register (the interrupted accumulator). On
    /// failure, a [`SigReturnFailure`].
    ///
    pub fn sigreturn_restore(&mut self) -> Result<i64, SigReturnFailure> {
        // Architectures without a delivery path ship inert trap-frame placeholders, so a frame can
        // never have been built; treat `sigreturn()` there as an unsupported kernel call instead of
        // running the placeholder restore path and terminating the process as if the frame were
        // forged.
        if !cfg!(target_arch = "x86") {
            return Err(SigReturnFailure::Unsupported);
        }

        let pid: ProcessIdentifier = self.get_pid();
        let tid: ThreadIdentifier = self.get_tid();

        let owner_tid: ThreadIdentifier = ThreadIdentifier::from(FPU_OWNER_TID.load(ORDER));
        let esp0: usize = {
            let mut thread = match self.get_running_mut().find_thread_mut(tid) {
                Some(thread) => thread,
                None => return Err(SigReturnFailure::Forged),
            };
            match thread.thread_state_mut().kernel_stack_top() {
                Some(esp0) => esp0.into_raw_value(),
                None => return Err(SigReturnFailure::Forged),
            }
        };

        // The restorer issues `sigreturn()` without adjusting the stack pointer, so the save area
        // sits just above the on-stack arguments.
        let user_sp: usize = unsafe { read_user_sp(esp0) };
        let save_area: usize = user_sp + save_area_offset_from_sigreturn_sp();

        // Copy the frame in and validate it.
        let mut frame: SigFrame = SigFrame {
            magic: 0,
            has_siginfo: 0,
            blocked: 0,
            siginfo: [0u32; 8],
            cpu: SignalCpuContext::default(),
            fpu: [0u8; FPU_AREA_SIZE],
        };
        if self
            .vmcopy_from_user(
                pid,
                VirtualAddress::new(&mut frame as *mut SigFrame as usize),
                VirtualAddress::new(save_area),
                core::mem::size_of::<SigFrame>(),
            )
            .is_err()
        {
            return Err(SigReturnFailure::Forged);
        }
        let cpu: SignalCpuContext = match validate_and_restore(&frame) {
            Ok(cpu) => cpu,
            Err(_) => return Err(SigReturnFailure::Forged),
        };

        // Restore the blocked mask and FPU state, then rewrite the trap frame to resume. If a
        // `sigsuspend()` is unwinding through this return, reinstate the mask it saved (restoring
        // the pre-suspend mask now that its interrupting handler has run) instead of the frame's
        // saved mask.
        {
            if let Some(mut thread) = self.get_running_mut().find_thread_mut(tid) {
                let state = thread.thread_state_mut();
                let restored_blocked: u64 = match state.take_saved_blocked() {
                    Some(saved) => saved & !UNBLOCKABLE,
                    None => frame.blocked & !UNBLOCKABLE,
                };
                state.set_blocked(restored_blocked);
            }
        }
        self.restore_thread_fpu(tid, owner_tid == tid, &frame.fpu);
        unsafe { restore_trap_context(esp0, &cpu) };

        Ok(join_kcall_result(cpu.ax, cpu.dx))
    }

    ///
    /// # Description
    ///
    /// Returns the FPU image of thread `tid`, refreshing the kernel-side copy from the live FPU
    /// first when the thread currently owns it.
    ///
    fn snapshot_thread_fpu(
        &mut self,
        tid: ThreadIdentifier,
        is_owner: bool,
    ) -> [u8; FPU_AREA_SIZE] {
        let mut image: [u8; FPU_AREA_SIZE] = [0u8; FPU_AREA_SIZE];
        if let Some(mut thread) = self.get_running_mut().find_thread_mut(tid) {
            let fpu_ptr = thread.thread_state_mut().fpu_state_mut();
            image = unsafe { capture_fpu(fpu_ptr, is_owner) };
        }
        image
    }

    ///
    /// # Description
    ///
    /// Writes `image` into thread `tid`'s FPU state, and reloads the live FPU when the thread
    /// currently owns it so the restored state takes effect immediately.
    ///
    fn restore_thread_fpu(
        &mut self,
        tid: ThreadIdentifier,
        is_owner: bool,
        image: &[u8; FPU_AREA_SIZE],
    ) {
        if let Some(mut thread) = self.get_running_mut().find_thread_mut(tid) {
            let fpu_ptr = thread.thread_state_mut().fpu_state_mut();
            unsafe { install_fpu(fpu_ptr, is_owner, image) };
        }
    }
}
