// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::delivery::DeliveryBroker;
#[cfg(target_arch = "x86")]
use crate::hal::arch::{
    join_kcall_result,
    split_kcall_result,
};
use crate::{
    ipc::Mailbox,
    pm::{
        process::{
            LifecycleCreationReservation,
            LifecycleTerminationCredit,
        },
        ProcessManager,
    },
};
use ::config::kernel::SCHEDULER_FREQ;
use ::sys::{
    event::{
        ProcessCreationInfo,
        ProcessRole,
        ProcessTerminationInfo,
    },
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    ExitStatus,
};

//==================================================================================================
// Test Helpers
//==================================================================================================

fn test_pid() -> ProcessIdentifier {
    ProcessIdentifier::from(1000)
}

fn test_tid() -> ThreadIdentifier {
    ThreadIdentifier::from(1000)
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

/// Verifies that older IPC is selected before a newer lifecycle record.
fn test_ipc_precedes_newer_lifecycle() -> bool {
    let mut broker: DeliveryBroker = DeliveryBroker::default();
    let mut mailbox: Mailbox = Mailbox::default();
    let receiver: MessageReceiver = MessageReceiver::new(test_pid(), ThreadIdentifier::NONE);
    let ipc_sequence = broker.allocate_sequence();
    mailbox.send(ipc_sequence, ipc_message(receiver));
    let _termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };

    if broker.lifecycle_precedes(mailbox.peek_sequence(test_tid()), true) {
        error!("newer lifecycle record was selected ahead of older IPC");
        return false;
    }
    if mailbox.receive(test_tid()).is_none() {
        error!("older IPC message was not eligible for delivery");
        return false;
    }
    if !broker.lifecycle_precedes(None, true) {
        error!("lifecycle record was not selected after older IPC was consumed");
        return false;
    }

    true
}

/// Verifies that older lifecycle is selected before newer IPC even after a failed wakeup attempt.
fn test_lifecycle_precedes_newer_ipc_after_failed_wakeup() -> bool {
    let mut broker: DeliveryBroker = DeliveryBroker::default();
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

    if !broker.lifecycle_precedes(mailbox.peek_sequence(test_tid()), true) {
        error!("newer IPC overtook lifecycle after failed wakeup");
        return false;
    }
    if broker
        .pop_lifecycle(test_pid())
        .map(|message| message.message_type)
        != Some(MessageType::ProcessCreationEvent)
    {
        error!("expected the older process-creation record");
        return false;
    }
    if mailbox.receive(test_tid()).is_none() {
        error!("newer IPC message was lost after lifecycle delivery");
        return false;
    }

    true
}

/// Verifies that IPC for another thread does not block an eligible lifecycle record.
fn test_other_thread_ipc_does_not_block_lifecycle() -> bool {
    let mut broker: DeliveryBroker = DeliveryBroker::default();
    let mut mailbox: Mailbox = Mailbox::default();
    let other_tid: ThreadIdentifier = ThreadIdentifier::from(2000);
    let other_receiver: MessageReceiver = MessageReceiver::new(test_pid(), other_tid);
    let ipc_sequence = broker.allocate_sequence();
    mailbox.send(ipc_sequence, ipc_message(other_receiver));
    let _termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };

    if mailbox.peek_sequence(test_tid()).is_some() {
        error!("message for another thread was considered eligible");
        return false;
    }
    if !broker.lifecycle_precedes(mailbox.peek_sequence(test_tid()), true) {
        error!("ineligible older IPC blocked lifecycle delivery");
        return false;
    }

    true
}

/// Verifies lifecycle FIFO ordering and lifecycle-owner eligibility.
fn test_lifecycle_fifo_and_eligibility() -> bool {
    let mut broker: DeliveryBroker = DeliveryBroker::default();
    let termination_credit: LifecycleTerminationCredit = match commit_creation(&mut broker) {
        Some(credit) => credit,
        None => return false,
    };
    broker.commit_termination(termination_credit, termination_info());

    if broker.lifecycle_precedes(None, false) {
        error!("lifecycle record was selected for a process that does not own the class");
        return false;
    }
    let first: Option<MessageType> = broker
        .pop_lifecycle(test_pid())
        .map(|message| message.message_type);
    let second: Option<MessageType> = broker
        .pop_lifecycle(test_pid())
        .map(|message| message.message_type);
    if first != Some(MessageType::ProcessCreationEvent)
        || second != Some(MessageType::ProcessTerminationEvent)
    {
        error!("lifecycle records were not delivered in FIFO order");
        return false;
    }

    true
}

/// Verifies that pre-registration buffering and failed wakeups can re-arm owner notification.
fn test_lifecycle_wakeup_request_can_be_rearmed() -> bool {
    let mut broker: DeliveryBroker = DeliveryBroker::default();
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
// Test Runner
//==================================================================================================

/// Runs all in-kernel unit tests for the process manager module.
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_intra_process_switch_resets_exhausted_quantum);
    passed &= run_test!(test_intra_process_switch_preserves_remaining_quantum);
    passed &= run_test!(test_cross_process_switch_resets_quantum);
    passed &= run_test!(test_ipc_precedes_newer_lifecycle);
    passed &= run_test!(test_lifecycle_precedes_newer_ipc_after_failed_wakeup);
    passed &= run_test!(test_other_thread_ipc_does_not_block_lifecycle);
    passed &= run_test!(test_lifecycle_fifo_and_eligibility);
    passed &= run_test!(test_lifecycle_wakeup_request_can_be_rearmed);
    passed &= super::delivery::test();
    #[cfg(target_arch = "x86")]
    {
        passed &= run_test!(test_signal_kcall_result_split_join_preserves_bits);
    }
    passed
}
