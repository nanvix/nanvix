// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    DeliveryBroker,
    DeliverySequence,
    LIFECYCLE_CAPACITY,
    THREAD_TERMINATION_RESERVATIONS,
};
use crate::pm::process::{
    LifecycleCreationReservation,
    LifecycleTerminationCredit,
    ThreadLifecycleReservation,
    ThreadLifecycleTerminationCredit,
};
use ::sys::{
    event::{
        ProcessCreationInfo,
        ProcessRole,
        ProcessTerminationInfo,
        ThreadTerminationInfo,
    },
    ipc::MessageType,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    ExitStatus,
};

///
/// # Description
///
/// Creates a lifecycle broker for an in-kernel test.
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

///
/// # Description
///
/// Selects and commits the lifecycle record at the head of a broker queue.
///
/// # Parameters
///
/// - `broker`: Delivery broker whose head record should be committed.
///
/// # Returns
///
/// `true` if a lifecycle record was present and committed, otherwise `false`.
///
fn commit_lifecycle(broker: &mut DeliveryBroker) -> bool {
    let Some((sequence, _message)) = broker.peek_lifecycle(ProcessIdentifier::KERNEL) else {
        return false;
    };
    broker.commit_lifecycle(sequence);
    true
}

///
/// # Description
///
/// Verifies that delivery sequence exhaustion is detected instead of wrapping.
///
/// # Returns
///
/// `true` if terminal sequence allocation is accepted exactly once, otherwise `false`.
///
fn test_delivery_sequence_exhaustion_is_detected() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    broker.set_next_sequence(DeliverySequence::new(u64::MAX));
    if broker.try_allocate_sequence() != Some(DeliverySequence::new(u64::MAX)) {
        error!("delivery broker did not allocate the terminal sequence value");
        return false;
    }
    if broker.try_allocate_sequence().is_some() || !broker.sequence_is_exhausted() {
        error!("delivery broker did not reject allocation after u64::MAX");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies lifecycle reservation ownership across cancel, commit, dequeue, and reuse.
///
/// # Returns
///
/// `true` if every ownership transition releases the expected reservations, otherwise `false`.
///
fn test_lifecycle_reservation_ownership_is_stateful() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };

    let first: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("first lifecycle reservation failed (error={error:?})");
            return false;
        },
    };
    let second: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("second lifecycle reservation failed (error={error:?})");
            return false;
        },
    };
    if broker.capacity_in_use() != Some(4) {
        error!("outstanding lifecycle reservations were accounted incorrectly");
        return false;
    }
    broker.cancel_creation(first);
    broker.cancel_creation(second);
    if broker.capacity_in_use() != Some(0) {
        error!("canceling lifecycle reservations did not release capacity");
        return false;
    }

    let reservation: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("released lifecycle capacity was not reusable (error={error:?})");
            return false;
        },
    };
    let pid: ProcessIdentifier = ProcessIdentifier::from(1);
    let credit: LifecycleTerminationCredit = broker.commit_creation(
        reservation,
        ProcessCreationInfo::new(pid, ProcessIdentifier::KERNEL, ProcessRole::User),
    );
    if broker.capacity_in_use() != Some(2) {
        error!("creation commit did not preserve its termination capacity");
        return false;
    }
    if !commit_lifecycle(&mut broker) || broker.capacity_in_use() != Some(1) {
        error!("creation dequeue did not retain exactly one termination credit");
        return false;
    }

    broker.commit_termination(
        credit,
        ProcessTerminationInfo::new(
            pid,
            ExitStatus::ok(),
            ProcessIdentifier::KERNEL,
            ProcessRole::User,
        ),
    );
    if broker.capacity_in_use() != Some(1) {
        error!("termination commit changed reserved lifecycle capacity");
        return false;
    }
    if !commit_lifecycle(&mut broker) || broker.capacity_in_use() != Some(0) {
        error!("termination dequeue did not release lifecycle capacity");
        return false;
    }

    true
}

///
/// # Description
///
/// Verifies thread lifecycle ownership across cancellation, commit, delivery, and reuse.
///
/// # Returns
///
/// `true` if every ownership transition preserves the expected capacity accounting, otherwise
/// `false`.
///
fn test_thread_lifecycle_reservation_ownership_is_stateful() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };

    let cancelled: ThreadLifecycleReservation = match broker.try_reserve_thread_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("thread lifecycle reservation failed (error={error:?})");
            return false;
        },
    };
    broker.cancel_thread_creation(cancelled);
    if broker.capacity_in_use() != Some(0) {
        error!("canceling a thread lifecycle reservation did not release capacity");
        return false;
    }

    let reservation: ThreadLifecycleReservation = match broker.try_reserve_thread_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("released thread lifecycle capacity was not reusable (error={error:?})");
            return false;
        },
    };
    let credit: ThreadLifecycleTerminationCredit = broker.commit_thread_creation(reservation);
    if broker.capacity_in_use() != Some(1) || broker.has_lifecycle() {
        error!("thread creation commit changed reservation ownership or queued a record");
        return false;
    }

    let info: ThreadTerminationInfo = ThreadTerminationInfo::new(
        ProcessIdentifier::from(3),
        ThreadIdentifier::from(7),
        ExitStatus::from(11_u32),
    );
    broker.commit_thread_termination(credit, info);
    let Some((_sequence, message)) = broker.peek_lifecycle(ProcessIdentifier::KERNEL) else {
        error!("committed thread termination record was not selectable");
        return false;
    };
    let info_bytes = info.to_ne_bytes();
    if message.message_type != MessageType::ThreadTerminationEvent
        || message.payload[..info_bytes.len()] != info_bytes
    {
        error!("thread termination record was serialized incorrectly");
        return false;
    }
    if !commit_lifecycle(&mut broker) || broker.capacity_in_use() != Some(0) {
        error!("thread termination dequeue did not release lifecycle capacity");
        return false;
    }

    true
}

/// Verifies a failed termination reservation rolls back the creation slot acquired first.
fn test_termination_reservation_failure_rolls_back_creation() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    for _ in 0..::config::kernel::MAX_PROCESSES {
        if !broker.termination_reservations.try_reserve() {
            error!("termination pool exhausted too early");
            return false;
        }
    }
    if broker.creation_reservations.in_use() != 0
        || broker.termination_reservations.in_use() != ::config::kernel::MAX_PROCESSES
    {
        error!("termination-only pressure produced incorrect reservation accounting");
        return false;
    }

    match broker.try_reserve_creation() {
        Err(error) if error.code == ::sys::error::ErrorCode::OutOfMemory => {},
        Err(error) => {
            error!("termination-pool rejection returned wrong error (error={error:?})");
            return false;
        },
        Ok(_) => {
            error!("creation succeeded despite an exhausted termination pool");
            return false;
        },
    }
    if broker.creation_reservations.in_use() != 0
        || broker.termination_reservations.in_use() != ::config::kernel::MAX_PROCESSES
    {
        error!("failed termination reservation leaked its provisional creation slot");
        return false;
    }

    if broker.termination_reservations.release_all() != ::config::kernel::MAX_PROCESSES {
        error!("termination-pool fixture released an unexpected reservation count");
        return false;
    }
    broker.capacity_in_use() == Some(0)
}

///
/// # Description
///
/// Verifies deterministic backpressure and recovery while the lifecycle consumer is stalled.
/// Every admitted process immediately terminates, proving that termination enqueue remains
/// infallible even as all preallocated reservation pools and the queue become full. The test
/// then drains and refills part of the queue to exercise ring wraparound before releasing all
/// capacity through transactional dequeue.
///
/// # Returns
///
/// `true` if capacity backpressures, wraps, drains, and becomes reusable, otherwise `false`.
///
fn test_stalled_lifecycle_consumer_backpressures_and_recovers() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let pid: ProcessIdentifier = ProcessIdentifier::from(1);
    let creation: ProcessCreationInfo =
        ProcessCreationInfo::new(pid, ProcessIdentifier::KERNEL, ProcessRole::User);
    let termination: ProcessTerminationInfo = ProcessTerminationInfo::new(
        pid,
        ExitStatus::ok(),
        ProcessIdentifier::KERNEL,
        ProcessRole::User,
    );

    for _ in 0..::config::kernel::MAX_PROCESSES {
        let reservation: LifecycleCreationReservation = match broker.try_reserve_creation() {
            Ok(reservation) => reservation,
            Err(error) => {
                error!("lifecycle capacity exhausted too early (error={error:?})");
                return false;
            },
        };
        let credit: LifecycleTerminationCredit = broker.commit_creation(reservation, creation);
        broker.commit_termination(credit, termination);
    }
    let thread_termination: ThreadTerminationInfo =
        ThreadTerminationInfo::new(pid, ThreadIdentifier::from(1), ExitStatus::ok());
    for _ in 0..THREAD_TERMINATION_RESERVATIONS {
        let reservation: ThreadLifecycleReservation = match broker.try_reserve_thread_creation() {
            Ok(reservation) => reservation,
            Err(error) => {
                error!("thread lifecycle capacity exhausted too early (error={error:?})");
                return false;
            },
        };
        let credit: ThreadLifecycleTerminationCredit = broker.commit_thread_creation(reservation);
        broker.commit_thread_termination(credit, thread_termination);
    }

    if broker.capacity_in_use() != Some(LIFECYCLE_CAPACITY)
        || broker.lifecycle.len() != LIFECYCLE_CAPACITY
    {
        error!("stalled lifecycle consumer did not retain all reserved records");
        return false;
    }
    match broker.try_reserve_creation() {
        Err(error) if error.code == ::sys::error::ErrorCode::OutOfMemory => {},
        Err(error) => {
            error!("lifecycle backpressure returned the wrong error (error={error:?})");
            return false;
        },
        Ok(_) => {
            error!("lifecycle creation exceeded preallocated capacity");
            return false;
        },
    }
    if broker.capacity_in_use() != Some(LIFECYCLE_CAPACITY) {
        error!("rejected lifecycle reservation changed pool accounting");
        return false;
    }

    let refill_processes: usize = ::config::kernel::MAX_PROCESSES / 2;
    for _ in 0..2 * refill_processes {
        if !commit_lifecycle(&mut broker) {
            error!("lifecycle queue drained before the requested partial dequeue");
            return false;
        }
    }
    for _ in 0..refill_processes {
        let reservation: LifecycleCreationReservation = match broker.try_reserve_creation() {
            Ok(reservation) => reservation,
            Err(error) => {
                error!("drained lifecycle capacity was not reusable (error={error:?})");
                return false;
            },
        };
        let credit: LifecycleTerminationCredit = broker.commit_creation(reservation, creation);
        broker.commit_termination(credit, termination);
    }
    if broker.capacity_in_use() != Some(LIFECYCLE_CAPACITY) {
        error!("wrapped lifecycle queue did not return to full capacity");
        return false;
    }

    while commit_lifecycle(&mut broker) {}
    if broker.capacity_in_use() != Some(0) || broker.has_lifecycle() {
        error!("transactional lifecycle drain did not release every reservation");
        return false;
    }
    let recovered: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("lifecycle creation did not recover after drain (error={error:?})");
            return false;
        },
    };
    broker.cancel_creation(recovered);
    broker.capacity_in_use() == Some(0)
}

///
/// # Description
///
/// Verifies terminal disposal releases queued and externally retained reservations separately.
///
/// # Returns
///
/// `true` if disposal reports and releases all queued and retained reservations, otherwise
/// `false`.
///
fn test_lifecycle_shutdown_disposal_accounts_undelivered_state() -> bool {
    let Some(mut broker) = new_broker() else {
        return false;
    };
    let pid: ProcessIdentifier = ProcessIdentifier::from(1);
    let creation: ProcessCreationInfo =
        ProcessCreationInfo::new(pid, ProcessIdentifier::KERNEL, ProcessRole::User);
    let termination: ProcessTerminationInfo = ProcessTerminationInfo::new(
        pid,
        ExitStatus::ok(),
        ProcessIdentifier::KERNEL,
        ProcessRole::User,
    );

    let live_reservation: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("failed to reserve live-process fixture (error={error:?})");
            return false;
        },
    };
    let _live_termination_credit: LifecycleTerminationCredit =
        broker.commit_creation(live_reservation, creation);

    let terminated_reservation: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("failed to reserve terminated-process fixture (error={error:?})");
            return false;
        },
    };
    let terminated_credit: LifecycleTerminationCredit =
        broker.commit_creation(terminated_reservation, creation);
    broker.commit_termination(terminated_credit, termination);

    let _pending_reservation: LifecycleCreationReservation = match broker.try_reserve_creation() {
        Ok(reservation) => reservation,
        Err(error) => {
            error!("failed to reserve pending-process fixture (error={error:?})");
            return false;
        },
    };
    let live_thread_reservation: ThreadLifecycleReservation =
        match broker.try_reserve_thread_creation() {
            Ok(reservation) => reservation,
            Err(error) => {
                error!("failed to reserve live-thread fixture (error={error:?})");
                return false;
            },
        };
    let _live_thread_credit: ThreadLifecycleTerminationCredit =
        broker.commit_thread_creation(live_thread_reservation);
    let terminated_thread_reservation: ThreadLifecycleReservation =
        match broker.try_reserve_thread_creation() {
            Ok(reservation) => reservation,
            Err(error) => {
                error!("failed to reserve terminated-thread fixture (error={error:?})");
                return false;
            },
        };
    let terminated_thread_credit: ThreadLifecycleTerminationCredit =
        broker.commit_thread_creation(terminated_thread_reservation);
    broker.commit_thread_termination(
        terminated_thread_credit,
        ThreadTerminationInfo::new(pid, ThreadIdentifier::from(1), ExitStatus::ok()),
    );
    let _pending_thread_reservation: ThreadLifecycleReservation =
        match broker.try_reserve_thread_creation() {
            Ok(reservation) => reservation,
            Err(error) => {
                error!("failed to reserve pending-thread fixture (error={error:?})");
                return false;
            },
        };
    let disposal = broker.dispose();
    if disposal.undelivered_records != 4 || disposal.retained_reservations != 5 {
        error!(
            "lifecycle shutdown disposal reported incorrect accounting (undelivered={}, \
             retained={})",
            disposal.undelivered_records, disposal.retained_reservations
        );
        return false;
    }
    if broker.capacity_in_use() != Some(0)
        || broker.has_lifecycle()
        || broker.take_lifecycle_wakeup_request()
    {
        error!("lifecycle shutdown disposal retained broker state");
        return false;
    }
    true
}

///
/// # Description
///
/// Runs all delivery broker in-kernel tests.
///
/// # Returns
///
/// `true` if every delivery broker test passes, otherwise `false`.
///
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= run_test!(test_delivery_sequence_exhaustion_is_detected);
    passed &= run_test!(test_lifecycle_reservation_ownership_is_stateful);
    passed &= run_test!(test_thread_lifecycle_reservation_ownership_is_stateful);
    passed &= run_test!(test_termination_reservation_failure_rolls_back_creation);
    passed &= run_test!(test_stalled_lifecycle_consumer_backpressures_and_recovers);
    passed &= run_test!(test_lifecycle_shutdown_disposal_accounts_undelivered_state);
    passed
}
