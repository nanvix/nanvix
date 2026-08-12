// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    EventManager,
    EventManagerInner,
    EventMasks,
    ExceptionEventInformation,
    WaitingThreadGuard,
    MANAGER,
};
use crate::{
    hal::arch::ExceptionInformation,
    ipc::{
        DeliverySequence,
        Mailbox,
    },
    kcall::{
        drain_lifecycle_wakeup,
        KcallResult,
    },
    pm::{
        sync::condvar::Condvar,
        ProcessManager,
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
fn make_inner() -> EventManagerInner {
    EventManagerInner {
        interrupt_capable: true,
        nevents: 0,
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
fn push_interrupt(inner: &mut EventManagerInner, bit: usize) {
    inner.nevents += 1;
    let event: Event = Event::Interrupt(InterruptEvent::VALUES[bit]);
    let descriptor: EventDescriptor = EventDescriptor::new(inner.nevents as usize, event);
    inner.pending_interrupts[bit].push_back((inner.nevents, descriptor));
}

///
/// # Description
///
/// Enqueues a pending exception event on `bit`, owned by `pid`, stamping it with the next event
/// sequence number. The event manager re-queues exceptions on each delivery until the owner resumes
/// from them, so a test that does not want continuous re-delivery must clear the slot once observed
/// (see [`resume_exception`]).
///
fn push_exception(
    inner: &mut EventManagerInner,
    bit: usize,
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
) {
    inner.nevents += 1;
    inner.exception_ownership[bit] = Some(pid);
    let event: Event = Event::Exception(ExceptionEvent::VALUES[bit]);
    let descriptor: EventDescriptor = EventDescriptor::new(inner.nevents as usize, event);
    // SAFETY: `ExceptionInformation` is plain-old-data: a register snapshot of fixed-width integer
    // fields (`u32` on 32-bit x86, `u64` on x86_64) for which the all-zero bit pattern is a valid
    // value on every supported architecture. The tests only need a placeholder to exercise delivery.
    let info: ExceptionInformation = unsafe { core::mem::zeroed() };
    inner.pending_exceptions[bit].push_back((
        inner.nevents,
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

/// Posts a process-directed IPC message through the global process manager.
fn post_global_ipc() -> bool {
    let message: Message = Message {
        source: MessageSender::KERNEL,
        destination: MessageReceiver::KERNEL,
        message_type: MessageType::Ipc,
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

/// Attempts production event selection against the global event and process managers.
fn receive_global_message() -> Result<Option<Message>, Error> {
    // SAFETY: the caller holds no reference to the process manager.
    unsafe {
        EventManager::get()?
            .try_borrow_mut()?
            .try_wait(ThreadIdentifier::KERNEL, ProcessIdentifier::KERNEL)
    }
}

/// Runs the ordered-delivery integration scenario while cleanup remains owned by the caller.
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

    true
}

///
/// # Description
///
/// Delivers at most one event from `inner` by invoking `try_wait` with every event class unmasked,
/// returning the delivered message (or [`None`] when nothing was delivered).
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
        &mut cursor,
        |_, _| Ok(None),
    ) {
        Ok(Some(message)) => Some(message),
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
fn test_selection_seam_combines_events_and_mailbox() -> bool {
    let mut inner: EventManagerInner = make_inner();
    let mut mailbox: Mailbox = Mailbox::default();
    let mut cursor: usize = 2;
    push_interrupt(&mut inner, 1);
    mailbox.send(DeliverySequence::new(0), ipc_message());

    let selected: Option<Message> = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: usize::MAX,
            scheduling: usize::MAX,
        },
        &mut cursor,
        |tid, _| Ok(mailbox.receive(tid).map(|(_, message)| message)),
    ) {
        Ok(message) => message,
        Err(e) => {
            error!("combined event/mailbox selection failed: {:?}", e);
            return false;
        },
    };
    if selected.map(|message| message.message_type) != Some(MessageType::Ipc) {
        error!("message-like class did not select the mailbox message");
        return false;
    }
    if inner.pending_interrupts[1].is_empty() {
        error!("mailbox selection unexpectedly consumed the pending interrupt");
        return false;
    }
    if cursor != 0 {
        error!("message-like selection advanced cursor to {}, expected 0", cursor);
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
fn test_receive_refreshes_scheduling_ownership() -> bool {
    let mut inner: EventManagerInner = make_inner();
    let mut cursor: usize = 2;
    let mut lifecycle_eligible: bool = true;

    let first: Option<Message> =
        match inner.try_wait_with(test_tid(), test_pid(), &mut cursor, |_, eligible| {
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
    let second: Option<Message> =
        match inner.try_wait_with(test_tid(), test_pid(), &mut cursor, |_, eligible| {
            lifecycle_eligible = eligible;
            if eligible {
                Ok(Some(lifecycle_message()))
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
    if second.map(|message| message.message_type) != Some(MessageType::ProcessCreationEvent)
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
fn test_all_service_classes_receive_bounded_service() -> bool {
    const INTERRUPT_BIT: usize = 1;
    const EXCEPTION_BIT: usize = 2;

    let mut inner: EventManagerInner = make_inner();
    let mut cursor: usize = 0;
    push_exception(&mut inner, EXCEPTION_BIT, test_pid(), test_tid());

    let mut delivered: [Option<MessageType>; 3] = [None; 3];
    for slot in delivered.iter_mut() {
        push_interrupt(&mut inner, INTERRUPT_BIT);
        *slot = match inner.select_with(
            test_tid(),
            test_pid(),
            EventMasks {
                interrupts: usize::MAX,
                exceptions: usize::MAX,
                scheduling: usize::MAX,
            },
            &mut cursor,
            |_, _| Ok(Some(ipc_message())),
        ) {
            Ok(message) => message.map(|message| message.message_type),
            Err(e) => {
                error!("service-class selection failed: {:?}", e);
                return false;
            },
        };
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
fn test_masked_event_classes_do_not_block_mailbox() -> bool {
    const INTERRUPT_BIT: usize = 1;
    const EXCEPTION_BIT: usize = 2;

    let mut inner: EventManagerInner = make_inner();
    let mut mailbox: Mailbox = Mailbox::default();
    let mut cursor: usize = 0;
    push_interrupt(&mut inner, INTERRUPT_BIT);
    push_exception(&mut inner, EXCEPTION_BIT, test_pid(), test_tid());
    mailbox.send(DeliverySequence::new(0), ipc_message());

    let selected: Option<MessageType> = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: 0,
            exceptions: 0,
            scheduling: 0,
        },
        &mut cursor,
        |tid, _| Ok(mailbox.receive(tid).map(|(_, message)| message)),
    ) {
        Ok(message) => message.map(|message| message.message_type),
        Err(e) => {
            error!("masked-class selection failed: {:?}", e);
            return false;
        },
    };
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
fn test_global_event_count_does_not_perturb_receiver_cursor() -> bool {
    let mut inner: EventManagerInner = make_inner();
    let mut receiver_cursor: usize = 0;
    let mut other_receiver_cursor: usize = 2;
    push_interrupt(&mut inner, 1);

    // A receive by another process advances only that process's cursor and leaves the interrupt
    // pending for this receiver.
    let other_selected: Option<MessageType> = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: usize::MAX,
            scheduling: usize::MAX,
        },
        &mut other_receiver_cursor,
        |_, _| Ok(Some(ipc_message())),
    ) {
        Ok(message) => message.map(|message| message.message_type),
        Err(e) => {
            error!("other-receiver selection failed: {:?}", e);
            return false;
        },
    };
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
    inner.nevents = inner.nevents.wrapping_add(97);
    let selected: Option<MessageType> = match inner.select_with(
        test_tid(),
        test_pid(),
        EventMasks {
            interrupts: usize::MAX,
            exceptions: usize::MAX,
            scheduling: usize::MAX,
        },
        &mut receiver_cursor,
        |_, _| Ok(Some(ipc_message())),
    ) {
        Ok(message) => message.map(|message| message.message_type),
        Err(e) => {
            error!("cursor-isolation selection failed: {:?}", e);
            return false;
        },
    };
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
fn test_smallest_pending_front_selects_oldest_sequence() -> bool {
    // Bit 1 (low) holds a newer event (sequence 20); bit 5 (high) holds an older one (sequence
    // 10). Selection must pick bit 5, proving a lower-numbered bit does not win merely by being
    // scanned first, the previous bit-0-first bias.
    let front_seq = |bit: usize| -> Option<u64> {
        match bit {
            1 => Some(20),
            5 => Some(10),
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
        5 => Some(10),
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
    inner.nevents = wrap - 2;

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
pub fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_each_call_delivers_a_single_interrupt);
    passed &= run_test!(test_selection_seam_combines_events_and_mailbox);
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
