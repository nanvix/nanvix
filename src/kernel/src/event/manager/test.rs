// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    DeliveryEpoch,
    EventManager,
    EventManagerInner,
    EventMasks,
    EventSequence,
    ExceptionEventInformation,
    PendingDelivery,
    WaitingThreadGuard,
    MANAGER,
};
use crate::{
    hal::arch::ExceptionInformation,
    ipc::{
        recv_with,
        Mailbox,
    },
    kcall::{
        drain_lifecycle_wakeup,
        KcallResult,
    },
    pm::{
        new_test_delivery_sequence,
        new_test_message,
        sync::condvar::Condvar,
        ProcessManager,
        SleepError,
        UncommittedMessage,
        UncommittedMessageToken,
    },
};
use ::alloc::collections::{
    LinkedList,
    VecDeque,
};
use ::core::cell::RefCell;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    event::{
        Event,
        EventCtrlRequest,
        EventDescriptor,
        EventInformation,
        ExceptionEvent,
        InterruptEvent,
        ProcessCreationInfo,
        ProcessRole,
        SchedulingEvent,
    },
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::{
        Capability,
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Test Helpers
//==================================================================================================

///
/// # Description
///
/// Returns a synthetic process identifier used as the owner and recipient of test events. The value
/// is arbitrary: every test operates on a function-local event manager, so the identifier is only
/// stamped into delivered messages and never resolved against live kernel state.
///
fn test_pid() -> ProcessIdentifier {
    ProcessIdentifier::from(1000)
}

///
/// # Description
///
/// Returns a synthetic thread identifier used as the waiter in test deliveries. As with
/// [`test_pid`], the value is arbitrary because tests never reach the path that resolves it.
///
fn test_tid() -> ThreadIdentifier {
    ThreadIdentifier::from(1000)
}

///
/// # Description
///
/// Builds a fresh, empty [`EventManagerInner`] for exclusive use by a single test. The instance is
/// independent of the global event manager, so tests neither observe nor perturb live kernel state.
///
/// # Returns
///
/// A fresh event-manager state with empty queues and no registered owners.
///
fn make_inner() -> EventManagerInner {
    EventManagerInner {
        interrupt_capable: true,
        event_sequence: EventSequence::default(),
        delivery_epoch: DeliveryEpoch::default(),
        wait: Some(Condvar::new()),
        waiting_threads: VecDeque::new(),
        interrupt_ownership: (0..usize::BITS).map(|_| None).collect(),
        pending_interrupts: (0..usize::BITS).map(|_| LinkedList::default()).collect(),
        exception_ownership: (0..usize::BITS).map(|_| None).collect(),
        pending_exceptions: (0..usize::BITS).map(|_| LinkedList::default()).collect(),
        scheduling_owner: None,
    }
}

///
/// # Description
///
/// Enqueues a pending interrupt event on `bit`, stamping it with the next event sequence number.
///
/// # Parameters
///
/// - `inner`: Event-manager state that receives the pending interrupt.
/// - `bit`: Interrupt queue index to populate.
///
fn push_interrupt(inner: &mut EventManagerInner, bit: usize) {
    let sequence: EventSequence = inner.next_event_sequence();
    let event: Event = Event::Interrupt(InterruptEvent::VALUES[bit]);
    let descriptor: EventDescriptor = EventDescriptor::new(sequence.descriptor_id(), event);
    inner.pending_interrupts[bit].push_back((sequence, descriptor));
}

///
/// # Description
///
/// Enqueues a pending exception event on `bit`, owned by `pid`, stamping it with the next event
/// sequence number. The event manager re-queues exceptions on each delivery until the owner resumes
/// from them, so a test that does not want continuous re-delivery must clear the slot once observed
/// (see [`resume_exception`]).
///
/// # Parameters
///
/// - `inner`: Event-manager state that receives the pending exception.
/// - `bit`: Exception queue index to populate.
/// - `pid`: Identifier of the process that owns the exception.
/// - `tid`: Identifier of the faulting thread.
///
fn push_exception(
    inner: &mut EventManagerInner,
    bit: usize,
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
) {
    let sequence: EventSequence = inner.next_event_sequence();
    inner.exception_ownership[bit] = Some(pid);
    let event: Event = Event::Exception(ExceptionEvent::VALUES[bit]);
    let descriptor: EventDescriptor = EventDescriptor::new(sequence.descriptor_id(), event);
    // SAFETY: `ExceptionInformation` is plain-old-data: a register snapshot of fixed-width integer
    // fields (`u32` on 32-bit x86, `u64` on x86_64) for which the all-zero bit pattern is a valid
    // value on every supported architecture. The tests only need a placeholder to exercise delivery.
    let info: ExceptionInformation = unsafe { core::mem::zeroed() };
    inner.pending_exceptions[bit].push_back((
        sequence,
        descriptor,
        ExceptionEventInformation { pid, tid, info },
        Condvar::new(),
    ));
}

///
/// # Description
///
/// Removes the pending exception on `bit`, mimicking the owner resuming from it. This stops the
/// event manager from re-delivering the same exception on subsequent calls.
///
fn resume_exception(inner: &mut EventManagerInner, bit: usize) {
    inner.pending_exceptions[bit].clear();
    inner.exception_ownership[bit] = None;
}

/// Returns a synthetic IPC message addressed to the test process.
fn ipc_message() -> Message {
    Message {
        source: MessageSender::KERNEL,
        destination: MessageReceiver::new(test_pid(), ThreadIdentifier::NONE),
        message_type: MessageType::Ipc,
        ..Message::default()
    }
}

/// Returns a synthetic lifecycle message addressed to the test process.
fn lifecycle_message() -> Message {
    Message {
        source: MessageSender::KERNEL,
        destination: MessageReceiver::new(test_pid(), ThreadIdentifier::NONE),
        message_type: MessageType::ProcessCreationEvent,
        ..Message::default()
    }
}

///
/// # Description
///
/// Wraps a synthetic message in an uncommitted lifecycle delivery for selection tests.
///
/// # Parameters
///
/// - `message`: Synthetic message to wrap.
///
/// # Returns
///
/// An uncommitted message carrying a synthetic lifecycle token.
///
fn synthetic_message_delivery(message: Message) -> UncommittedMessage {
    new_test_message(message, UncommittedMessageToken::Lifecycle(new_test_delivery_sequence(0)))
}

///
/// # Description
///
/// Peeks a local mailbox and wraps the selected entry in its uncommitted delivery token.
///
/// # Parameters
///
/// - `mailbox`: Mailbox from which to select a message.
/// - `tid`: Identifier of the receiving thread.
///
/// # Returns
///
/// The selected uncommitted message, or [`None`] if no message is eligible for `tid`.
///
fn peek_mailbox_delivery(mailbox: &Mailbox, tid: ThreadIdentifier) -> Option<UncommittedMessage> {
    mailbox.peek(tid).map(|(sequence, message)| {
        new_test_message(message, UncommittedMessageToken::Mailbox { tid, sequence })
    })
}

///
/// # Description
///
/// Commits a mailbox delivery token against a local test mailbox.
///
/// # Parameters
///
/// - `mailbox`: Mailbox that owns the selected entry.
/// - `token`: Token identifying the selected mailbox entry.
///
/// # Panics
///
/// This function panics if `token` is not a mailbox token or no longer identifies the selected
/// entry.
///
fn commit_mailbox_delivery(mailbox: &mut Mailbox, token: UncommittedMessageToken) {
    match token {
        UncommittedMessageToken::Mailbox { tid, sequence } => {
            assert!(mailbox.commit(tid, sequence), "mailbox delivery token became invalid");
        },
        UncommittedMessageToken::Lifecycle(_) => {
            unreachable!("expected a mailbox delivery token");
        },
    }
}

///
/// # Description
///
/// Commits a local selection and advances its test cursor exactly as production does.
///
/// # Parameters
///
/// - `inner`: Event-manager state that owns the pending delivery.
/// - `cursor`: Receiver cursor to advance after commit.
/// - `delivery`: Pending delivery to commit.
/// - `commit_message`: Operation that commits mailbox or lifecycle tokens.
///
/// # Returns
///
/// The message carried by `delivery`.
///
/// # Panics
///
/// This function panics if the pending delivery token is stale or invalid.
///
fn commit_delivery<F>(
    inner: &mut EventManagerInner,
    cursor: &mut usize,
    delivery: PendingDelivery,
    commit_message: F,
) -> Message
where
    F: FnOnce(UncommittedMessageToken),
{
    let next_cursor: usize = delivery.token.next_cursor();
    let message: Message = delivery.message;
    inner.commit_with(delivery.token, commit_message);
    *cursor = next_cursor;
    message
}

/// Queues a synthetic lifecycle record through the global process manager.
fn queue_global_lifecycle(pid: ProcessIdentifier) -> bool {
    let info: ProcessCreationInfo =
        ProcessCreationInfo::new(pid, ProcessIdentifier::KERNEL, ProcessRole::User);
    // SAFETY: this late integration test runs after process-manager initialization with interrupts
    // disabled and does not retain a process-manager reference across another global access.
    match unsafe { ProcessManager::get_mut() }.queue_test_process_creation(info) {
        Ok(()) => true,
        Err(error) => {
            error!("failed to queue test lifecycle record (error={error:?})");
            false
        },
    }
}

///
/// # Description
///
/// Posts a process-directed IPC message with a zero status through the global process manager.
///
/// # Returns
///
/// `true` if the message was posted successfully, otherwise `false`.
///
fn post_global_ipc() -> bool {
    post_global_ipc_with_status(0)
}

///
/// # Description
///
/// Posts a process-directed IPC message carrying a status through the global process manager.
///
/// # Parameters
///
/// - `status`: Status value to store in the test message.
///
/// # Returns
///
/// `true` if the message was posted successfully, otherwise `false`.
///
fn post_global_ipc_with_status(status: i32) -> bool {
    let message: Message = Message {
        source: MessageSender::KERNEL,
        destination: MessageReceiver::KERNEL,
        message_type: MessageType::Ipc,
        status,
        ..Message::default()
    };
    // SAFETY: this late integration test runs after process-manager initialization with interrupts
    // disabled and does not retain a process-manager reference across another global access.
    match unsafe { ProcessManager::get_mut() }.post_message(MessageReceiver::KERNEL, message) {
        Ok(()) => true,
        Err(error) => {
            error!("failed to post test IPC message (error={error:?})");
            false
        },
    }
}

///
/// # Description
///
/// Reads the global number of buffered mailbox messages from the process manager.
///
/// # Returns
///
/// The current number of globally buffered mailbox messages.
///
fn global_buffered_messages() -> usize {
    // SAFETY: this late integration test runs with interrupts disabled and retains no reference.
    unsafe { ProcessManager::get_mut() }.number_buffered_messages()
}

///
/// # Description
///
/// Verifies that failing the receive copy boundary leaves every delivery class pending and does
/// not advance accounting or the service cursor.
///
/// # Returns
///
/// `true` if IPC, lifecycle, interrupt, and exception delivery remain transactional, otherwise
/// `false`.
///
fn test_receive_copy_failure_is_transactional() -> bool {
    const FIRST_IPC_STATUS: i32 = 3050;
    const SECOND_IPC_STATUS: i32 = 3051;
    const INTERRUPT_BIT: usize = 1;
    const EXCEPTION_BIT: usize = 2;

    let buffered_before: usize = global_buffered_messages();
    if !post_global_ipc_with_status(FIRST_IPC_STATUS)
        || !post_global_ipc_with_status(SECOND_IPC_STATUS)
    {
        return false;
    }
    if global_buffered_messages() != buffered_before + 2 {
        error!("posting transactional IPC fixtures changed accounting unexpectedly");
        return false;
    }
    // SAFETY: the integration test owns the running kernel process state.
    unsafe { ProcessManager::set_delivery_cursor(2) };
    let mut failed_ipc_status: Option<i32> = None;
    // SAFETY: an IPC message is pending, so the kernel test process cannot block.
    let failed_ipc: Result<(), SleepError> = unsafe {
        recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |message| {
            failed_ipc_status = Some(message.status);
            Err(Error::new(ErrorCode::TryAgain, "injected IPC copy failure"))
        })
    };
    if !matches!(failed_ipc, Err(SleepError::Generic(_)))
        || failed_ipc_status != Some(FIRST_IPC_STATUS)
        || global_buffered_messages() != buffered_before + 2
        || unsafe { ProcessManager::delivery_cursor() } != 2
    {
        error!("failed IPC copy changed delivery state");
        return false;
    }

    let mut retried_ipc_status: Option<i32> = None;
    // SAFETY: the IPC message retained by the failed copy is still pending.
    if unsafe {
        recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |message| {
            retried_ipc_status = Some(message.status);
            Ok(())
        })
    }
    .is_err()
        || retried_ipc_status != Some(FIRST_IPC_STATUS)
        || global_buffered_messages() != buffered_before + 1
        || unsafe { ProcessManager::delivery_cursor() } != 0
    {
        error!("IPC retry did not commit the originally selected message exactly once");
        return false;
    }
    let mut second_ipc_status: Option<i32> = None;
    // SAFETY: the second IPC fixture is pending.
    if unsafe {
        recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |message| {
            second_ipc_status = Some(message.status);
            Ok(())
        })
    }
    .is_err()
        || second_ipc_status != Some(SECOND_IPC_STATUS)
        || global_buffered_messages() != buffered_before
    {
        error!("IPC commit did not preserve FIFO order or accounting");
        return false;
    }

    if !queue_global_lifecycle(ProcessIdentifier::from(2000)) {
        return false;
    }
    // SAFETY: the integration test owns the running kernel process state.
    unsafe { ProcessManager::set_delivery_cursor(2) };
    let mut failed_lifecycle_type: Option<MessageType> = None;
    // SAFETY: a lifecycle record is pending and the kernel process owns scheduling events.
    let failed_lifecycle: Result<(), SleepError> = unsafe {
        recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |message| {
            failed_lifecycle_type = Some(message.message_type);
            Err(Error::new(ErrorCode::TryAgain, "injected lifecycle copy failure"))
        })
    };
    if !matches!(failed_lifecycle, Err(SleepError::Generic(_)))
        || failed_lifecycle_type != Some(MessageType::ProcessCreationEvent)
        || unsafe { ProcessManager::delivery_cursor() } != 2
    {
        error!("failed lifecycle copy changed delivery state");
        return false;
    }
    let mut retried_lifecycle_type: Option<MessageType> = None;
    // SAFETY: the lifecycle record retained by the failed copy is still pending.
    if unsafe {
        recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |message| {
            retried_lifecycle_type = Some(message.message_type);
            Ok(())
        })
    }
    .is_err()
        || retried_lifecycle_type != failed_lifecycle_type
        || unsafe { ProcessManager::delivery_cursor() } != 0
    {
        error!("lifecycle retry did not commit the originally selected record");
        return false;
    }

    let interrupt_sequence: EventSequence = {
        let manager: &EventManager = match EventManager::get() {
            Ok(manager) => manager,
            Err(error) => {
                error!("failed to get event manager (error={error:?})");
                return false;
            },
        };
        let mut inner = match manager.try_borrow_mut() {
            Ok(inner) => inner,
            Err(error) => {
                error!("failed to borrow event manager (error={error:?})");
                return false;
            },
        };
        inner.interrupt_ownership[INTERRUPT_BIT] = Some(ProcessIdentifier::KERNEL);
        push_interrupt(&mut inner, INTERRUPT_BIT);
        inner.event_sequence
    };
    // SAFETY: the integration test owns the running kernel process state.
    unsafe { ProcessManager::set_delivery_cursor(0) };
    // SAFETY: the injected interrupt is pending.
    let failed_interrupt: Result<(), SleepError> = unsafe {
        recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |_| {
            Err(Error::new(ErrorCode::TryAgain, "injected interrupt copy failure"))
        })
    };
    let interrupt_retained: bool =
        match EventManager::get().and_then(|manager| manager.try_borrow_mut()) {
            Ok(inner) => {
                inner.pending_interrupts[INTERRUPT_BIT]
                    .front()
                    .map(|(sequence, _)| *sequence)
                    == Some(interrupt_sequence)
            },
            Err(error) => {
                error!("failed to inspect retained interrupt (error={error:?})");
                return false;
            },
        };
    if !matches!(failed_interrupt, Err(SleepError::Generic(_)))
        || !interrupt_retained
        || unsafe { ProcessManager::delivery_cursor() } != 0
    {
        error!("failed interrupt copy changed delivery state");
        return false;
    }
    // SAFETY: the interrupt retained by the failed copy is still pending.
    if unsafe { recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |_| Ok(())) }
        .is_err()
        || unsafe { ProcessManager::delivery_cursor() } != 1
    {
        error!("interrupt retry did not commit the selected event");
        return false;
    }
    match EventManager::get().and_then(|manager| manager.try_borrow_mut()) {
        Ok(mut inner) => {
            if !inner.pending_interrupts[INTERRUPT_BIT].is_empty() {
                error!("interrupt commit did not remove the selected event");
                return false;
            }
            inner.interrupt_ownership[INTERRUPT_BIT] = None;
        },
        Err(error) => {
            error!("failed to clean up interrupt test state (error={error:?})");
            return false;
        },
    }

    let exception_sequence: EventSequence = {
        let manager: &EventManager = match EventManager::get() {
            Ok(manager) => manager,
            Err(error) => {
                error!("failed to get event manager (error={error:?})");
                return false;
            },
        };
        let mut inner = match manager.try_borrow_mut() {
            Ok(inner) => inner,
            Err(error) => {
                error!("failed to borrow event manager (error={error:?})");
                return false;
            },
        };
        push_exception(
            &mut inner,
            EXCEPTION_BIT,
            ProcessIdentifier::KERNEL,
            ThreadIdentifier::KERNEL,
        );
        inner.event_sequence
    };
    // SAFETY: the integration test owns the running kernel process state.
    unsafe { ProcessManager::set_delivery_cursor(1) };
    // SAFETY: the injected exception is pending.
    let failed_exception: Result<(), SleepError> = unsafe {
        recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |_| {
            Err(Error::new(ErrorCode::TryAgain, "injected exception copy failure"))
        })
    };
    let exception_retained: bool =
        match EventManager::get().and_then(|manager| manager.try_borrow_mut()) {
            Ok(inner) => {
                inner.pending_exceptions[EXCEPTION_BIT]
                    .front()
                    .map(|(sequence, _, _, _)| *sequence)
                    == Some(exception_sequence)
            },
            Err(error) => {
                error!("failed to inspect retained exception (error={error:?})");
                return false;
            },
        };
    if !matches!(failed_exception, Err(SleepError::Generic(_)))
        || !exception_retained
        || unsafe { ProcessManager::delivery_cursor() } != 1
    {
        error!("failed exception copy changed delivery state");
        return false;
    }
    // SAFETY: the exception retained by the failed copy is still pending.
    if unsafe { recv_with(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL, |_| Ok(())) }
        .is_err()
        || unsafe { ProcessManager::delivery_cursor() } != 2
    {
        error!("exception retry did not advance the cursor after copying");
        return false;
    }
    match EventManager::get().and_then(|manager| manager.try_borrow_mut()) {
        Ok(mut inner) => {
            if inner.pending_exceptions[EXCEPTION_BIT]
                .front()
                .map(|(sequence, _, _, _)| *sequence)
                != Some(exception_sequence)
            {
                error!("successful exception copy removed the replayable event");
                return false;
            }
            resume_exception(&mut inner, EXCEPTION_BIT);
        },
        Err(error) => {
            error!("failed to clean up exception test state (error={error:?})");
            return false;
        },
    }

    true
}

///
/// # Description
///
/// Attempts production event selection against the global event and process managers, committing
/// any selected delivery before returning its message.
///
/// # Returns
///
/// The committed message, [`None`] when no item is eligible, or an error if selection fails.
///
fn receive_global_message() -> Result<Option<Message>, Error> {
    // SAFETY: the caller holds no reference to the process manager.
    let delivery: Option<PendingDelivery> = unsafe {
        EventManager::get()?
            .try_borrow_mut()?
            .try_wait(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL)
    }?;
    match delivery {
        Some(delivery) => {
            let message: Message = delivery.message().clone();
            // SAFETY: the selection borrow was released and the token is still current.
            unsafe { EventManager::commit(delivery) };
            Ok(Some(message))
        },
        None => Ok(None),
    }
}

///
/// # Description
///
/// Runs the ordered-delivery integration scenario while leaving capability and registration
/// cleanup to the caller.
///
/// # Parameters
///
/// - `capability_set`: Set to `true` after process-management capability is granted.
/// - `registered`: Set to `true` after lifecycle event ownership is registered.
///
/// # Returns
///
/// `true` if the production ordered-delivery scenario passes, otherwise `false`.
///
fn run_delivery_integration(capability_set: &mut bool, registered: &mut bool) -> bool {
    // SAFETY: this late integration test runs after process-manager initialization with interrupts
    // disabled and does not retain a process-manager reference across another global access.
    if let Err(error) = unsafe { ProcessManager::get_mut() }.capctl(
        ProcessIdentifier::KERNEL,
        Capability::ProcessManagement,
        true,
    ) {
        error!("failed to grant process-management capability (error={error:?})");
        return false;
    }
    *capability_set = true;

    let waiter: WaitingThreadGuard =
        match WaitingThreadGuard::register(ProcessIdentifier::KERNEL, ThreadIdentifier::KERNEL) {
            Ok(waiter) => waiter,
            Err(error) => {
                error!("failed to register test lifecycle waiter (error={error:?})");
                return false;
            },
        };
    let wait: Condvar = match EventManager::get().and_then(|manager| {
        manager
            .try_borrow_mut()
            .map(|manager| manager.get_wait().clone())
    }) {
        Ok(wait) => wait,
        Err(error) => {
            error!("failed to access test lifecycle condition variable (error={error:?})");
            return false;
        },
    };
    wait.stage_test_waiter(ThreadIdentifier::KERNEL);

    // Queue lifecycle before registration and force the first wakeup attempt to fail. The kernel
    // loop must re-arm it, and a later no-owner drain must leave the record buffered.
    if !queue_global_lifecycle(ProcessIdentifier::from(1000)) {
        return false;
    }
    if drain_lifecycle_wakeup(|| {
        Err(Error::new(ErrorCode::TryAgain, "injected lifecycle wakeup failure"))
    }) {
        error!("failed lifecycle wakeup was reported as delivered");
        return false;
    }
    // SAFETY: no process-manager reference is held while notifying the event manager.
    if !drain_lifecycle_wakeup(|| unsafe { EventManager::notify_process_lifecycle() }) {
        error!("failed lifecycle wakeup was not re-armed");
        return false;
    }
    if !wait.has_test_waiter(ThreadIdentifier::KERNEL) {
        error!("no-owner wakeup unexpectedly consumed the registered waiter");
        return false;
    }
    match receive_global_message() {
        Ok(None) => {},
        Ok(Some(message)) => {
            error!("lifecycle record was delivered before registration (message={message:?})");
            return false;
        },
        Err(error) => {
            error!("pre-registration receive failed (error={error:?})");
            return false;
        },
    }

    // Newer IPC must remain behind the buffered lifecycle record once the owner registers.
    if !post_global_ipc() {
        return false;
    }
    let scheduling: Event = Event::Scheduling(SchedulingEvent::ProcessCreation);
    if !matches!(
        crate::event::evctrl(
            ProcessIdentifier::KERNEL,
            u32::from(scheduling),
            u32::from(EventCtrlRequest::Register),
        ),
        KcallResult::Success(_)
    ) {
        error!("failed to register lifecycle ownership through event control");
        return false;
    }
    *registered = true;
    // SAFETY: no process-manager reference is held while notifying the event manager.
    if !drain_lifecycle_wakeup(|| unsafe { EventManager::notify_process_lifecycle() }) {
        error!("registration did not request a lifecycle-owner wakeup");
        return false;
    }
    if wait.has_test_waiter(ThreadIdentifier::KERNEL) {
        error!("registered lifecycle owner was not notified through its waiting list");
        return false;
    }
    drop(waiter);

    // Start at the message-like class and verify that production selection persists its successor
    // through the running process's cursor accessors.
    unsafe {
        ProcessManager::set_delivery_cursor(2);
    }

    let first: Option<Message> = match receive_global_message() {
        Ok(message) => message,
        Err(error) => {
            error!("post-registration lifecycle receive failed (error={error:?})");
            return false;
        },
    };
    if unsafe { ProcessManager::delivery_cursor() } != 0 {
        error!("production selection did not persist the process delivery cursor");
        return false;
    }
    let second: Option<Message> = match receive_global_message() {
        Ok(message) => message,
        Err(error) => {
            error!("post-registration IPC receive failed (error={error:?})");
            return false;
        },
    };
    if first.map(|message| message.message_type) != Some(MessageType::ProcessCreationEvent)
        || second.map(|message| message.message_type) != Some(MessageType::Ipc)
    {
        error!("older pre-registration lifecycle did not precede newer IPC");
        return false;
    }

    // Exercise the reverse production order while lifecycle ownership remains registered.
    if !post_global_ipc() || !queue_global_lifecycle(ProcessIdentifier::from(1001)) {
        return false;
    }
    // SAFETY: no process-manager reference is held while notifying the event manager.
    if !drain_lifecycle_wakeup(|| unsafe { EventManager::notify_process_lifecycle() }) {
        error!("queued lifecycle record did not request owner wakeup");
        return false;
    }
    let first: Option<Message> = match receive_global_message() {
        Ok(message) => message,
        Err(error) => {
            error!("older IPC receive failed (error={error:?})");
            return false;
        },
    };
    let second: Option<Message> = match receive_global_message() {
        Ok(message) => message,
        Err(error) => {
            error!("newer lifecycle receive failed (error={error:?})");
            return false;
        },
    };
    if first.map(|message| message.message_type) != Some(MessageType::Ipc)
        || second.map(|message| message.message_type) != Some(MessageType::ProcessCreationEvent)
    {
        error!("older IPC did not precede newer lifecycle");
        return false;
    }

    if !test_receive_copy_failure_is_transactional() {
        return false;
    }

    true
}

///
/// # Description
///
/// Delivers at most one event from `inner` by invoking `select_with` with every event class
/// unmasked, returning the delivered message (or [`None`] when nothing was delivered).
///
/// # Parameters
///
/// - `inner`: Event-manager state from which to select and commit an event.
///
/// # Returns
///
/// The committed message, or [`None`] if no event is eligible or selection fails.
///
/// # Panics
///
/// This function panics if event-only selection unexpectedly returns a message token or if the
/// selected event token is stale or invalid.
///
fn deliver_message(inner: &mut EventManagerInner) -> Option<Message> {
    let mut cursor: usize = 0;
    match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: usize::MAX,
            scheduling: usize::MAX,
        },
        cursor,
        |_, _| Ok(None),
    ) {
        Ok(Some(delivery)) => Some(commit_delivery(inner, &mut cursor, delivery, |_| {
            unreachable!("unexpected message token")
        })),
        Ok(None) => None,
        Err(e) => {
            error!("try_wait returned an unexpected error: {:?}", e);
            None
        },
    }
}

///
/// # Description
///
/// Delivers at most one event from `inner` and returns the delivered message type.
///
fn deliver_once(inner: &mut EventManagerInner) -> Option<MessageType> {
    deliver_message(inner).map(|message| message.message_type)
}

///
/// # Description
///
/// Delivers one exception and returns the exception event index encoded in the event descriptor.
///
fn deliver_exception_index(inner: &mut EventManagerInner) -> Option<usize> {
    let message: Message = match deliver_message(inner) {
        Some(message) => message,
        None => {
            error!("expected an exception message, got none");
            return None;
        },
    };

    if message.message_type != MessageType::Exception {
        error!("expected an exception message, got {:?}", message.message_type);
        return None;
    }

    match EventInformation::try_from(message) {
        Ok(info) => match info.id.event() {
            Event::Exception(exception) => Some(usize::from(exception)),
            other => {
                error!("expected an exception descriptor, got {:?}", other);
                None
            },
        },
        Err(e) => {
            error!("failed to decode exception event information: {:?}", e);
            None
        },
    }
}

//==================================================================================================
// Tests
//==================================================================================================

///
/// # Description
///
/// Verifies that each call delivers a single event by draining three queued interrupts. Bounded,
/// one-event-per-call delivery is the foundation of starvation freedom: it guarantees that the
/// delivery loop always makes forward progress and returns control to its caller.
///
fn test_each_call_delivers_a_single_interrupt() -> bool {
    let mut inner: EventManagerInner = make_inner();
    let bits: [usize; 3] = [1, 2, 3];
    for bit in bits {
        push_interrupt(&mut inner, bit);
    }

    // Each call delivers exactly one interrupt, so draining three queued interrupts takes exactly
    // three calls.
    for _ in bits {
        match deliver_once(&mut inner) {
            Some(MessageType::Interrupt) => {},
            other => {
                error!("expected an interrupt to be delivered, got {:?}", other);
                return false;
            },
        }
    }

    // Every queued interrupt has now been delivered.
    for bit in bits {
        if !inner.pending_interrupts[bit].is_empty() {
            error!("interrupt queue for bit {} was not drained", bit);
            return false;
        }
    }

    true
}

///
/// # Description
///
/// Verifies that the selection seam arbitrates events and a real local mailbox together.
///
/// # Returns
///
/// `true` if selection is non-consuming and commit advances the message-class cursor, otherwise
/// `false`.
///
fn test_selection_seam_combines_events_and_mailbox() -> bool {
    let mut inner: EventManagerInner = make_inner();
    let mut mailbox: Mailbox = Mailbox::default();
    let mut cursor: usize = 2;
    push_interrupt(&mut inner, 1);
    mailbox.send(new_test_delivery_sequence(0), ipc_message());

    let selected: PendingDelivery = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: usize::MAX,
            scheduling: usize::MAX,
        },
        cursor,
        |tid, _| Ok(peek_mailbox_delivery(&mailbox, tid)),
    ) {
        Ok(Some(delivery)) => delivery,
        Ok(None) => {
            error!("combined event/mailbox selection returned no delivery");
            return false;
        },
        Err(e) => {
            error!("combined event/mailbox selection failed: {:?}", e);
            return false;
        },
    };
    if selected.message().message_type != MessageType::Ipc {
        error!("message-like class did not select the mailbox message");
        return false;
    }
    if inner.pending_interrupts[1].is_empty() {
        error!("mailbox selection unexpectedly consumed the pending interrupt");
        return false;
    }
    if cursor != 2 {
        error!("message-like selection changed cursor before commit");
        return false;
    }
    let message: Message = commit_delivery(&mut inner, &mut cursor, selected, |token| {
        commit_mailbox_delivery(&mut mailbox, token)
    });
    if message.message_type != MessageType::Ipc || cursor != 0 {
        error!("message-like commit did not advance the cursor");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that interrupt and exception tokens remain current until commit and become stale after
/// commit while replayable exceptions remain queued.
///
/// # Returns
///
/// `true` if token identity and epoch transitions follow the transactional contract, otherwise
/// `false`.
///
/// # Panics
///
/// This function panics if an event-only selection unexpectedly returns a message token or if a
/// selected token violates the commit invariants under test.
///
fn test_event_tokens_are_stable_until_commit() -> bool {
    const INTERRUPT_BIT: usize = 1;
    const EXCEPTION_BIT: usize = 2;

    let mut inner: EventManagerInner = make_inner();
    let mut cursor: usize = 0;
    push_interrupt(&mut inner, INTERRUPT_BIT);
    let first_interrupt: PendingDelivery = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: 0,
            scheduling: 0,
        },
        cursor,
        |_, _| Ok(None),
    ) {
        Ok(Some(delivery)) => delivery,
        Ok(None) => {
            error!("interrupt token test selected no event");
            return false;
        },
        Err(error) => {
            error!("interrupt token selection failed (error={error:?})");
            return false;
        },
    };
    let (interrupt_epoch, interrupt_index, interrupt_sequence): (
        DeliveryEpoch,
        usize,
        EventSequence,
    ) = match first_interrupt.token {
        super::DeliveryToken::Interrupt {
            epoch,
            index,
            sequence,
        } => (epoch, index, sequence),
        _ => {
            error!("interrupt selection returned the wrong token kind");
            return false;
        },
    };
    if !inner.delivery_epoch_is_current(interrupt_epoch)
        || !inner.interrupt_token_is_current(interrupt_index, interrupt_sequence)
    {
        error!("new interrupt token was already stale");
        return false;
    }
    let retried_interrupt: PendingDelivery = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: 0,
            scheduling: 0,
        },
        cursor,
        |_, _| Ok(None),
    ) {
        Ok(Some(delivery)) => delivery,
        _ => {
            error!("abandoned interrupt token was not selected on retry");
            return false;
        },
    };
    match retried_interrupt.token {
        super::DeliveryToken::Interrupt {
            epoch,
            index,
            sequence,
        } if epoch == interrupt_epoch
            && index == interrupt_index
            && sequence == interrupt_sequence => {},
        _ => {
            error!("interrupt retry changed the selected token");
            return false;
        },
    }
    commit_delivery(&mut inner, &mut cursor, retried_interrupt, |_| {
        unreachable!("unexpected message token")
    });
    if inner.delivery_epoch_is_current(interrupt_epoch)
        || inner.interrupt_token_is_current(interrupt_index, interrupt_sequence)
        || cursor != 1
    {
        error!("committed interrupt token did not become stale");
        return false;
    }

    push_exception(&mut inner, EXCEPTION_BIT, test_pid(), test_tid());
    let first_exception: PendingDelivery = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: 0,
            exceptions: usize::MAX,
            scheduling: 0,
        },
        cursor,
        |_, _| Ok(None),
    ) {
        Ok(Some(delivery)) => delivery,
        Ok(None) => {
            error!("exception token test selected no event");
            return false;
        },
        Err(error) => {
            error!("exception token selection failed (error={error:?})");
            return false;
        },
    };
    let (exception_epoch, exception_index, exception_sequence): (
        DeliveryEpoch,
        usize,
        EventSequence,
    ) = match first_exception.token {
        super::DeliveryToken::Exception {
            epoch,
            index,
            sequence,
        } => (epoch, index, sequence),
        _ => {
            error!("exception selection returned the wrong token kind");
            return false;
        },
    };
    if !inner.delivery_epoch_is_current(exception_epoch)
        || !inner.exception_token_is_current(exception_index, exception_sequence)
    {
        error!("new exception token was already stale");
        return false;
    }
    let retried_exception: PendingDelivery = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: 0,
            exceptions: usize::MAX,
            scheduling: 0,
        },
        cursor,
        |_, _| Ok(None),
    ) {
        Ok(Some(delivery)) => delivery,
        _ => {
            error!("abandoned exception token was not selected on retry");
            return false;
        },
    };
    match retried_exception.token {
        super::DeliveryToken::Exception {
            epoch,
            index,
            sequence,
        } if epoch == exception_epoch
            && index == exception_index
            && sequence == exception_sequence => {},
        _ => {
            error!("exception retry changed the selected token");
            return false;
        },
    }
    commit_delivery(&mut inner, &mut cursor, retried_exception, |_| {
        unreachable!("unexpected message token")
    });
    if inner.delivery_epoch_is_current(exception_epoch)
        || !inner.exception_token_is_current(exception_index, exception_sequence)
        || cursor != 2
    {
        error!("committed exception token did not become stale");
        return false;
    }
    resume_exception(&mut inner, EXCEPTION_BIT);

    true
}

///
/// # Description
///
/// Verifies that one class-wide scheduling owner is eligible for every scheduling event.
///
/// # Returns
///
/// `true` if one owner receives every scheduling-event mask bit and a non-owner receives none,
/// otherwise `false`.
///
fn test_scheduling_owner_mask_includes_all_events() -> bool {
    let mut inner: EventManagerInner = make_inner();
    inner.scheduling_owner = Some(test_pid());

    let owner_masks: EventMasks = inner.event_masks(test_pid());
    for event in SchedulingEvent::VALUES {
        let event_mask: usize = 1usize << usize::from(event);
        if owner_masks.scheduling & event_mask == 0 {
            error!("scheduling owner mask omitted event: {:?}", event);
            return false;
        }
    }

    let non_owner: ProcessIdentifier = ProcessIdentifier::from(1001);
    if inner.event_masks(non_owner).scheduling != 0 {
        error!("non-owner received scheduling event eligibility");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that a receive attempt refreshes scheduling ownership acquired while the receiver was
/// blocked. The first attempt models the receiver before registration; the second models its retry
/// after the registration wakeup.
///
/// # Returns
///
/// `true` if the retry observes newly acquired scheduling ownership, otherwise `false`.
///
fn test_receive_refreshes_scheduling_ownership() -> bool {
    let mut inner: EventManagerInner = make_inner();
    let cursor: usize = 2;
    let mut lifecycle_eligible: bool = true;

    let first: Option<PendingDelivery> =
        match inner.try_wait_with(test_tid(), test_pid(), cursor, |_, eligible| {
            lifecycle_eligible = eligible;
            Ok(None)
        }) {
            Ok(message) => message,
            Err(e) => {
                error!("pre-registration selection failed: {:?}", e);
                return false;
            },
        };
    if first.is_some() || lifecycle_eligible {
        error!("lifecycle delivery was eligible before scheduling registration");
        return false;
    }

    inner.scheduling_owner = Some(test_pid());
    let second: Option<PendingDelivery> =
        match inner.try_wait_with(test_tid(), test_pid(), cursor, |_, eligible| {
            lifecycle_eligible = eligible;
            if eligible {
                Ok(Some(synthetic_message_delivery(lifecycle_message())))
            } else {
                Ok(None)
            }
        }) {
            Ok(message) => message,
            Err(e) => {
                error!("post-registration selection failed: {:?}", e);
                return false;
            },
        };
    if second
        .as_ref()
        .map(|delivery| delivery.message().message_type)
        != Some(MessageType::ProcessCreationEvent)
        || !lifecycle_eligible
    {
        error!("scheduling registration was not observed on the next receive attempt");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that continuously eligible interrupt, exception, and message-like classes are each
/// selected within three successful deliveries.
///
/// # Returns
///
/// `true` if all service classes receive bounded service in cursor order, otherwise `false`.
///
fn test_all_service_classes_receive_bounded_service() -> bool {
    const INTERRUPT_BIT: usize = 1;
    const EXCEPTION_BIT: usize = 2;

    let mut inner: EventManagerInner = make_inner();
    let mut cursor: usize = 0;
    push_exception(&mut inner, EXCEPTION_BIT, test_pid(), test_tid());

    let mut delivered: [Option<MessageType>; 3] = [None; 3];
    for slot in delivered.iter_mut() {
        push_interrupt(&mut inner, INTERRUPT_BIT);
        let delivery: Option<PendingDelivery> = match inner.select_with(
            test_tid(),
            test_pid(),
            EventMasks {
                interrupts: usize::MAX,
                exceptions: usize::MAX,
                scheduling: usize::MAX,
            },
            cursor,
            |_, _| Ok(Some(synthetic_message_delivery(ipc_message()))),
        ) {
            Ok(delivery) => delivery,
            Err(e) => {
                error!("service-class selection failed: {:?}", e);
                return false;
            },
        };
        *slot = delivery.map(|delivery| {
            commit_delivery(&mut inner, &mut cursor, delivery, |_| {}).message_type
        });
    }

    let expected: [Option<MessageType>; 3] = [
        Some(MessageType::Interrupt),
        Some(MessageType::Exception),
        Some(MessageType::Ipc),
    ];
    if delivered != expected {
        error!("service classes were not selected fairly: {:?}", delivered);
        return false;
    }
    resume_exception(&mut inner, EXCEPTION_BIT);

    true
}

///
/// # Description
///
/// Verifies that masked event classes do not block an eligible mailbox message.
///
/// # Returns
///
/// `true` if the mailbox message is committed while masked events remain pending, otherwise
/// `false`.
///
fn test_masked_event_classes_do_not_block_mailbox() -> bool {
    const INTERRUPT_BIT: usize = 1;
    const EXCEPTION_BIT: usize = 2;

    let mut inner: EventManagerInner = make_inner();
    let mut mailbox: Mailbox = Mailbox::default();
    let mut cursor: usize = 0;
    push_interrupt(&mut inner, INTERRUPT_BIT);
    push_exception(&mut inner, EXCEPTION_BIT, test_pid(), test_tid());
    mailbox.send(new_test_delivery_sequence(0), ipc_message());

    let delivery: Option<PendingDelivery> = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: 0,
            exceptions: 0,
            scheduling: 0,
        },
        cursor,
        |tid, _| Ok(peek_mailbox_delivery(&mailbox, tid)),
    ) {
        Ok(delivery) => delivery,
        Err(e) => {
            error!("masked-class selection failed: {:?}", e);
            return false;
        },
    };
    let selected: Option<MessageType> = delivery.map(|delivery| {
        commit_delivery(&mut inner, &mut cursor, delivery, |token| {
            commit_mailbox_delivery(&mut mailbox, token)
        })
        .message_type
    });
    if selected != Some(MessageType::Ipc) {
        error!("masked event classes blocked mailbox delivery: {:?}", selected);
        return false;
    }
    if inner.pending_interrupts[INTERRUPT_BIT].is_empty()
        || inner.pending_exceptions[EXCEPTION_BIT].is_empty()
    {
        error!("masked event selection consumed an ineligible event");
        return false;
    }
    resume_exception(&mut inner, EXCEPTION_BIT);

    true
}

///
/// # Description
///
/// Verifies that global event generation does not perturb a receiver's service cursor.
///
/// # Returns
///
/// `true` if receiver cursors remain independent of global event traffic and each other, otherwise
/// `false`.
///
fn test_global_event_count_does_not_perturb_receiver_cursor() -> bool {
    let mut inner: EventManagerInner = make_inner();
    let mut receiver_cursor: usize = 0;
    let mut other_receiver_cursor: usize = 2;
    push_interrupt(&mut inner, 1);

    // A receive by another process advances only that process's cursor and leaves the interrupt
    // pending for this receiver.
    let other_delivery: Option<PendingDelivery> = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: usize::MAX,
            scheduling: usize::MAX,
        },
        other_receiver_cursor,
        |_, _| Ok(Some(synthetic_message_delivery(ipc_message()))),
    ) {
        Ok(delivery) => delivery,
        Err(e) => {
            error!("other-receiver selection failed: {:?}", e);
            return false;
        },
    };
    let other_selected: Option<MessageType> = other_delivery.map(|delivery| {
        commit_delivery(&mut inner, &mut other_receiver_cursor, delivery, |_| {}).message_type
    });
    if other_selected != Some(MessageType::Ipc)
        || other_receiver_cursor != 0
        || receiver_cursor != 0
    {
        error!(
            "receive cursors were not independent: selected={:?}, receiver={}, other={}",
            other_selected, receiver_cursor, other_receiver_cursor
        );
        return false;
    }

    // Model unrelated event traffic changing the global event counter after this receiver last ran.
    inner.event_sequence = inner.event_sequence.wrapping_add(97);
    let delivery: Option<PendingDelivery> = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: usize::MAX,
            scheduling: usize::MAX,
        },
        receiver_cursor,
        |_, _| Ok(Some(synthetic_message_delivery(ipc_message()))),
    ) {
        Ok(delivery) => delivery,
        Err(e) => {
            error!("cursor-isolation selection failed: {:?}", e);
            return false;
        },
    };
    let selected: Option<MessageType> = delivery.map(|delivery| {
        commit_delivery(&mut inner, &mut receiver_cursor, delivery, |_| {}).message_type
    });
    if selected != Some(MessageType::Interrupt) || receiver_cursor != 1 {
        error!(
            "receiver cursor was perturbed by global traffic: selected={:?}, cursor={}",
            selected, receiver_cursor
        );
        return false;
    }
    if other_receiver_cursor != 0 {
        error!("receiver selection changed another process's cursor");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies the FIFO-by-sequence selection rule that underlies starvation-free interrupt and
/// exception delivery: among the eligible bits, [`EventManagerInner::smallest_pending_front`] picks
/// the queue head with the smallest event sequence number rather than the lowest-numbered bit. This
/// is the structural property that makes delivery starvation-free, exercised here directly and
/// independently of the surrounding `try_wait` machinery (and shared verbatim by both the interrupt
/// and exception delivery paths).
///
/// # Returns
///
/// `true` if selection chooses the oldest eligible sequence and respects the eligibility mask,
/// otherwise `false`.
///
fn test_smallest_pending_front_selects_oldest_sequence() -> bool {
    // Bit 1 (low) holds a newer event (sequence 20); bit 5 (high) holds an older one (sequence
    // 10). Selection must pick bit 5, proving a lower-numbered bit does not win merely by being
    // scanned first, the previous bit-0-first bias.
    let front_seq = |bit: usize| -> Option<EventSequence> {
        match bit {
            1 => Some(EventSequence::new(20)),
            5 => Some(EventSequence::new(10)),
            _ => None,
        }
    };

    let both: usize = (1usize << 1) | (1usize << 5);
    if EventManagerInner::smallest_pending_front(both, front_seq) != Some(5) {
        error!("oldest event (bit 5) must be selected ahead of the lower-numbered bit 1");
        return false;
    }

    // When only the lower bit is eligible, it is selected.
    if EventManagerInner::smallest_pending_front(1usize << 1, front_seq) != Some(1) {
        error!("bit 1 must be selected when it is the only eligible bit");
        return false;
    }

    // A pending entry on a bit outside the mask is never selected.
    if EventManagerInner::smallest_pending_front(1usize << 1, |bit| match bit {
        5 => Some(EventSequence::new(10)),
        _ => None,
    })
    .is_some()
    {
        error!("a bit outside the mask must not be selected");
        return false;
    }

    // No eligible bit has a pending entry.
    if EventManagerInner::smallest_pending_front(both, |_| None).is_some() {
        error!("no selection is expected when every eligible queue is empty");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that interrupt delivery follows FIFO order by event sequence number end-to-end: an older
/// interrupt enqueued on a high-numbered bit is delivered before a newer interrupt on a
/// low-numbered bit. The previous bit-0-first scan delivered the low-numbered bit regardless of
/// age; FIFO-by-sequence delivers the oldest first.
///
fn test_interrupt_delivery_is_fifo_by_sequence() -> bool {
    const OLDER_HIGH_BIT: usize = 5;
    const NEWER_LOW_BIT: usize = 1;

    let mut inner: EventManagerInner = make_inner();

    // Enqueue the older interrupt on the higher bit, then a newer interrupt on the lower bit.
    push_interrupt(&mut inner, OLDER_HIGH_BIT);
    push_interrupt(&mut inner, NEWER_LOW_BIT);

    // The first delivery must serve the older interrupt (high bit), not the lower-numbered one.
    if deliver_once(&mut inner) != Some(MessageType::Interrupt) {
        error!("expected an interrupt to be delivered");
        return false;
    }
    if !inner.pending_interrupts[OLDER_HIGH_BIT].is_empty() {
        error!("the oldest interrupt (bit {}) must be delivered first", OLDER_HIGH_BIT);
        return false;
    }
    if inner.pending_interrupts[NEWER_LOW_BIT].is_empty() {
        error!("the newer interrupt (bit {}) was delivered out of order", NEWER_LOW_BIT);
        return false;
    }

    // The second delivery then serves the remaining (newer) interrupt.
    if deliver_once(&mut inner) != Some(MessageType::Interrupt) {
        error!("expected the second interrupt to be delivered");
        return false;
    }
    if !inner.pending_interrupts[NEWER_LOW_BIT].is_empty() {
        error!("interrupt queue for bit {} was not drained", NEWER_LOW_BIT);
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that an interrupt is not starved by a continuous stream of lower-numbered interrupts: an
/// interrupt on a high-numbered bit is enqueued first (oldest sequence number), then a
/// lower-numbered interrupt is enqueued before every delivery. Under the previous bit-0-first scan
/// the low bit would win on every call and the older interrupt would never be delivered;
/// FIFO-by-sequence delivers the oldest within a bounded number of calls.
///
fn test_oldest_interrupt_not_starved_by_low_bit_load() -> bool {
    const VICTIM_HIGH_BIT: usize = 7;
    const LOAD_LOW_BIT: usize = 1;

    let mut inner: EventManagerInner = make_inner();

    // The victim: an older interrupt on a high-numbered bit.
    push_interrupt(&mut inner, VICTIM_HIGH_BIT);

    // A generous, implementation-agnostic budget within which the victim must be served.
    let max_deliveries: usize = 4 * EventManagerInner::NUMBER_EVENT_CLASSES;

    let mut victim_seen: bool = false;
    for _ in 0..max_deliveries {
        // Offer continuous lower-numbered interrupt load alongside the standing victim.
        push_interrupt(&mut inner, LOAD_LOW_BIT);
        let _ = deliver_once(&mut inner);
        if inner.pending_interrupts[VICTIM_HIGH_BIT].is_empty() {
            victim_seen = true;
            break;
        }
    }

    if !victim_seen {
        error!("the oldest interrupt was starved by continuous low-bit load");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that exception delivery follows FIFO order by event sequence number end-to-end: an older
/// exception enqueued on a high-numbered bit is delivered before a newer exception on a
/// low-numbered bit.
///
fn test_exception_delivery_is_fifo_by_sequence() -> bool {
    const OLDER_HIGH_BIT: usize = 5;
    const NEWER_LOW_BIT: usize = 1;

    let mut inner: EventManagerInner = make_inner();
    push_exception(&mut inner, OLDER_HIGH_BIT, test_pid(), test_tid());
    push_exception(&mut inner, NEWER_LOW_BIT, test_pid(), test_tid());

    let first: Option<usize> = deliver_exception_index(&mut inner);
    if first != Some(OLDER_HIGH_BIT) {
        error!("expected exception bit {} first, got {:?}", OLDER_HIGH_BIT, first);
        return false;
    }
    resume_exception(&mut inner, OLDER_HIGH_BIT);

    let second: Option<usize> = deliver_exception_index(&mut inner);
    if second != Some(NEWER_LOW_BIT) {
        error!("expected exception bit {} second, got {:?}", NEWER_LOW_BIT, second);
        return false;
    }
    resume_exception(&mut inner, NEWER_LOW_BIT);

    true
}

///
/// # Description
///
/// Verifies that an older high-numbered exception is not starved by continuous lower-numbered
/// exception load.
///
fn test_oldest_exception_not_starved_by_low_bit_load() -> bool {
    const VICTIM_HIGH_BIT: usize = 7;
    const LOAD_LOW_BIT: usize = 1;

    let mut inner: EventManagerInner = make_inner();
    push_exception(&mut inner, VICTIM_HIGH_BIT, test_pid(), test_tid());

    let max_deliveries: usize = 4 * EventManagerInner::NUMBER_EVENT_CLASSES;

    for _ in 0..max_deliveries {
        push_exception(&mut inner, LOAD_LOW_BIT, test_pid(), test_tid());
        if deliver_exception_index(&mut inner) == Some(VICTIM_HIGH_BIT) {
            resume_exception(&mut inner, VICTIM_HIGH_BIT);
            return true;
        }
        resume_exception(&mut inner, LOAD_LOW_BIT);
    }

    error!("the oldest exception was starved by continuous low-bit load");
    false
}

///
/// # Description
///
/// Verifies that delivery ordering follows the full-width event sequence number rather than the
/// truncated [`EventDescriptor`] id, so FIFO order is preserved across the descriptor's id wrap
/// boundary (issue #2674). [`EventDescriptor`] stores its id in a narrow field (24 bits on the
/// kernel's 32-bit target), so an event generated just past the wrap reads back a *smaller* id than
/// an older one generated just before it. Ordering by the descriptor id would deliver the newer
/// event first; ordering by the `u64` sequence number delivers the older one first, as it must.
///
/// # Returns
///
/// `true` if full-width event ordering remains FIFO across descriptor identifier wrap, otherwise
/// `false`.
///
fn test_delivery_orders_by_sequence_across_id_wrap() -> bool {
    const OLDER_HIGH_BIT: usize = 7;
    const NEWER_LOW_BIT: usize = 1;

    // Width of the `EventDescriptor` id field: the top bit is reserved and the low `BIT_LENGTH` bits
    // hold the event tag, leaving the rest for the id. The id read back from a descriptor is the
    // generation sequence number truncated to this many bits.
    let id_bits: u32 = usize::BITS - 1 - Event::BIT_LENGTH as u32;
    let wrap: u64 = 1u64 << id_bits;

    let mut inner: EventManagerInner = make_inner();

    // Arrange for the next two generated events to straddle the id wrap: the older event lands on
    // the last id before the wrap (`wrap - 1`) and the newer event on the wrapped id (`0`).
    inner.event_sequence = EventSequence::new(wrap - 2);

    // Older event (high bit): sequence `wrap - 1`, descriptor id `wrap - 1` (the maximum).
    push_interrupt(&mut inner, OLDER_HIGH_BIT);
    // Newer event (low bit): sequence `wrap`, descriptor id `0` (wrapped).
    push_interrupt(&mut inner, NEWER_LOW_BIT);

    // Confirm the wrap actually occurred, otherwise the test would not exercise the regression.
    let older_id: usize = inner.pending_interrupts[OLDER_HIGH_BIT]
        .front()
        .map(|(_, descriptor)| descriptor.id())
        .unwrap_or(0);
    let newer_id: usize = inner.pending_interrupts[NEWER_LOW_BIT]
        .front()
        .map(|(_, descriptor)| descriptor.id())
        .unwrap_or(0);
    if older_id <= newer_id {
        error!(
            "test setup did not straddle the id wrap (older_id={}, newer_id={})",
            older_id, newer_id
        );
        return false;
    }

    // The older event must be delivered first even though its descriptor id is larger.
    if deliver_once(&mut inner) != Some(MessageType::Interrupt) {
        error!("expected an interrupt to be delivered");
        return false;
    }
    if !inner.pending_interrupts[OLDER_HIGH_BIT].is_empty() {
        error!("the older interrupt must be delivered first across the id wrap");
        return false;
    }
    if inner.pending_interrupts[NEWER_LOW_BIT].is_empty() {
        error!("the newer interrupt was delivered out of order across the id wrap");
        return false;
    }

    true
}

//==================================================================================================
// Test Runner
//==================================================================================================

///
/// # Description
///
/// Runs all event-manager in-kernel tests, returning `true` only if every test passed.
///
/// # Returns
///
/// `true` if every event-manager test passes, otherwise `false`.
///
pub fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_each_call_delivers_a_single_interrupt);
    passed &= run_test!(test_selection_seam_combines_events_and_mailbox);
    passed &= run_test!(test_event_tokens_are_stable_until_commit);
    passed &= run_test!(test_scheduling_owner_mask_includes_all_events);
    passed &= run_test!(test_receive_refreshes_scheduling_ownership);
    passed &= run_test!(test_all_service_classes_receive_bounded_service);
    passed &= run_test!(test_masked_event_classes_do_not_block_mailbox);
    passed &= run_test!(test_global_event_count_does_not_perturb_receiver_cursor);
    passed &= run_test!(test_smallest_pending_front_selects_oldest_sequence);
    passed &= run_test!(test_interrupt_delivery_is_fifo_by_sequence);
    passed &= run_test!(test_oldest_interrupt_not_starved_by_low_bit_load);
    passed &= run_test!(test_exception_delivery_is_fifo_by_sequence);
    passed &= run_test!(test_oldest_exception_not_starved_by_low_bit_load);
    passed &= run_test!(test_delivery_orders_by_sequence_across_id_wrap);
    passed
}

/// Exercises production-path ordered delivery after process-manager initialization.
fn test_ordered_delivery_production_path() -> bool {
    // Install a global event manager without registering hardware handlers. This test runs before
    // user processes are spawned, and the manager is removed after all ownership guards are dropped.
    unsafe {
        MANAGER = Some(EventManager(RefCell::new(make_inner())));
    }

    let mut capability_set: bool = false;
    let mut registered: bool = false;
    let passed: bool = run_delivery_integration(&mut capability_set, &mut registered);
    let mut cleanup_passed: bool = true;

    if registered {
        let scheduling: Event = Event::Scheduling(SchedulingEvent::ProcessCreation);
        if !matches!(
            crate::event::evctrl(
                ProcessIdentifier::KERNEL,
                u32::from(scheduling),
                u32::from(EventCtrlRequest::Unregister),
            ),
            KcallResult::Success(_)
        ) {
            error!("failed to unregister test lifecycle ownership");
            cleanup_passed = false;
        } else {
            registered = false;
        }
    }
    if capability_set {
        // SAFETY: process-manager access is synchronized during single-threaded kernel startup.
        if let Err(error) = unsafe { ProcessManager::get_mut() }.capctl(
            ProcessIdentifier::KERNEL,
            Capability::ProcessManagement,
            false,
        ) {
            error!("failed to revoke test process-management capability (error={error:?})");
            cleanup_passed = false;
        }
    }

    // Remove the test manager only after every ownership guard has been released.
    if !registered {
        unsafe {
            MANAGER = None;
        }
    }

    passed && cleanup_passed
}

/// Runs production-path ordered-delivery tests after process-manager initialization.
pub fn test_delivery_integration() -> bool {
    run_test!(test_ordered_delivery_production_path)
}
