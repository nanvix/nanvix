// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ipc::DeliverySequence,
    pm::process::{
        LifecycleCreationReservation,
        LifecycleTerminationCredit,
    },
};
use ::alloc::collections::VecDeque;
use ::core::mem;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    event::{
        ProcessCreationInfo,
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
};

::static_assert::assert_eq!(mem::size_of::<ProcessTerminationInfo>() <= Message::PAYLOAD_SIZE);
::static_assert::assert_eq!(mem::size_of::<ProcessCreationInfo>() <= Message::PAYLOAD_SIZE);

//==================================================================================================
// Lifecycle Notification
//==================================================================================================

/// Lifecycle record stored in the ordered delivery broker.
enum LifecycleNotification {
    /// Process-creation record.
    Creation(ProcessCreationInfo),
    /// Process-termination record.
    Termination(ProcessTerminationInfo),
}

impl LifecycleNotification {
    ///
    /// # Description
    ///
    /// Serializes this lifecycle record into a message addressed to a process.
    ///
    /// # Parameters
    ///
    /// - `owner`: Identifier of the process that owns the lifecycle notification.
    ///
    /// # Returns
    ///
    /// The serialized lifecycle message.
    ///
    fn into_message(self, owner: ProcessIdentifier) -> Message {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        let message_type: MessageType = match self {
            Self::Creation(info) => {
                let info_bytes = info.to_ne_bytes();
                payload[0..info_bytes.len()].copy_from_slice(&info_bytes);
                MessageType::ProcessCreationEvent
            },
            Self::Termination(info) => {
                let info_bytes = info.to_ne_bytes();
                payload[0..info_bytes.len()].copy_from_slice(&info_bytes);
                MessageType::ProcessTerminationEvent
            },
        };

        Message {
            source: MessageSender::KERNEL,
            destination: MessageReceiver::new(owner, ThreadIdentifier::NONE),
            message_type,
            payload,
            ..Message::default()
        }
    }
}

//==================================================================================================
// Delivery Broker
//==================================================================================================

///
/// # Description
///
/// Orders the delivery domain shared by local IPC and process lifecycle notifications: it reserves
/// lifecycle capacity, allocates the sequence number stamped on every committed item, holds records
/// in production-sequence order, and tracks whether the lifecycle owner needs a deferred wakeup.
///
pub(super) struct DeliveryBroker {
    /// Sequence number to allocate to the next committed item.
    next_sequence: Option<DeliverySequence>,
    /// Lifecycle records in production-sequence order.
    lifecycle: VecDeque<(DeliverySequence, LifecycleNotification)>,
    /// Creation reservations that have not yet been committed or canceled.
    pending_creations: usize,
    /// Capacity credits reserved for future terminations of live processes.
    termination_credits: usize,
    /// Does the lifecycle owner need a deferred wakeup attempt?
    lifecycle_wakeup_pending: bool,
}

impl Default for DeliveryBroker {
    fn default() -> Self {
        Self {
            next_sequence: Some(DeliverySequence::default()),
            lifecycle: VecDeque::new(),
            pending_creations: 0,
            termination_credits: 0,
            lifecycle_wakeup_pending: false,
        }
    }
}

impl DeliveryBroker {
    /// Maximum lifecycle capacity consumed by buffered records, pending creations, and termination
    /// credits. A pending creation consumes two slots until it becomes one buffered creation and one
    /// termination credit.
    const LIFECYCLE_CAPACITY: usize = 2 * ::config::kernel::MAX_PROCESSES;

    ///
    /// # Description
    ///
    /// Allocates a sequence number for an item that is about to commit to kernel storage.
    ///
    /// # Returns
    ///
    /// The allocated sequence number.
    ///
    /// # Panics
    ///
    /// This function panics if the delivery sequence is exhausted. Continuing after exhaustion
    /// would silently reorder delivery-domain items.
    ///
    pub(super) fn allocate_sequence(&mut self) -> DeliverySequence {
        match self.try_allocate_sequence() {
            Some(sequence) => sequence,
            // Reaching this branch would require allocating all 2^64 sequence numbers first,
            // which is unlikely to happen during the kernel's lifetime.
            None => unreachable!("delivery sequence exhausted"),
        }
    }

    /// Attempts to allocate the next delivery sequence without applying the fatal exhaustion
    /// policy used by [`Self::allocate_sequence`].
    fn try_allocate_sequence(&mut self) -> Option<DeliverySequence> {
        let sequence: DeliverySequence = self.next_sequence?;
        self.next_sequence = sequence.checked_next();
        Some(sequence)
    }

    /// Sets the next sequence number for an in-kernel boundary test.
    #[cfg(feature = "test")]
    fn set_next_sequence(&mut self, sequence: DeliverySequence) {
        self.next_sequence = Some(sequence);
    }

    /// Reports whether the delivery sequence is exhausted.
    #[cfg(feature = "test")]
    fn sequence_is_exhausted(&self) -> bool {
        self.next_sequence.is_none()
    }

    ///
    /// # Description
    ///
    /// Reports the amount of lifecycle capacity currently committed or reserved.
    ///
    /// # Returns
    ///
    /// The number of lifecycle slots committed or reserved, or [`None`] if the accounting
    /// overflows.
    ///
    fn capacity_in_use(&self) -> Option<usize> {
        self.pending_creations
            .checked_mul(2)?
            .checked_add(self.termination_credits)?
            .checked_add(self.lifecycle.len())
    }

    /// Reports whether the used lifecycle capacity leaves room for a creation reservation.
    fn creation_capacity_available(capacity_in_use: usize) -> bool {
        matches!(
            capacity_in_use.checked_add(2),
            Some(required_capacity) if required_capacity <= Self::LIFECYCLE_CAPACITY
        )
    }

    ///
    /// # Description
    ///
    /// Reserves capacity for a process-creation record and the future termination of that process.
    ///
    /// # Returns
    ///
    /// Upon success, a creation reservation is returned. Otherwise, an error is returned instead.
    ///
    pub(super) fn try_reserve_creation(&mut self) -> Result<LifecycleCreationReservation, Error> {
        let capacity_in_use: usize = match self.capacity_in_use() {
            Some(capacity) if Self::creation_capacity_available(capacity) => capacity,
            _ => {
                let reason: &str = "process lifecycle queue cannot reserve a creation record";
                error!("{reason}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };
        let required_capacity: usize = capacity_in_use + 2;

        // Reserve the backing storage needed by all outstanding credits. Commits therefore cannot
        // allocate or fail after process-manager state starts changing.
        let additional_capacity: usize = required_capacity - self.lifecycle.len();
        if self
            .lifecycle
            .try_reserve_exact(additional_capacity)
            .is_err()
        {
            let reason: &str = "process lifecycle queue allocation failed";
            error!("{reason}");
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }

        self.pending_creations += 1;
        Ok(LifecycleCreationReservation::new())
    }

    ///
    /// # Description
    ///
    /// Cancels a process-creation reservation.
    ///
    /// # Parameters
    ///
    /// - `_reservation`: Reservation to cancel.
    ///
    pub(super) fn cancel_creation(&mut self, _reservation: LifecycleCreationReservation) {
        self.pending_creations -= 1;
    }

    ///
    /// # Description
    ///
    /// Commits a process-creation record in the ordered delivery domain.
    ///
    /// # Parameters
    ///
    /// - `_reservation`: Reservation that supplies capacity for this creation and its termination.
    /// - `info`: The process-creation record to queue.
    ///
    /// # Returns
    ///
    /// A capacity credit for the future termination of the created process.
    ///
    pub(super) fn commit_creation(
        &mut self,
        _reservation: LifecycleCreationReservation,
        info: ProcessCreationInfo,
    ) -> LifecycleTerminationCredit {
        self.pending_creations -= 1;
        self.termination_credits += 1;
        let sequence: DeliverySequence = self.allocate_sequence();
        self.lifecycle
            .push_back((sequence, LifecycleNotification::Creation(info)));
        self.lifecycle_wakeup_pending = true;
        LifecycleTerminationCredit::new()
    }

    ///
    /// # Description
    ///
    /// Commits a process-termination record in the ordered delivery domain.
    ///
    /// # Parameters
    ///
    /// - `_credit`: Capacity credit reserved when the process was created.
    /// - `info`: The process-termination record to queue.
    ///
    pub(super) fn commit_termination(
        &mut self,
        _credit: LifecycleTerminationCredit,
        info: ProcessTerminationInfo,
    ) {
        self.termination_credits -= 1;
        let sequence: DeliverySequence = self.allocate_sequence();
        self.lifecycle
            .push_back((sequence, LifecycleNotification::Termination(info)));
        self.lifecycle_wakeup_pending = true;
    }

    /// Releases a termination credit when the process terminates without producing a lifecycle
    /// record.
    pub(super) fn release_termination(&mut self, _credit: LifecycleTerminationCredit) {
        self.termination_credits -= 1;
    }

    ///
    /// # Description
    ///
    /// Returns the sequence number of the oldest buffered lifecycle record.
    ///
    /// # Returns
    ///
    /// The sequence number of the oldest lifecycle record, or [`None`] if none is buffered.
    ///
    fn lifecycle_sequence(&self) -> Option<DeliverySequence> {
        self.lifecycle.front().map(|(sequence, _)| *sequence)
    }

    ///
    /// # Description
    ///
    /// Reports whether a buffered lifecycle record should be selected ahead of the eligible mailbox
    /// head, that is, whether it is older than the eligible mailbox message.
    ///
    /// # Parameters
    ///
    /// - `mailbox_sequence`: Sequence number of the eligible mailbox head, if any.
    /// - `lifecycle_eligible`: Whether the receiver owns the lifecycle scheduling-event class.
    ///
    /// # Returns
    ///
    /// `true` if a lifecycle record precedes the eligible mailbox head, otherwise `false`.
    ///
    pub(super) fn lifecycle_precedes(
        &self,
        mailbox_sequence: Option<DeliverySequence>,
        lifecycle_eligible: bool,
    ) -> bool {
        if !lifecycle_eligible {
            return false;
        }

        match (self.lifecycle_sequence(), mailbox_sequence) {
            (Some(lifecycle), Some(mailbox)) => lifecycle < mailbox,
            (Some(_), None) => true,
            _ => false,
        }
    }

    ///
    /// # Description
    ///
    /// Removes the oldest buffered lifecycle record and serializes it into a message addressed to a
    /// process.
    ///
    /// # Parameters
    ///
    /// - `owner`: Identifier of the process that owns the lifecycle scheduling-event class.
    ///
    /// # Returns
    ///
    /// The serialized lifecycle message, or [`None`] if no lifecycle record is buffered.
    ///
    pub(super) fn pop_lifecycle(&mut self, owner: ProcessIdentifier) -> Option<Message> {
        let (_sequence, notification): (DeliverySequence, LifecycleNotification) =
            self.lifecycle.pop_front()?;
        if self.lifecycle.is_empty() {
            self.lifecycle_wakeup_pending = false;
        }
        Some(notification.into_message(owner))
    }

    ///
    /// # Description
    ///
    /// Reports whether at least one lifecycle record is buffered.
    ///
    /// # Returns
    ///
    /// `true` if a lifecycle record is buffered, otherwise `false`.
    ///
    pub(super) fn has_lifecycle(&self) -> bool {
        !self.lifecycle.is_empty()
    }

    ///
    /// # Description
    ///
    /// Marks buffered lifecycle records for a deferred owner wakeup, so a later drain retries the
    /// notification.
    ///
    pub(super) fn request_lifecycle_wakeup(&mut self) {
        if self.has_lifecycle() {
            self.lifecycle_wakeup_pending = true;
        }
    }

    ///
    /// # Description
    ///
    /// Takes the pending lifecycle-owner wakeup request, clearing it in the process.
    ///
    /// # Returns
    ///
    /// `true` if a wakeup request was pending, otherwise `false`.
    ///
    pub(super) fn take_lifecycle_wakeup_request(&mut self) -> bool {
        mem::take(&mut self.lifecycle_wakeup_pending)
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(feature = "test")]
mod test {
    use super::DeliveryBroker;
    use crate::{
        ipc::DeliverySequence,
        pm::process::{
            LifecycleCreationReservation,
            LifecycleTerminationCredit,
        },
    };
    use ::sys::{
        event::{
            ProcessCreationInfo,
            ProcessRole,
            ProcessTerminationInfo,
        },
        pm::ProcessIdentifier,
        ExitStatus,
    };

    /// Verifies that delivery sequence exhaustion is detected instead of wrapping.
    fn test_delivery_sequence_exhaustion_is_detected() -> bool {
        let mut broker: DeliveryBroker = DeliveryBroker::default();
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

    /// Verifies lifecycle capacity across reservation, commit, dequeue, and reuse transitions.
    fn test_lifecycle_capacity_accounting_is_stateful() -> bool {
        let mut broker: DeliveryBroker = DeliveryBroker::default();
        let lifecycle_capacity: usize = 2 * ::config::kernel::MAX_PROCESSES;
        if !DeliveryBroker::creation_capacity_available(lifecycle_capacity - 2) {
            error!("lifecycle creation capacity exhausted too early");
            return false;
        }
        if DeliveryBroker::creation_capacity_available(lifecycle_capacity - 1) {
            error!("lifecycle capacity exceeded its configured bound");
            return false;
        }

        broker.pending_creations = ::config::kernel::MAX_PROCESSES;
        match broker.try_reserve_creation() {
            Err(error) if error.code == ::sys::error::ErrorCode::OutOfMemory => {},
            Err(error) => {
                error!("capacity rejection returned the wrong error (error={error:?})");
                return false;
            },
            Ok(_) => {
                error!("lifecycle broker accepted a reservation beyond capacity");
                return false;
            },
        }
        if broker.pending_creations != ::config::kernel::MAX_PROCESSES {
            error!("rejected lifecycle reservation changed broker accounting");
            return false;
        }
        broker.pending_creations = 0;

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
        if broker.pop_lifecycle(ProcessIdentifier::KERNEL).is_none()
            || broker.capacity_in_use() != Some(1)
        {
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
        if broker.pop_lifecycle(ProcessIdentifier::KERNEL).is_none()
            || broker.capacity_in_use() != Some(0)
        {
            error!("termination dequeue did not release lifecycle capacity");
            return false;
        }

        true
    }

    /// Runs all delivery broker in-kernel tests.
    pub(super) fn test() -> bool {
        let mut passed: bool = true;
        passed &= run_test!(test_delivery_sequence_exhaustion_is_detected);
        passed &= run_test!(test_lifecycle_capacity_accounting_is_stateful);
        passed
    }
}

///
/// # Description
///
/// Runs the delivery broker in-kernel tests.
///
/// # Returns
///
/// `true` if every test passed, `false` otherwise.
///
#[cfg(feature = "test")]
pub(super) fn test() -> bool {
    test::test()
}
