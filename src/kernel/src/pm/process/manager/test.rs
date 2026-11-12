// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::delivery::{
    DeliveryBroker,
    DeliverySequence,
};
#[cfg(target_arch = "x86")]
use crate::hal::arch::{
    join_kcall_result,
    split_kcall_result,
};
use crate::{
    hal::arch::{
        x86::cpu::FpuState,
        ContextInformation,
    },
    ipc::Mailbox,
    mm::{
        elf::Elf32Fhdr,
        VirtMemoryManager,
        Vmem,
    },
    pm::{
        process::{
            new_test_process_termination_credit,
            new_test_thread_termination_credit,
            state::{
                signal::{
                    SignalControl,
                    SignalDisposition,
                    SignalHandler,
                },
                PendingProcessTermination,
                RunnableProcess,
                RunningProcess,
                ZombieProcess,
            },
            LifecycleCreationReservation,
            LifecycleTerminationCredit,
            ThreadLifecycleReservation,
            ThreadLifecycleTerminationCredit,
        },
        thread::{
            ReadyThread,
            ThreadManager,
        },
        ProcessManager,
    },
};
use ::alloc::boxed::Box;
use ::config::kernel::SCHEDULER_FREQ;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    event::{
        ProcessCreationInfo,
        ProcessRole,
        ProcessTerminationInfo,
        ThreadTerminationInfo,
    },
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    mm::VirtualAddress,
    pm::{
        Capability,
        ProcessIdentifier,
        SigSet,
        ThreadCreateArgs,
        ThreadIdentifier,
        SIGCHLD,
        SIGTERM,
        SIGUSR1,
        SIGUSR2,
        SIGWINCH,
    },
    ExitStatus,
};

//==================================================================================================
// Process Manager Test Helpers
//==================================================================================================

impl ProcessManager {
    ///
    /// # Description
    ///
    /// Injects one failure into the next standalone thread or `execv()` thread preparation.
    ///
    fn inject_thread_lifecycle_creation_failure(&mut self) {
        self.fail_next_thread_lifecycle_creation = true;
    }

    ///
    /// # Description
    ///
    /// Injects one failure into the next process creation, after it reserves lifecycle capacity.
    ///
    fn inject_lifecycle_creation_failure(&mut self) {
        self.fail_next_lifecycle_creation = true;
    }

    ///
    /// # Description
    ///
    /// Reports whether the currently running process owns any "special" resources, as
    /// defined by [`crate::pm::process::state::ProcessState::has_special_resources`].
    ///
    /// Thin test-only accessor used by in-kernel tests; it avoids widening the visibility of
    /// [`Self::get_running`]. The `duplicate()` kernel call reaches the same predicate
    /// directly through its internal `get_running()` view.
    ///
    pub fn current_has_special_resources(&self) -> bool {
        self.get_running().state().has_special_resources()
    }

    ///
    /// # Description
    ///
    /// Returns a shared reference to the currently running process's virtual address space.
    ///
    /// Thin test-only accessor used by in-kernel tests that need a reference [`Vmem`] to clone
    /// from (e.g. via [`crate::mm::VirtMemoryManager::new_vmem`]). Production code paths reach
    /// this through the internal [`Self::get_running`] view instead.
    ///
    pub fn current_vmem(&self) -> &Vmem {
        self.get_running().state().vmem()
    }

    /// Queues a synthetic creation record without retaining a future termination credit.
    pub(crate) fn queue_test_process_creation(
        &mut self,
        info: ProcessCreationInfo,
    ) -> Result<(), Error> {
        let reservation: LifecycleCreationReservation = self.delivery.try_reserve_creation()?;
        let credit: LifecycleTerminationCredit = self.delivery.commit_creation(reservation, info);
        self.delivery.release_termination(credit);
        Ok(())
    }
}

//==================================================================================================
// Test Helpers
//==================================================================================================

fn test_pid() -> ProcessIdentifier {
    ProcessIdentifier::from(1000)
}

fn test_tid() -> ThreadIdentifier {
    ThreadIdentifier::from(1000)
}

/// Returns the bit for signal `signum` (1-based) in a signal set.
fn signal_bit(signum: usize) -> SigSet {
    1u64 << (signum - 1)
}

///
/// # Description
///
/// Creates a lifecycle broker fixture for an in-kernel test.
///
/// # Returns
///
/// The broker fixture, or [`None`] if it could not be created.
///
fn new_broker() -> Option<DeliveryBroker> {
    match DeliveryBroker::new() {
        Ok(broker) => Some(broker),
        Err(error) => {
            error!("failed to create lifecycle broker fixture (error={error:?})");
            None
        },
    }
}

fn creation_info() -> ProcessCreationInfo {
    ProcessCreationInfo::new(test_pid(), ProcessIdentifier::KERNEL, ProcessRole::User)
}

fn termination_info() -> ProcessTerminationInfo {
    ProcessTerminationInfo::new(
        test_pid(),
        ExitStatus::ok(),
        ProcessIdentifier::KERNEL,
        ProcessRole::User,
    )
}

fn ipc_message(receiver: MessageReceiver) -> Message {
    Message {
        source: MessageSender::KERNEL,
        destination: receiver,
        message_type: MessageType::Ipc,
        ..Message::default()
    }
}

fn commit_creation(broker: &mut DeliveryBroker) -> Option<LifecycleTerminationCredit> {
    let reservation: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("failed to reserve lifecycle creation capacity (error={error:?})");
            return None;
        },
    };
    Some(broker.commit_creation(reservation, creation_info()))
}

///
/// # Description
///
/// Selects and commits the lifecycle record at the head of a delivery broker.
///
/// # Parameters
///
/// - `broker`: Delivery broker from which to receive a lifecycle record.
///
/// # Returns
///
/// The committed lifecycle message, or [`None`] if no lifecycle record is buffered.
///
fn receive_lifecycle(broker: &mut DeliveryBroker) -> Option<Message> {
    let (sequence, message): (DeliverySequence, Message) = broker.peek_lifecycle(test_pid())?;
    broker.commit_lifecycle(sequence);
    Some(message)
}

///
/// # Description
///
/// Receives one lifecycle message and verifies its type and serialized payload.
///
/// # Parameters
///
/// - `broker`: Delivery broker from which to receive the lifecycle record.
/// - `message_type`: Expected message type.
/// - `expected_payload`: Expected serialized payload prefix.
///
/// # Returns
///
/// `true` if the next lifecycle message matches the expected type and payload, otherwise `false`.
///
/// # Panics
///
/// This function panics if `expected_payload` is larger than [`Message::PAYLOAD_SIZE`].
///
fn receive_expected_lifecycle(
    broker: &mut DeliveryBroker,
    message_type: MessageType,
    expected_payload: &[u8],
) -> bool {
    let message: Message = match receive_lifecycle(broker) {
        Some(message) => message,
        None => {
            error!("expected lifecycle record was not queued (type={message_type:?})");
            return false;
        },
    };
    if message.message_type != message_type
        || message.payload[..expected_payload.len()] != *expected_payload
    {
        error!("lifecycle record did not match expected type or payload");
        return false;
    }
    true
}

///
/// # Description
///
/// Creates a local process manager whose running process is a single credited user thread.
///
/// # Returns
///
/// The local process-manager fixture, or [`None`] if its resources could not be prepared.
///
fn new_user_process_manager() -> Option<ProcessManager> {
    new_process_manager_running(ProcessIdentifier::from(1), ProcessIdentifier::KERNEL)
}

///
/// # Description
///
/// Creates a local process manager whose running process is a single credited user thread with a
/// given identity.
///
/// # Parameters
///
/// - `pid`: Identifier of the running process.
/// - `parent`: Identifier of the parent of the running process.
///
/// # Returns
///
/// The local process-manager fixture, or [`None`] if its resources could not be prepared.
///
fn new_process_manager_running(
    pid: ProcessIdentifier,
    parent: ProcessIdentifier,
) -> Option<ProcessManager> {
    // SAFETY: process-manager tests run on one core with interrupts disabled and hold no other
    // reference to either global manager.
    let global_pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
    let root: Vmem = match mm.new_vmem(global_pm.current_vmem()) {
        Ok(vmem) => vmem,
        Err(error) => {
            error!("failed to create local process-manager root vmem (error={error:?})");
            return None;
        },
    };
    let user_vmem: Vmem = match mm.new_vmem(global_pm.current_vmem()) {
        Ok(vmem) => vmem,
        Err(error) => {
            error!("failed to create local user-process vmem (error={error:?})");
            return None;
        },
    };
    let (kernel, tm): (ReadyThread, ThreadManager) = crate::pm::thread::init();
    let mut manager: ProcessManager = match ProcessManager::new(false, kernel, root, tm) {
        Ok(manager) => manager,
        Err(error) => {
            error!("failed to create local process manager (error={error:?})");
            return None;
        },
    };
    let (tid, next_tid): (ThreadIdentifier, ThreadIdentifier) = match manager.tm.try_next_tid() {
        Ok(ids) => ids,
        Err(error) => {
            error!("failed to reserve local user thread identifier (error={error:?})");
            return None;
        },
    };
    let thread: ReadyThread = ReadyThread::new(
        tid,
        Some(new_test_thread_termination_credit()),
        None,
        None,
        None,
        ContextInformation::default(),
        // SAFETY: calls to FpuState::new are synchronized by the single-threaded test runner.
        unsafe { FpuState::new() },
    );
    let mut process: RunnableProcess =
        RunnableProcess::new_uncommitted(pid, parent, thread, user_vmem);
    process.install_termination_credit(new_test_process_termination_credit());
    let (running, reason, _ctx, _user_tda): (
        RunningProcess,
        Option<crate::pm::thread::InterruptReason>,
        *mut ContextInformation,
        Option<VirtualAddress>,
    ) = process.run();
    if reason.is_some() {
        error!("local user process was unexpectedly interrupted");
        return None;
    }
    manager.tm.commit_next_tid(next_tid);
    manager.running = Some(running);
    Some(manager)
}

///
/// # Description
///
/// Snapshots state that must remain unchanged after failed standalone thread preparation.
///
/// # Parameters
///
/// - `pm`: Process manager to snapshot.
///
/// # Returns
///
/// A tuple containing the next thread identifiers, lifecycle capacity in use, running process and
/// thread identifiers, and ready-queue length.
///
/// # Panics
///
/// This function panics if `pm` does not contain a running process.
///
fn thread_creation_state(
    pm: &ProcessManager,
) -> (
    Option<(ThreadIdentifier, ThreadIdentifier)>,
    Option<usize>,
    ProcessIdentifier,
    ThreadIdentifier,
    usize,
) {
    (
        pm.tm.try_next_tid().ok(),
        pm.delivery.capacity_in_use(),
        pm.get_running().state().pid(),
        pm.get_running().get_tid(),
        pm.ready.len(),
    )
}

///
/// # Description
///
/// Disarms and reports whether the standalone thread-preparation injection was consumed.
///
/// # Parameters
///
/// - `pm`: Process manager whose injection flag is disarmed.
///
/// # Returns
///
/// `true` if the injection was consumed by the call under test, otherwise `false`.
///
fn thread_injection_was_consumed(pm: &mut ProcessManager) -> bool {
    !::core::mem::take(&mut pm.fail_next_thread_lifecycle_creation)
}

///
/// # Description
///
/// Builds arguments that create a user thread in a local process-manager fixture.
///
/// # Returns
///
/// Thread-creation arguments whose entry point and stack lie in the user address space.
///
fn test_thread_create_args() -> ThreadCreateArgs {
    ThreadCreateArgs {
        user_fn: VirtualAddress::from_raw_value(::config::memory_layout::USER_BASE_RAW),
        user_fn_arg0: 0,
        user_fn_arg1: 0,
        user_stack_base: VirtualAddress::from_raw_value(
            ::config::memory_layout::USER_STACK_TOP_RAW - ::arch::mem::PAGE_SIZE,
        ),
        user_stack_size: ::arch::mem::PAGE_SIZE,
        user_tda: None,
    }
}

///
/// # Description
///
/// Creates a thread in the running process through the production creation path.
///
/// # Parameters
///
/// - `pm`: Process manager whose running process receives the thread.
///
/// # Returns
///
/// Upon success, the identifier of the new thread is returned. Otherwise, the creation error is
/// returned instead.
///
fn create_test_thread(pm: &mut ProcessManager) -> Result<ThreadIdentifier, Error> {
    let pid: ProcessIdentifier = pm.get_running().state().pid();
    // SAFETY: the local process manager is not aliased and tests run with interrupts disabled.
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
    pm.create_thread(mm, pid, &test_thread_create_args())
}

///
/// # Description
///
/// Detaches a ready thread of the running process, runs it, terminates it through the production
/// thread-exit transition, switches back to the calling thread, and reaps the zombie through the
/// production reaper that thread admission drains. Termination alone keeps the thread's execution
/// capacity and only the reap releases it, while the termination record of the thread stays queued
/// in the delivery broker until a consumer receives it.
///
/// # Parameters
///
/// - `pm`: Process manager whose running process owns the thread.
/// - `tid`: Identifier of the ready thread to terminate.
/// - `status`: Exit status of the thread.
///
/// # Returns
///
/// `true` if the thread terminated and the reaper reaped it, otherwise `false`.
///
fn exit_and_reap_thread(
    pm: &mut ProcessManager,
    tid: ThreadIdentifier,
    status: ExitStatus,
) -> bool {
    let pid: ProcessIdentifier = pm.get_running().state().pid();
    if !matches!(pm.do_detach_thread(pid, tid), Ok(None)) {
        error!("failed to detach a live thread (tid={tid:?})");
        return false;
    }
    let live_before_exit: usize = pm.tm.live_count();

    let caller: RunningProcess = pm.take_running();
    let caller_tid: ThreadIdentifier = caller.get_tid();
    let (runnable, _ctx) = caller.schedule();
    // A freshly created thread was admitted before the caller was rescheduled, so it runs first.
    let (running, reason, _ctx, _user_tda) = runnable.run();
    if reason.is_some() || running.get_tid() != tid {
        error!("expected thread {tid:?} to run (running={:?})", running.get_tid());
        return false;
    }

    let delivery: &mut DeliveryBroker = &mut pm.delivery;
    let (transition, deferred) = running.exit_thread(status, &mut |pending| {
        ProcessManager::commit_thread_termination(delivery, pending);
    });
    // As in the production thread-exit path, the zombie of a detached thread waits for the reaper
    // until the switch away from its context completes.
    let runnable: RunnableProcess = match (transition, deferred) {
        (Ok((_join_cond, runnable, _ctx)), Some(zombie)) => {
            pm.deferred_reap.push((pid, zombie));
            runnable
        },
        _ => {
            error!("detached thread exit left an unexpected process state (tid={tid:?})");
            return false;
        },
    };
    let (running, reason, _ctx, _user_tda) = runnable.run();
    if reason.is_some() || running.get_tid() != caller_tid {
        error!("calling thread did not resume after thread exit (tid={tid:?})");
        return false;
    }
    pm.running = Some(running);

    if pm.tm.live_count() != live_before_exit {
        error!("thread termination released execution capacity before the reap (tid={tid:?})");
        return false;
    }
    // SAFETY: the local process manager is not aliased and tests run with interrupts disabled.
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
    if pm.reap_pending_zombies(mm) != 1 {
        error!("thread admission reaper did not reap the terminated thread (tid={tid:?})");
        return false;
    }
    true
}

//==================================================================================================
// Tests
//==================================================================================================

///
/// # Description
///
/// Verifies that an intra-process context switch that follows the exhaustion of the outgoing
/// thread's quantum resets the quantum for the incoming thread.
///
/// This is a regression test for the quantum-inheritance starvation bug (issue #1695): when a
/// thread is preempted because its quantum reached zero and the scheduler selects another thread of
/// the same process, the incoming thread must not inherit the exhausted quantum. Otherwise it is
/// immediately preempted on the next tick and is permanently starved by its sibling threads.
///
fn test_intra_process_switch_resets_exhausted_quantum() -> bool {
    let pid: ProcessIdentifier = ProcessIdentifier::from(1);

    // The outgoing thread exhausted its quantum (remaining == 0) and the scheduler selected another
    // thread of the same process. The incoming thread must start with a fresh quantum.
    let quantum: usize = ProcessManager::next_thread_quantum(pid, pid, 0);
    if quantum != SCHEDULER_FREQ {
        error!(
            "intra-process switch after quantum exhaustion did not reset the quantum (got {}, \
             expected {})",
            quantum, SCHEDULER_FREQ
        );
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that an intra-process context switch that follows a voluntary yield (the outgoing
/// thread still had quantum left) preserves the remaining quantum for the incoming thread.
///
/// This ensures the fix for issue #1695 does not regress into unconditionally resetting the quantum
/// on every thread switch (the naive fix), which would let a process whose threads frequently yield
/// accumulate more than its fair share of CPU time and starve other processes.
///
fn test_intra_process_switch_preserves_remaining_quantum() -> bool {
    let pid: ProcessIdentifier = ProcessIdentifier::from(1);

    // Pick a remaining quantum that is strictly between zero and a full quantum.
    let remaining: usize = SCHEDULER_FREQ / 2;
    let quantum: usize = ProcessManager::next_thread_quantum(pid, pid, remaining);
    if quantum != remaining {
        error!(
            "intra-process voluntary yield did not preserve the remaining quantum (got {}, \
             expected {})",
            quantum, remaining
        );
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that a cross-process context switch always starts the incoming process with a fresh
/// quantum, regardless of how much quantum the outgoing thread had left.
///
fn test_cross_process_switch_resets_quantum() -> bool {
    let previous_pid: ProcessIdentifier = ProcessIdentifier::from(1);
    let next_pid: ProcessIdentifier = ProcessIdentifier::from(2);

    // A cross-process switch must reset the quantum whether or not the outgoing thread had quantum
    // left.
    for remaining in [0, SCHEDULER_FREQ / 2, SCHEDULER_FREQ] {
        let quantum: usize = ProcessManager::next_thread_quantum(next_pid, previous_pid, remaining);
        if quantum != SCHEDULER_FREQ {
            error!(
                "cross-process switch did not reset the quantum (remaining={}, got {}, expected \
                 {})",
                remaining, quantum, SCHEDULER_FREQ
            );
            return false;
        }
    }

    true
}

///
/// # Description
///
/// Snapshots process-creation state that must not change when lifecycle reservation preparation
/// fails.
///
/// # Parameters
///
/// - `pm`: Process manager to snapshot.
///
/// # Returns
///
/// A tuple of the next process identifier, live count, ready-queue length, and lifecycle capacity
/// in use.
///
fn process_creation_state(pm: &ProcessManager) -> (ProcessIdentifier, usize, usize, Option<usize>) {
    (pm.next_pid, pm.live_count, pm.ready.len(), pm.delivery.capacity_in_use())
}

///
/// # Description
///
/// Disarms the injected creation failure and reports whether the call under test consumed it. An
/// unconsumed injection means the entry point returned before reserving lifecycle capacity, which
/// would leave the injection armed and break the next real process creation.
///
/// # Parameters
///
/// - `pm`: Process manager whose injection flag is disarmed.
///
/// # Returns
///
/// `true` if the injection was consumed by the call under test, otherwise `false`.
///
fn injection_was_consumed(pm: &mut ProcessManager) -> bool {
    !::core::mem::take(&mut pm.fail_next_lifecycle_creation)
}

/// Verifies `create_process()` rolls back both lifecycle reservations on an injected failure.
fn test_create_process_reservation_failure_rolls_back() -> bool {
    let elf: Elf32Fhdr = Elf32Fhdr {
        e_ident: [0u8; 16],
        e_type: 0,
        e_machine: 0,
        e_version: 0,
        e_entry: 0,
        e_phoff: 0,
        e_shoff: 0,
        e_flags: 0,
        e_ehsize: 0,
        e_phentsize: 0,
        e_phnum: 0,
        e_shentsize: 0,
        e_shnum: 0,
        e_shstrndx: 0,
    };
    // SAFETY: process-manager tests run on one core with interrupts disabled and hold no other
    // reference to either global manager.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
    let before = process_creation_state(pm);
    pm.inject_lifecycle_creation_failure();

    match pm.create_process(mm, &elf, "", "") {
        Err(error) if error.code == ::sys::error::ErrorCode::OutOfMemory => {},
        Err(error) => {
            error!("injected create-process failure returned wrong error (error={error:?})");
            return false;
        },
        Ok(pid) => {
            error!("create_process() ignored injected reservation failure (pid={pid:?})");
            return false;
        },
    }
    if !injection_was_consumed(pm) {
        error!("create_process() failed before reserving lifecycle capacity");
        return false;
    }
    if process_creation_state(pm) != before {
        error!("create_process() changed state after reservation rollback");
        return false;
    }
    true
}

/// Verifies `duplicate_process()` rolls back both lifecycle reservations on an injected failure.
fn test_duplicate_process_reservation_failure_rolls_back() -> bool {
    let args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: VirtualAddress::from_raw_value(::config::memory_layout::USER_BASE_RAW),
        user_fn_arg0: 0,
        user_fn_arg1: 0,
        user_stack_base: VirtualAddress::from_raw_value(
            ::config::memory_layout::USER_STACK_TOP_RAW,
        ),
        user_stack_size: ::arch::mem::PAGE_SIZE,
        user_tda: None,
    };
    // SAFETY: process-manager tests run on one core with interrupts disabled and hold no other
    // reference to either global manager.
    let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
    let before = process_creation_state(pm);
    pm.inject_lifecycle_creation_failure();

    match pm.duplicate_process(mm, ProcessIdentifier::KERNEL, &args) {
        Err(error) if error.code == ::sys::error::ErrorCode::OutOfMemory => {},
        Err(error) => {
            error!("injected duplicate failure returned wrong error (error={error:?})");
            return false;
        },
        Ok(pid) => {
            error!("duplicate_process() ignored injected reservation failure (pid={pid:?})");
            return false;
        },
    }
    if !injection_was_consumed(pm) {
        error!("duplicate_process() failed before reserving lifecycle capacity");
        return false;
    }
    if process_creation_state(pm) != before {
        error!("duplicate_process() changed state after reservation rollback");
        return false;
    }
    true
}

///
/// # Description
///
/// Verifies ordinary thread preparation releases its reservation on failure.
///
/// # Returns
///
/// `true` if failed preparation leaves identifiers, process state, and lifecycle capacity
/// unchanged, otherwise `false`.
///
fn test_create_thread_reservation_failure_rolls_back() -> bool {
    let Some(mut pm) = new_user_process_manager() else {
        return false;
    };
    let pid: ProcessIdentifier = pm.get_running().state().pid();
    let args: ThreadCreateArgs = ThreadCreateArgs {
        user_fn: VirtualAddress::from_raw_value(::config::memory_layout::USER_BASE_RAW),
        user_fn_arg0: 0,
        user_fn_arg1: 0,
        user_stack_base: VirtualAddress::from_raw_value(
            ::config::memory_layout::USER_STACK_TOP_RAW - ::arch::mem::PAGE_SIZE,
        ),
        user_stack_size: ::arch::mem::PAGE_SIZE,
        user_tda: None,
    };
    let before = thread_creation_state(&pm);
    pm.inject_thread_lifecycle_creation_failure();
    // SAFETY: the local process manager is not aliased and tests run with interrupts disabled.
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
    match pm.create_thread(mm, pid, &args) {
        Err(error) if error.code == ::sys::error::ErrorCode::OutOfMemory => {},
        Err(error) => {
            error!("injected create-thread failure returned wrong error (error={error:?})");
            return false;
        },
        Ok(tid) => {
            error!("create_thread() ignored injected reservation failure (tid={tid:?})");
            return false;
        },
    }
    if !thread_injection_was_consumed(&mut pm) || thread_creation_state(&pm) != before {
        error!("create_thread() changed state or leaked capacity after reservation rollback");
        return false;
    }
    true
}

///
/// # Description
///
/// Verifies `execv()` replacement-thread preparation releases its reservation on failure.
///
/// # Returns
///
/// `true` if failed replacement preparation leaves the process and lifecycle capacity unchanged,
/// otherwise `false`.
///
fn test_execv_thread_reservation_failure_rolls_back() -> bool {
    let Some(mut pm) = new_user_process_manager() else {
        return false;
    };
    let pid: ProcessIdentifier = pm.get_running().state().pid();
    let before = thread_creation_state(&pm);
    pm.inject_thread_lifecycle_creation_failure();
    // SAFETY: the local process manager is not aliased and tests run with interrupts disabled.
    let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };
    match pm.do_execv(
        mm,
        pid,
        VirtualAddress::from_raw_value(::config::memory_layout::USER_BASE_RAW),
        1,
        "",
        "",
    ) {
        Err(error) if error.code == ::sys::error::ErrorCode::OutOfMemory => {},
        Err(error) => {
            error!("injected execv preparation failure returned wrong error (error={error:?})");
            return false;
        },
        Ok(_) => {
            error!("execv() ignored injected thread reservation failure");
            return false;
        },
    }
    if !thread_injection_was_consumed(&mut pm) || thread_creation_state(&pm) != before {
        error!("execv() changed state or leaked capacity after thread reservation rollback");
        return false;
    }
    true
}

///
/// # Description
///
/// Verifies that reaping a terminated thread releases its execution capacity while its lifecycle
/// reservation stays held by the undelivered termination record until a consumer receives it.
///
/// # Returns
///
/// `true` if reap and delivery release execution and lifecycle capacity independently, otherwise
/// `false`.
///
fn test_reap_releases_execution_capacity_before_delivery() -> bool {
    let Some(mut pm) = new_user_process_manager() else {
        return false;
    };
    let pid: ProcessIdentifier = pm.get_running().state().pid();
    let status: ExitStatus = ExitStatus::from(0x0305_5000_u32);
    let live_before: usize = pm.tm.live_count();
    let lifecycle_before: Option<usize> = pm.delivery.capacity_in_use();

    let tid: ThreadIdentifier = match create_test_thread(&mut pm) {
        Ok(tid) => tid,
        Err(error) => {
            error!("failed to create lifecycle capacity fixture thread (error={error:?})");
            return false;
        },
    };
    if pm.tm.live_count() != live_before + 1
        || pm.delivery.capacity_in_use() != lifecycle_before.map(|in_use| in_use + 1)
        || pm.delivery.has_lifecycle()
    {
        error!("thread creation did not reserve execution and lifecycle capacity");
        return false;
    }

    if !exit_and_reap_thread(&mut pm, tid, status) {
        return false;
    }
    if pm.tm.live_count() != live_before {
        error!("reaping a terminated thread did not release its execution capacity");
        return false;
    }
    if pm.delivery.capacity_in_use() != lifecycle_before.map(|in_use| in_use + 1)
        || !pm.delivery.has_lifecycle()
    {
        error!("reaping released lifecycle capacity before its record was delivered");
        return false;
    }

    let expected: ThreadTerminationInfo = ThreadTerminationInfo::new(pid, tid, status);
    if !receive_expected_lifecycle(
        &mut pm.delivery,
        MessageType::ThreadTerminationEvent,
        &expected.to_ne_bytes(),
    ) {
        return false;
    }
    if pm.delivery.capacity_in_use() != lifecycle_before || pm.delivery.has_lifecycle() {
        error!("delivering the termination record did not release its lifecycle capacity");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that a stalled lifecycle consumer backpressures thread creation deterministically and
/// without losing records. Each cycle creates, terminates, and reaps a thread, so execution
/// capacity stays available while undelivered termination records accumulate. Once every thread
/// lifecycle reservation is held by a record, creation fails without side effects until a delivery
/// releases one, and then every record is delivered exactly once in production order.
///
/// # Returns
///
/// `true` if backpressure is deterministic, creation resumes after delivery, and no record is lost,
/// otherwise `false`.
///
fn test_stalled_consumer_backpressures_thread_creation_without_loss() -> bool {
    let Some(mut pm) = new_user_process_manager() else {
        return false;
    };
    let pid: ProcessIdentifier = pm.get_running().state().pid();
    // Kernel-heap allocations are bounded by the slab size, so records are tracked on the stack.
    let mut produced: [Option<ThreadTerminationInfo>; ::config::kernel::MAX_THREADS] =
        [None; ::config::kernel::MAX_THREADS];
    let mut count: usize = 0;

    loop {
        let tid: ThreadIdentifier = match create_test_thread(&mut pm) {
            Ok(tid) => tid,
            Err(error) if error.code == ErrorCode::OutOfMemory => break,
            Err(error) => {
                error!("thread creation failed with an unexpected error (error={error:?})");
                return false;
            },
        };
        if count == produced.len() {
            error!("thread creation was never backpressured by a stalled consumer");
            return false;
        }
        let status: ExitStatus = match u32::try_from(count + 1) {
            Ok(cycle) => ExitStatus::from(cycle),
            Err(_) => {
                error!("thread lifecycle cycle count does not fit an exit status");
                return false;
            },
        };
        if !exit_and_reap_thread(&mut pm, tid, status) {
            return false;
        }
        produced[count] = Some(ThreadTerminationInfo::new(pid, tid, status));
        count += 1;
    }

    // Only the permanent bootstrap kernel thread lacks a termination reservation.
    if count != ::config::kernel::MAX_THREADS - 1 {
        error!("backpressure did not engage at thread lifecycle capacity (cycles={count})");
        return false;
    }
    if pm.tm.try_next_tid().is_err() {
        error!("reaped threads did not leave execution capacity available");
        return false;
    }

    // Backpressure is deterministic: further attempts fail the same way and change nothing.
    let before = thread_creation_state(&pm);
    match create_test_thread(&mut pm) {
        Err(error) if error.code == ErrorCode::OutOfMemory => {},
        Err(error) => {
            error!("backpressured thread creation returned the wrong error (error={error:?})");
            return false;
        },
        Ok(tid) => {
            error!("thread creation exceeded lifecycle capacity (tid={tid:?})");
            return false;
        },
    }
    if thread_creation_state(&pm) != before {
        error!("backpressured thread creation changed process-manager state");
        return false;
    }

    // Delivering the oldest record releases exactly one reservation, so creation resumes.
    let Some(oldest) = produced[0] else {
        error!("oldest thread termination record was not tracked");
        return false;
    };
    if !receive_expected_lifecycle(
        &mut pm.delivery,
        MessageType::ThreadTerminationEvent,
        &oldest.to_ne_bytes(),
    ) {
        return false;
    }
    let resumed_tid: ThreadIdentifier = match create_test_thread(&mut pm) {
        Ok(tid) => tid,
        Err(error) => {
            error!("thread creation did not resume after a delivery (error={error:?})");
            return false;
        },
    };
    let resumed_status: ExitStatus = ExitStatus::from(0x0305_5001_u32);
    if !exit_and_reap_thread(&mut pm, resumed_tid, resumed_status) {
        return false;
    }
    produced[count] = Some(ThreadTerminationInfo::new(pid, resumed_tid, resumed_status));
    count += 1;

    for info in produced[1..count].iter().flatten() {
        if !receive_expected_lifecycle(
            &mut pm.delivery,
            MessageType::ThreadTerminationEvent,
            &info.to_ne_bytes(),
        ) {
            error!("thread termination record was lost or reordered (tid={:?})", info.tid);
            return false;
        }
    }
    if pm.delivery.has_lifecycle() || pm.delivery.capacity_in_use() != Some(0) {
        error!("draining the stalled consumer left lifecycle state behind");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies creation, exact thread termination, and process termination production order.
///
/// # Returns
///
/// `true` if lifecycle records carry exact metadata, preserve ordering, and release all capacity,
/// otherwise `false`.
///
fn test_zombie_transitions_produce_ordered_lifecycle_records() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let pid: ProcessIdentifier = test_pid();
    let parent: ProcessIdentifier = ProcessIdentifier::from(99);
    let tid: ThreadIdentifier = test_tid();
    let ready_tid: ThreadIdentifier = ThreadIdentifier::from(1001);
    let status: ExitStatus = ExitStatus::from(0x1020_3040_u32);
    let creation: ProcessCreationInfo = ProcessCreationInfo::new(pid, parent, ProcessRole::User);

    let process_reservation: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("failed to reserve process lifecycle capacity (error={error:?})");
            return false;
        },
    };
    let thread_reservation: ThreadLifecycleReservation = match broker.try_reserve_thread_creation()
    {
        Ok(reservation) => reservation,
        Err(error) => {
            broker.cancel_creation(process_reservation);
            error!("failed to reserve thread lifecycle capacity (error={error:?})");
            return false;
        },
    };
    let ready_thread_reservation: ThreadLifecycleReservation =
        match broker.try_reserve_thread_creation() {
            Ok(reservation) => reservation,
            Err(error) => {
                broker.cancel_thread_creation(thread_reservation);
                broker.cancel_creation(process_reservation);
                error!("failed to reserve ready-thread lifecycle capacity (error={error:?})");
                return false;
            },
        };
    let process_credit: LifecycleTerminationCredit =
        broker.commit_creation(process_reservation, creation);
    let thread_credit: ThreadLifecycleTerminationCredit =
        broker.commit_thread_creation(thread_reservation);
    let ready_thread_credit: ThreadLifecycleTerminationCredit =
        broker.commit_thread_creation(ready_thread_reservation);

    // SAFETY: process-manager tests run on one core with interrupts disabled and hold no other
    // reference to either global manager.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
    let vmem: Vmem = match mm.new_vmem(pm.current_vmem()) {
        Ok(vmem) => vmem,
        Err(error) => {
            error!("failed to create lifecycle transition vmem (error={error:?})");
            return false;
        },
    };
    let thread: ReadyThread = ReadyThread::new(
        tid,
        Some(thread_credit),
        None,
        None,
        None,
        ContextInformation::default(),
        // SAFETY: calls to FpuState::new are synchronized by the single-threaded test runner.
        unsafe { FpuState::new() },
    );
    let mut process: RunnableProcess = RunnableProcess::new_uncommitted(pid, parent, thread, vmem);
    process.install_termination_credit(process_credit);
    let (mut running, reason, _ctx, _user_tda) = process.run();
    if reason.is_some() {
        error!("new lifecycle transition fixture was unexpectedly interrupted");
        return false;
    }
    running.add_thread(ReadyThread::new(
        ready_tid,
        Some(ready_thread_credit),
        None,
        None,
        None,
        ContextInformation::default(),
        // SAFETY: calls to FpuState::new are synchronized by the single-threaded test runner.
        unsafe { FpuState::new() },
    ));

    let (transition, _ctx) = match running.exit(status, &mut |pending| {
        ProcessManager::commit_thread_termination(&mut broker, pending);
    }) {
        Err(transition) => transition,
        Ok(_) => {
            error!("process with a ready sibling did not transition to zombie state");
            return false;
        },
    };
    let (zombie, pending): (ZombieProcess, PendingProcessTermination) = transition.into_parts();
    let (actual_pid, actual_parent, actual_status, process_credit) = pending.into_parts();
    if actual_pid != pid || actual_parent != parent || actual_status != status {
        error!("zombie process transition changed authoritative termination metadata");
        return false;
    }
    let process_termination: ProcessTerminationInfo =
        ProcessTerminationInfo::new(pid, status, parent, ProcessRole::User);
    broker.commit_termination(process_credit, process_termination);

    let thread_termination: ThreadTerminationInfo = ThreadTerminationInfo::new(pid, tid, status);
    let ready_thread_termination: ThreadTerminationInfo =
        ThreadTerminationInfo::new(pid, ready_tid, ::sys::error::ErrorCode::Interrupted.into());
    if !receive_expected_lifecycle(
        &mut broker,
        MessageType::ProcessCreationEvent,
        &creation.to_ne_bytes(),
    ) || !receive_expected_lifecycle(
        &mut broker,
        MessageType::ThreadTerminationEvent,
        &thread_termination.to_ne_bytes(),
    ) || !receive_expected_lifecycle(
        &mut broker,
        MessageType::ThreadTerminationEvent,
        &ready_thread_termination.to_ne_bytes(),
    ) || !receive_expected_lifecycle(
        &mut broker,
        MessageType::ProcessTerminationEvent,
        &process_termination.to_ne_bytes(),
    ) {
        return false;
    }

    let (zombie_threads, _state, buried_status) = zombie.bury();
    let (mut remaining, first) = zombie_threads.pop_front();
    let _ = first.harvest();
    while let Some(thread) = remaining.pop_front() {
        let _ = thread.harvest();
    }
    if buried_status != status || broker.has_lifecycle() || broker.capacity_in_use() != Some(0) {
        error!("zombie reclamation produced a duplicate or retained lifecycle capacity");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies `execv()` retirement emits one zero-status thread record and keeps the process live.
///
/// # Returns
///
/// `true` if image replacement emits only the outgoing thread record and preserves the process,
/// otherwise `false`.
///
fn test_exec_replacement_produces_thread_termination_only() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let reservation: ThreadLifecycleReservation = match broker.try_reserve_thread_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("failed to reserve exec retirement capacity (error={error:?})");
            return false;
        },
    };
    let old_credit: ThreadLifecycleTerminationCredit = broker.commit_thread_creation(reservation);
    let pid: ProcessIdentifier = test_pid();
    let old_tid: ThreadIdentifier = test_tid();
    let new_tid: ThreadIdentifier = ThreadIdentifier::from(1001);

    // SAFETY: process-manager tests run on one core with interrupts disabled and hold no other
    // reference to either global manager.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
    let old_vmem: Vmem = match mm.new_vmem(pm.current_vmem()) {
        Ok(vmem) => vmem,
        Err(error) => {
            error!("failed to create outgoing exec vmem (error={error:?})");
            return false;
        },
    };
    let new_vmem: Vmem = match mm.new_vmem(pm.current_vmem()) {
        Ok(vmem) => vmem,
        Err(error) => {
            error!("failed to create replacement exec vmem (error={error:?})");
            return false;
        },
    };
    let old_thread: ReadyThread = ReadyThread::new(
        old_tid,
        Some(old_credit),
        None,
        None,
        None,
        ContextInformation::default(),
        // SAFETY: calls to FpuState::new are synchronized by the single-threaded test runner.
        unsafe { FpuState::new() },
    );
    let mut process: RunnableProcess =
        RunnableProcess::new_uncommitted(pid, ProcessIdentifier::from(99), old_thread, old_vmem);
    process.install_termination_credit(new_test_process_termination_credit());
    let (mut running, reason, _ctx, _user_tda) = process.run();
    if reason.is_some() {
        error!("outgoing exec fixture was unexpectedly interrupted");
        return false;
    }
    let new_thread: ReadyThread = ReadyThread::new(
        new_tid,
        Some(new_test_thread_termination_credit()),
        None,
        None,
        None,
        ContextInformation::default(),
        // SAFETY: calls to FpuState::new are synchronized by the single-threaded test runner.
        unsafe { FpuState::new() },
    );
    let (old_vmem, old_zombie, pending, _from_ctx, _to_ctx, _user_tda) =
        running.replace_image(new_vmem, new_thread);
    let (info, credit) = pending.into_parts();
    let expected: ThreadTerminationInfo =
        ThreadTerminationInfo::new(pid, old_tid, ExitStatus::ok());
    if info != expected || old_zombie.status() != ExitStatus::ok() || running.get_tid() != new_tid {
        error!("exec replacement changed retirement metadata or replacement identity");
        return false;
    }
    broker.commit_thread_termination(credit, info);
    if !receive_expected_lifecycle(
        &mut broker,
        MessageType::ThreadTerminationEvent,
        &expected.to_ne_bytes(),
    ) || broker.has_lifecycle()
        || broker.capacity_in_use() != Some(0)
    {
        error!("exec replacement produced extra lifecycle state");
        return false;
    }
    let _ = old_zombie.harvest();
    drop(old_vmem);
    true
}

///
/// # Description
///
/// Builds a single-threaded running process, lets `seed` prepare its outgoing image, replaces that
/// image as `execv()` does, and lets `check` inspect the resulting process.
///
/// # Parameters
///
/// - `seed`: Prepares the outgoing image (for example, its signal state) before the replacement.
/// - `check`: Inspects the process after the replacement.
///
/// # Returns
///
/// `true` if the fixture was built and `check` passes, otherwise `false`.
///
fn check_exec_replacement(
    seed: impl FnOnce(&mut RunningProcess),
    check: impl FnOnce(&mut RunningProcess) -> bool,
) -> bool {
    // SAFETY: process-manager tests run on one core with interrupts disabled and hold no other
    // reference to either global manager.
    let pm: &ProcessManager = unsafe { ProcessManager::get() };
    let mm: &VirtMemoryManager = unsafe { VirtMemoryManager::get() };
    let old_vmem: Vmem = match mm.new_vmem(pm.current_vmem()) {
        Ok(vmem) => vmem,
        Err(error) => {
            error!("failed to create outgoing exec vmem (error={error:?})");
            return false;
        },
    };
    let new_vmem: Vmem = match mm.new_vmem(pm.current_vmem()) {
        Ok(vmem) => vmem,
        Err(error) => {
            error!("failed to create replacement exec vmem (error={error:?})");
            return false;
        },
    };
    let old_thread: ReadyThread = ReadyThread::new(
        test_tid(),
        Some(new_test_thread_termination_credit()),
        None,
        None,
        None,
        ContextInformation::default(),
        // SAFETY: calls to FpuState::new are synchronized by the single-threaded test runner.
        unsafe { FpuState::new() },
    );
    let mut process: RunnableProcess = RunnableProcess::new_uncommitted(
        test_pid(),
        ProcessIdentifier::from(99),
        old_thread,
        old_vmem,
    );
    process.install_termination_credit(new_test_process_termination_credit());
    let (mut running, reason, _ctx, _user_tda) = process.run();
    if reason.is_some() {
        error!("outgoing exec fixture was unexpectedly interrupted");
        return false;
    }
    seed(&mut running);

    let new_thread: ReadyThread = ReadyThread::new(
        ThreadIdentifier::from(1001),
        Some(new_test_thread_termination_credit()),
        None,
        None,
        None,
        ContextInformation::default(),
        // SAFETY: calls to FpuState::new are synchronized by the single-threaded test runner.
        unsafe { FpuState::new() },
    );
    let (old_vmem, old_zombie, pending, _from_ctx, _to_ctx, _user_tda) =
        running.replace_image(new_vmem, new_thread);
    let passed: bool = check(&mut running);

    let _ = pending.into_parts();
    let _ = old_zombie.harvest();
    drop(old_vmem);
    passed
}

///
/// # Description
///
/// Verifies that `execv()` hands the blocked-signal mask of the calling thread to the initial
/// thread of the new image.
///
/// # Returns
///
/// `true` if the new image's initial thread inherits the outgoing thread's mask, otherwise `false`.
///
fn test_exec_replacement_inherits_signal_mask() -> bool {
    let mask: SigSet = signal_bit(SIGUSR1) | signal_bit(SIGTERM);
    check_exec_replacement(
        |running| running.running_mut().thread_state_mut().set_blocked(mask),
        |running| {
            let blocked: SigSet = running.running_mut().thread_state().blocked();
            if blocked != mask {
                error!(
                    "new image did not inherit the signal mask (blocked={blocked:#x}, \
                     expected={mask:#x})"
                );
                return false;
            }
            true
        },
    )
}

///
/// # Description
///
/// Returns a caught disposition whose handler lives in the outgoing image.
///
fn caught_disposition() -> SignalDisposition {
    SignalDisposition::Handler(Box::new(SignalHandler {
        entry: VirtualAddress::new(0x1000),
        mask: 0,
        flags: 0,
        sigaction: 0,
    }))
}

///
/// # Description
///
/// Verifies that `execv()` preserves the signals pending against the process and against the
/// calling thread, except those that the reset of caught dispositions leaves ignored by default.
///
/// # Returns
///
/// `true` if the new image observes the expected pending signals, otherwise `false`.
///
fn test_exec_replacement_preserves_pending_signals() -> bool {
    check_exec_replacement(
        |running| {
            let signals: &mut SignalControl = running.state_mut().signals_mut();
            for signum in [SIGUSR1, SIGCHLD, SIGWINCH] {
                signals.set_disposition(signum, caught_disposition());
            }
            signals.post(SIGUSR1);
            signals.post(SIGCHLD);
            for signum in [SIGUSR2, SIGWINCH] {
                running
                    .running_mut()
                    .thread_state_mut()
                    .post_pending(signum);
            }
        },
        |running| {
            let process_pending: SigSet = running.state().signals().pending();
            if process_pending != signal_bit(SIGUSR1) {
                error!(
                    "new image did not keep the pending signals of the process \
                     (pending={process_pending:#x})"
                );
                return false;
            }
            let thread_pending: SigSet = running.running_mut().thread_state().pending();
            if thread_pending != signal_bit(SIGUSR2) {
                error!(
                    "new image did not keep the pending signals of the calling thread \
                     (pending={thread_pending:#x})"
                );
                return false;
            }
            true
        },
    )
}

///
/// # Description
///
/// Verifies that older IPC is selected before a newer lifecycle record.
///
/// # Returns
///
/// `true` if delivery follows production-sequence order, otherwise `false`.
///
fn test_ipc_precedes_newer_lifecycle() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let mut mailbox: Mailbox = Mailbox::default();
    let receiver: MessageReceiver = MessageReceiver::new(test_pid(), ThreadIdentifier::NONE);
    let ipc_sequence = broker.allocate_sequence();
    mailbox.send(ipc_sequence, ipc_message(receiver));
    let _termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };

    let mailbox_sequence: Option<DeliverySequence> =
        mailbox.peek(test_tid()).map(|(sequence, _)| sequence);
    if broker.lifecycle_precedes(mailbox_sequence, true) {
        error!("newer lifecycle record was selected ahead of older IPC");
        return false;
    }
    let ipc_sequence: DeliverySequence = match mailbox.peek(test_tid()) {
        Some((sequence, _)) => sequence,
        None => {
            error!("older IPC message was not eligible for delivery");
            return false;
        },
    };
    if !mailbox.commit(test_tid(), ipc_sequence) {
        error!("older IPC message could not be committed");
        return false;
    }
    if !broker.lifecycle_precedes(None, true) {
        error!("lifecycle record was not selected after older IPC was consumed");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that older lifecycle is selected before newer IPC even after a failed wakeup attempt.
///
/// # Returns
///
/// `true` if the failed wakeup leaves ordering and both records intact, otherwise `false`.
///
fn test_lifecycle_precedes_newer_ipc_after_failed_wakeup() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let mut mailbox: Mailbox = Mailbox::default();
    let _termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };

    // Model a wakeup attempt that failed after taking the request. The lifecycle record remains.
    if !broker.take_lifecycle_wakeup_request() {
        error!("queued lifecycle record did not request owner wakeup");
        return false;
    }
    let ipc_sequence = broker.allocate_sequence();
    let receiver: MessageReceiver = MessageReceiver::new(test_pid(), ThreadIdentifier::NONE);
    mailbox.send(ipc_sequence, ipc_message(receiver));

    let mailbox_sequence: Option<DeliverySequence> =
        mailbox.peek(test_tid()).map(|(sequence, _)| sequence);
    if !broker.lifecycle_precedes(mailbox_sequence, true) {
        error!("newer IPC overtook lifecycle after failed wakeup");
        return false;
    }
    if receive_lifecycle(&mut broker).map(|message| message.message_type)
        != Some(MessageType::ProcessCreationEvent)
    {
        error!("expected the older process-creation record");
        return false;
    }
    let ipc_sequence: DeliverySequence = match mailbox.peek(test_tid()) {
        Some((sequence, _)) => sequence,
        None => {
            error!("newer IPC message was lost after lifecycle delivery");
            return false;
        },
    };
    if !mailbox.commit(test_tid(), ipc_sequence) {
        error!("newer IPC message could not be committed");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that IPC for another thread does not block an eligible lifecycle record.
///
/// # Returns
///
/// `true` if ineligible IPC is excluded from lifecycle arbitration, otherwise `false`.
///
fn test_other_thread_ipc_does_not_block_lifecycle() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let mut mailbox: Mailbox = Mailbox::default();
    let other_tid: ThreadIdentifier = ThreadIdentifier::from(2000);
    let other_receiver: MessageReceiver = MessageReceiver::new(test_pid(), other_tid);
    let ipc_sequence = broker.allocate_sequence();
    mailbox.send(ipc_sequence, ipc_message(other_receiver));
    let _termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };

    let mailbox_sequence: Option<DeliverySequence> =
        mailbox.peek(test_tid()).map(|(sequence, _)| sequence);
    if mailbox_sequence.is_some() {
        error!("message for another thread was considered eligible");
        return false;
    }
    if !broker.lifecycle_precedes(mailbox_sequence, true) {
        error!("ineligible older IPC blocked lifecycle delivery");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies lifecycle FIFO ordering and lifecycle-owner eligibility.
///
/// # Returns
///
/// `true` if only an eligible owner receives lifecycle records in FIFO order, otherwise `false`.
///
fn test_lifecycle_fifo_and_eligibility() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };
    broker.commit_termination(termination_credit, termination_info());

    if broker.lifecycle_precedes(None, false) {
        error!("lifecycle record was selected for a process that does not own the class");
        return false;
    }
    let first: Option<MessageType> =
        receive_lifecycle(&mut broker).map(|message| message.message_type);
    let second: Option<MessageType> =
        receive_lifecycle(&mut broker).map(|message| message.message_type);
    if first != Some(MessageType::ProcessCreationEvent)
        || second != Some(MessageType::ProcessTerminationEvent)
    {
        error!("lifecycle records were not delivered in FIFO order");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that lifecycle selection is stable until commit and committed tokens become stale.
///
/// # Returns
///
/// `true` if retries preserve the selected sequence and commits invalidate it exactly once,
/// otherwise `false`.
///
fn test_lifecycle_delivery_is_transactional() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };
    broker.commit_termination(termination_credit, termination_info());

    let (first_sequence, first_message): (DeliverySequence, Message) =
        match broker.peek_lifecycle(test_pid()) {
            Some(delivery) => delivery,
            None => {
                error!("lifecycle selection returned no record");
                return false;
            },
        };
    let retry_sequence: Option<DeliverySequence> = broker
        .peek_lifecycle(test_pid())
        .map(|(sequence, _)| sequence);
    if first_message.message_type != MessageType::ProcessCreationEvent
        || retry_sequence != Some(first_sequence)
        || !broker.test_lifecycle_token_is_current(first_sequence)
    {
        error!("lifecycle retry did not preserve the selected sequence");
        return false;
    }

    broker.commit_lifecycle(first_sequence);
    if broker.test_lifecycle_token_is_current(first_sequence) {
        error!("committed lifecycle token did not become stale");
        return false;
    }
    let (second_sequence, second_message): (DeliverySequence, Message) =
        match broker.peek_lifecycle(test_pid()) {
            Some(delivery) => delivery,
            None => {
                error!("lifecycle commit removed the following record");
                return false;
            },
        };
    if second_sequence == first_sequence
        || second_message.message_type != MessageType::ProcessTerminationEvent
    {
        error!("lifecycle commit exposed the wrong following record");
        return false;
    }
    broker.commit_lifecycle(second_sequence);

    true
}

/// Verifies that pre-registration buffering and failed wakeups can re-arm owner notification.
fn test_lifecycle_wakeup_request_can_be_rearmed() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let _termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };

    if !broker.take_lifecycle_wakeup_request() || !broker.has_lifecycle() {
        error!("taking a wakeup request removed its lifecycle record");
        return false;
    }
    if broker.take_lifecycle_wakeup_request() {
        error!("wakeup request was reported twice without being re-armed");
        return false;
    }

    // Registration or a failed wakeup retries notification for the still-buffered record.
    broker.request_lifecycle_wakeup();
    if !broker.take_lifecycle_wakeup_request() {
        error!("buffered lifecycle record did not re-arm owner wakeup");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that signal delivery preserves the complete `EDX:EAX` kernel-call return value.
///
/// This catches sign-extension regressions for negative errno-style returns and high-half loss for
/// 64-bit success values restored through `sigreturn()`.
///
#[cfg(target_arch = "x86")]
fn test_signal_kcall_result_split_join_preserves_bits() -> bool {
    for value in [0i64, 1, -1, -4, 0x1234_5678_9abc_def0u64 as i64] {
        let (ax, dx): (u32, u32) = split_kcall_result(value);
        let restored: i64 = join_kcall_result(ax, dx);
        if restored != value {
            error!(
                "split/join changed kcall result bits (value={value}, ax={ax:#x}, dx={dx:#x}, \
                 restored={restored})"
            );
            return false;
        }
    }

    true
}

//==================================================================================================
// Capability Control Tests
//==================================================================================================

/// Every capability that a process may attempt to acquire.
const CAPABILITIES: [Capability; 5] = [
    Capability::ExceptionControl,
    Capability::InterruptControl,
    Capability::IoManagement,
    Capability::MemoryManagement,
    Capability::ProcessManagement,
];

/// Raw identifier of a user process forked from the init process.
const FORKED_PID_RAW: i32 = ProcessIdentifier::INIT_RAW + 1;

///
/// # Description
///
/// Creates a local process manager whose running process has a given identity and verifies that
/// the kernel classifies that process with the expected role.
///
/// # Parameters
///
/// - `pid`: Identifier of the running process.
/// - `parent`: Identifier of the parent of the running process.
/// - `daemon_name`: Name under which the running process is registered as a system daemon, if
///   any.
/// - `expected`: Role that the running process is expected to have.
///
/// # Returns
///
/// The local process-manager fixture, or [`None`] if it could not be prepared or the running
/// process does not have the expected role.
///
fn new_capctl_fixture(
    pid: ProcessIdentifier,
    parent: ProcessIdentifier,
    daemon_name: Option<&'static str>,
    expected: ProcessRole,
) -> Option<ProcessManager> {
    let mut pm: ProcessManager = new_process_manager_running(pid, parent)?;
    if let Some(daemon_name) = daemon_name {
        pm.daemon_pids.push((daemon_name, pid));
    }
    let role: ProcessRole = pm.classify_role(pid, parent);
    if role != expected {
        error!("capctl fixture has an unexpected role (pid={pid:?}, role={role:?})");
        return None;
    }
    Some(pm)
}

///
/// # Description
///
/// Acquires and releases every capability on behalf of a process, checking the capability state of
/// the process after each step.
///
/// # Parameters
///
/// - `pm`: Process manager that hosts the process.
/// - `pid`: Identifier of the process.
///
/// # Returns
///
/// `true` if every capability was acquired and released, otherwise `false`.
///
fn acquire_and_release_all(pm: &mut ProcessManager, pid: ProcessIdentifier) -> bool {
    for capability in CAPABILITIES {
        if let Err(error) = pm.capctl(pid, capability, true) {
            error!("failed to acquire capability (capability={capability:?}, error={error:?})");
            return false;
        }
        if !matches!(pm.has_capability(pid, capability), Ok(true)) {
            error!("acquired capability is not set (capability={capability:?})");
            return false;
        }
        if let Err(error) = pm.capctl(pid, capability, false) {
            error!("failed to release capability (capability={capability:?}, error={error:?})");
            return false;
        }
        if !matches!(pm.has_capability(pid, capability), Ok(false)) {
            error!("released capability is still set (capability={capability:?})");
            return false;
        }
    }
    true
}

///
/// # Description
///
/// Verifies that the init process, which the kernel spawns directly, may acquire and release every
/// capability.
///
/// # Returns
///
/// `true` if the init process acquired and released every capability, otherwise `false`.
///
fn test_capctl_allows_acquire_from_init_process() -> bool {
    let pid: ProcessIdentifier = ProcessIdentifier::INIT;
    let Some(mut pm) = new_capctl_fixture(pid, ProcessIdentifier::KERNEL, None, ProcessRole::Init)
    else {
        return false;
    };
    acquire_and_release_all(&mut pm, pid)
}

///
/// # Description
///
/// Verifies that a system daemon, which the kernel spawns directly and registers by name, may
/// acquire and release every capability.
///
/// # Returns
///
/// `true` if the daemon acquired and released every capability, otherwise `false`.
///
fn test_capctl_allows_acquire_from_daemon_process() -> bool {
    let pid: ProcessIdentifier = ProcessIdentifier::PROCD;
    let Some(mut pm) = new_capctl_fixture(
        pid,
        ProcessIdentifier::KERNEL,
        Some(::config::daemons::PROCD_NAME),
        ProcessRole::Daemon,
    ) else {
        return false;
    };
    acquire_and_release_all(&mut pm, pid)
}

///
/// # Description
///
/// Verifies that a process forked from a user process cannot acquire any capability, and that a
/// denied acquisition leaves the capability unset.
///
/// # Returns
///
/// `true` if every acquisition is denied with [`ErrorCode::PermissionDenied`], otherwise `false`.
///
fn test_capctl_denies_acquire_from_user_process() -> bool {
    let pid: ProcessIdentifier = ProcessIdentifier::from(FORKED_PID_RAW);
    let Some(mut pm) = new_capctl_fixture(pid, ProcessIdentifier::INIT, None, ProcessRole::User)
    else {
        return false;
    };
    for capability in CAPABILITIES {
        match pm.capctl(pid, capability, true) {
            Err(error) if error.code == ErrorCode::PermissionDenied => {},
            result => {
                error!(
                    "user process acquisition was not denied (capability={capability:?}, \
                     result={result:?})"
                );
                return false;
            },
        }
        if !matches!(pm.has_capability(pid, capability), Ok(false)) {
            error!("denied acquisition changed capability state (capability={capability:?})");
            return false;
        }
    }
    true
}

///
/// # Description
///
/// Verifies that a process forked from a user process may still relinquish capabilities, since
/// doing so can only reduce its privileges. Releasing a capability that the process does not hold
/// fails with [`ErrorCode::NoSuchEntry`] rather than [`ErrorCode::PermissionDenied`], and releasing
/// one that it holds succeeds without authorizing the process to acquire it back.
///
/// # Returns
///
/// `true` if every release follows this policy, otherwise `false`.
///
fn test_capctl_allows_release_from_user_process() -> bool {
    let pid: ProcessIdentifier = ProcessIdentifier::from(FORKED_PID_RAW);
    let Some(mut pm) = new_capctl_fixture(pid, ProcessIdentifier::INIT, None, ProcessRole::User)
    else {
        return false;
    };
    for capability in CAPABILITIES {
        match pm.capctl(pid, capability, false) {
            Err(error) if error.code == ErrorCode::NoSuchEntry => {},
            result => {
                error!(
                    "release of an unset capability did not report a missing entry \
                     (capability={capability:?}, result={result:?})"
                );
                return false;
            },
        }

        // Install the capability directly, because the process cannot acquire it by itself.
        pm.get_running_mut().state_mut().set_capability(capability);
        if let Err(error) = pm.capctl(pid, capability, false) {
            error!(
                "failed to release held capability (capability={capability:?}, error={error:?})"
            );
            return false;
        }
        if !matches!(pm.has_capability(pid, capability), Ok(false)) {
            error!("released capability is still set (capability={capability:?})");
            return false;
        }
        match pm.capctl(pid, capability, true) {
            Err(error) if error.code == ErrorCode::PermissionDenied => {},
            result => {
                error!(
                    "user process acquired a released capability (capability={capability:?}, \
                     result={result:?})"
                );
                return false;
            },
        }
    }
    true
}

//==================================================================================================
// Test Runner
//==================================================================================================

///
/// # Description
///
/// Runs all in-kernel unit tests for the process manager module.
///
/// # Returns
///
/// `true` if every process-manager test passes, otherwise `false`.
///
pub(in crate::pm::process) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_intra_process_switch_resets_exhausted_quantum);
    passed &= run_test!(test_intra_process_switch_preserves_remaining_quantum);
    passed &= run_test!(test_cross_process_switch_resets_quantum);
    passed &= run_test!(test_create_process_reservation_failure_rolls_back);
    passed &= run_test!(test_duplicate_process_reservation_failure_rolls_back);
    passed &= run_test!(test_create_thread_reservation_failure_rolls_back);
    passed &= run_test!(test_execv_thread_reservation_failure_rolls_back);
    passed &= run_test!(test_reap_releases_execution_capacity_before_delivery);
    passed &= run_test!(test_stalled_consumer_backpressures_thread_creation_without_loss);
    passed &= run_test!(test_zombie_transitions_produce_ordered_lifecycle_records);
    passed &= run_test!(test_exec_replacement_produces_thread_termination_only);
    passed &= run_test!(test_exec_replacement_inherits_signal_mask);
    passed &= run_test!(test_exec_replacement_preserves_pending_signals);
    passed &= run_test!(test_ipc_precedes_newer_lifecycle);
    passed &= run_test!(test_lifecycle_precedes_newer_ipc_after_failed_wakeup);
    passed &= run_test!(test_other_thread_ipc_does_not_block_lifecycle);
    passed &= run_test!(test_lifecycle_fifo_and_eligibility);
    passed &= run_test!(test_lifecycle_delivery_is_transactional);
    passed &= run_test!(test_lifecycle_wakeup_request_can_be_rearmed);
    passed &= run_test!(test_capctl_allows_acquire_from_init_process);
    passed &= run_test!(test_capctl_allows_acquire_from_daemon_process);
    passed &= run_test!(test_capctl_denies_acquire_from_user_process);
    passed &= run_test!(test_capctl_allows_release_from_user_process);
    passed &= super::delivery::test();
    #[cfg(target_arch = "x86")]
    {
        passed &= run_test!(test_signal_kcall_result_split_join_preserves_bits);
    }
    passed
}
