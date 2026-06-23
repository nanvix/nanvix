// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    EventManagerInner,
    ExceptionEventInformation,
    SchedulingNotification,
};
use crate::{
    hal::arch::ExceptionInformation,
    pm::sync::condvar::Condvar,
};
use ::alloc::collections::{
    LinkedList,
    VecDeque,
};
use ::sys::{
    event::{
        Event,
        EventDescriptor,
        EventInformation,
        ExceptionEvent,
        InterruptEvent,
        ProcessCreationInfo,
        ProcessRole,
        ProcessTerminationInfo,
        SchedulingEvent,
    },
    ipc::{
        Message,
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
        interrupt_ownership: core::array::from_fn(|_| None),
        pending_interrupts: core::array::from_fn(|_| LinkedList::default()),
        exception_ownership: core::array::from_fn(|_| None),
        pending_exceptions: core::array::from_fn(|_| LinkedList::default()),
        scheduling_owner: None,
        pending_scheduling: LinkedList::default(),
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

///
/// # Description
///
/// Enqueues a pending process-creation scheduling event, stamping it with the next event sequence
/// number.
///
fn push_creation(inner: &mut EventManagerInner, pid: ProcessIdentifier) {
    inner.nevents += 1;
    let event: Event = Event::Scheduling(SchedulingEvent::ProcessCreation);
    let descriptor: EventDescriptor = EventDescriptor::new(inner.nevents as usize, event);
    let info: ProcessCreationInfo =
        ProcessCreationInfo::new(pid, ProcessIdentifier::KERNEL, ProcessRole::User);
    inner.pending_scheduling.push_back((
        inner.nevents,
        descriptor,
        SchedulingNotification::Creation(info),
    ));
}

///
/// # Description
///
/// Enqueues a pending process-termination scheduling event, stamping it with the next event
/// sequence number.
///
fn push_termination(inner: &mut EventManagerInner, pid: ProcessIdentifier) {
    inner.nevents += 1;
    let event: Event = Event::Scheduling(SchedulingEvent::ProcessTermination);
    let descriptor: EventDescriptor = EventDescriptor::new(inner.nevents as usize, event);
    let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
        pid,
        ExitStatus::ok(),
        ProcessIdentifier::KERNEL,
        ProcessRole::User,
    );
    inner.pending_scheduling.push_back((
        inner.nevents,
        descriptor,
        SchedulingNotification::Termination(info),
    ));
}

///
/// # Description
///
/// Delivers at most one event from `inner` by invoking `try_wait` with every event class unmasked,
/// returning the delivered message (or [`None`] when nothing was delivered).
///
fn deliver_message(inner: &mut EventManagerInner) -> Option<Message> {
    // SAFETY: `inner` is a function-local event manager that is not shared with any other context,
    // and the caller holds no reference to the process manager.
    match unsafe { inner.try_wait(test_tid(), test_pid(), usize::MAX, usize::MAX, usize::MAX) } {
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
/// Verifies that scheduling events are delivered in FIFO order by event sequence number: a process's
/// creation event, enqueued before its termination event, is delivered first. This per-process
/// creation-before-termination invariant is the cross-event ordering guarantee preserved while
/// making delivery starvation-free.
///
fn test_scheduling_events_delivered_fifo_creation_before_termination() -> bool {
    let mut inner: EventManagerInner = make_inner();

    // A process is always created before it terminates, so the creation event is enqueued first and
    // must be delivered first.
    push_creation(&mut inner, test_pid());
    push_termination(&mut inner, test_pid());

    let first: Option<MessageType> = deliver_once(&mut inner);
    if first != Some(MessageType::ProcessCreationEvent) {
        error!("expected the creation event to be delivered first, got {:?}", first);
        return false;
    }

    let second: Option<MessageType> = deliver_once(&mut inner);
    if second != Some(MessageType::ProcessTerminationEvent) {
        error!("expected the termination event to be delivered second, got {:?}", second);
        return false;
    }

    if !inner.pending_scheduling.is_empty() {
        error!("scheduling-event queue was not drained");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies starvation-free cross-class delivery: with a drainable event pending in more than one
/// class (an interrupt plus two scheduling events), repeated calls serve every class and deliver
/// every event within a bounded number of calls. No class is permanently skipped in favor of
/// another, and the scheduling events retain their creation-before-termination order.
///
fn test_cross_class_drainable_events_all_delivered() -> bool {
    let mut inner: EventManagerInner = make_inner();

    // One drainable event in the interrupt class and two in the scheduling class. Exceptions are
    // excluded here because they are re-queued until resumed (covered by
    // `test_all_three_event_classes_make_progress`).
    push_interrupt(&mut inner, 1);
    push_creation(&mut inner, test_pid());
    push_termination(&mut inner, test_pid());

    // Three queued events, one delivered per call: three calls drain them all.
    let mut delivered: [Option<MessageType>; 3] = [None; 3];
    for slot in delivered.iter_mut() {
        *slot = deliver_once(&mut inner);
    }

    // Every class with a pending event is served; no class is skipped.
    if !delivered.contains(&Some(MessageType::Interrupt)) {
        error!("interrupt event was never delivered: {:?}", delivered);
        return false;
    }

    let creation_index: Option<usize> = delivered
        .iter()
        .position(|m| *m == Some(MessageType::ProcessCreationEvent));
    let termination_index: Option<usize> = delivered
        .iter()
        .position(|m| *m == Some(MessageType::ProcessTerminationEvent));
    match (creation_index, termination_index) {
        (Some(creation), Some(termination)) => {
            if creation >= termination {
                error!("creation must be delivered before termination, got {:?}", delivered);
                return false;
            }
        },
        _ => {
            error!("both scheduling events must be delivered, got {:?}", delivered);
            return false;
        },
    }

    // All queues are now drained.
    if !inner.pending_interrupts[1].is_empty() || !inner.pending_scheduling.is_empty() {
        error!("event queues were not drained: {:?}", delivered);
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that all three event classes make progress when each has a pending event, including the
/// re-queuing exception class. Once the exception is observed it is resumed (removed) so that it
/// does not crowd out the remaining classes, demonstrating that every class is eventually served.
///
fn test_all_three_event_classes_make_progress() -> bool {
    const INTERRUPT_BIT: usize = 1;
    const EXCEPTION_BIT: usize = 2;

    let mut inner: EventManagerInner = make_inner();
    push_interrupt(&mut inner, INTERRUPT_BIT);
    push_exception(&mut inner, EXCEPTION_BIT, test_pid(), test_tid());
    push_creation(&mut inner, test_pid());

    // Three classes, one pending event each. Deliver three times; once the (re-queued) exception is
    // observed, resume it so it does not crowd out the remaining classes.
    let mut interrupt_seen: bool = false;
    let mut exception_seen: bool = false;
    let mut creation_seen: bool = false;
    for _ in 0..3 {
        match deliver_once(&mut inner) {
            Some(MessageType::Interrupt) => interrupt_seen = true,
            Some(MessageType::Exception) => {
                exception_seen = true;
                resume_exception(&mut inner, EXCEPTION_BIT);
            },
            Some(MessageType::ProcessCreationEvent) => creation_seen = true,
            other => {
                error!("unexpected delivery while draining all classes: {:?}", other);
                return false;
            },
        }
    }

    if !(interrupt_seen && exception_seen && creation_seen) {
        error!(
            "not every class made progress (interrupt={}, exception={}, creation={})",
            interrupt_seen, exception_seen, creation_seen
        );
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies that a long-standing scheduling event is not starved by continuous interrupt load: an
/// interrupt is enqueued before every delivery, yet the scheduling event that was pending from the
/// start is still delivered within a bounded number of calls.
///
fn test_standing_scheduling_event_not_starved_under_interrupt_load() -> bool {
    let mut inner: EventManagerInner = make_inner();

    // A long-standing scheduling event that must eventually be delivered.
    push_creation(&mut inner, test_pid());

    // Two full rotations across the event classes is a generous, implementation-agnostic budget
    // within which the standing event must be served.
    let max_deliveries: usize = 2 * EventManagerInner::NUMBER_EVENTS;

    let mut creation_seen: bool = false;
    for _ in 0..max_deliveries {
        // Offer continuous interrupt load alongside the standing scheduling event.
        push_interrupt(&mut inner, 1);
        if deliver_once(&mut inner) == Some(MessageType::ProcessCreationEvent) {
            creation_seen = true;
            break;
        }
    }

    if !creation_seen {
        error!("standing scheduling event was starved under interrupt load");
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
    let max_deliveries: usize = 4 * EventManagerInner::NUMBER_EVENTS;

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

    let max_deliveries: usize = 4 * EventManagerInner::NUMBER_EVENTS;

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
    passed &= run_test!(test_scheduling_events_delivered_fifo_creation_before_termination);
    passed &= run_test!(test_cross_class_drainable_events_all_delivered);
    passed &= run_test!(test_all_three_event_classes_make_progress);
    passed &= run_test!(test_standing_scheduling_event_not_starved_under_interrupt_load);
    passed &= run_test!(test_smallest_pending_front_selects_oldest_sequence);
    passed &= run_test!(test_interrupt_delivery_is_fifo_by_sequence);
    passed &= run_test!(test_oldest_interrupt_not_starved_by_low_bit_load);
    passed &= run_test!(test_exception_delivery_is_fifo_by_sequence);
    passed &= run_test!(test_oldest_exception_not_starved_by_low_bit_load);
    passed &= run_test!(test_delivery_orders_by_sequence_across_id_wrap);
    passed
}
