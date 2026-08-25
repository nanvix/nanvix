// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod delivery_sequence;

//==================================================================================================
// Imports
//==================================================================================================

pub(crate) use self::delivery_sequence::DeliverySequence;
use crate::{
    mm::{
        try_box,
        try_vec_with_capacity,
    },
    pm::process::{
        LifecycleCreationCredit,
        LifecycleCreationReservation,
        LifecycleTerminationCredit,
        ThreadLifecycleReservation,
        ThreadLifecycleTerminationCredit,
    },
};
use ::alloc::{
    boxed::Box,
    vec::Vec,
};
use ::core::mem;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    event::{
        ProcessCreationInfo,
        ProcessTerminationInfo,
        ThreadTerminationInfo,
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
::static_assert::assert_eq!(mem::size_of::<ThreadTerminationInfo>() <= Message::PAYLOAD_SIZE);

/// Number of process-creation reservations. This is the depth of the undelivered-creation buffer:
/// it bounds how many processes may be created while the lifecycle consumer is stalled, not how many
/// may be live.
const PROCESS_CREATION_RESERVATIONS: usize = ::config::kernel::MAX_PROCESSES;

/// Number of process-termination reservations. This must be at least the maximum number of live
/// processes, because each one holds a credit from creation until its termination record is
/// delivered. The excess over that bound is undelivered-termination buffer depth.
const PROCESS_TERMINATION_RESERVATIONS: usize = ::config::kernel::MAX_PROCESSES;

/// Number of thread-termination reservations. The permanent bootstrap kernel thread cannot
/// terminate and therefore does not consume an undeliverable reservation.
const THREAD_TERMINATION_RESERVATIONS: usize = match ::config::kernel::MAX_THREADS.checked_sub(1) {
    Some(capacity) => capacity,
    None => panic!("thread lifecycle capacity underflow"),
};

/// Number of lifecycle records that may be queued. This covers every credit even though credits
/// retained by live processes and threads do not occupy queue slots.
const LIFECYCLE_CAPACITY: usize = {
    let process_capacity: usize =
        match PROCESS_CREATION_RESERVATIONS.checked_add(PROCESS_TERMINATION_RESERVATIONS) {
            Some(capacity) => capacity,
            None => panic!("process lifecycle capacity overflow"),
        };
    match process_capacity.checked_add(THREAD_TERMINATION_RESERVATIONS) {
        Some(capacity) => capacity,
        None => panic!("thread lifecycle capacity overflow"),
    }
};

//==================================================================================================
// Lifecycle Notification
//==================================================================================================

/// Lifecycle record stored in the ordered delivery broker.
enum LifecycleNotification {
    /// Process-creation record.
    Creation(ProcessCreationInfo, LifecycleCreationCredit),
    /// Process-termination record.
    Termination(ProcessTerminationInfo, LifecycleTerminationCredit),
    /// Thread-termination record.
    ThreadTermination(ThreadTerminationInfo, ThreadLifecycleTerminationCredit),
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
    fn to_message(&self, owner: ProcessIdentifier) -> Message {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0u8; Message::PAYLOAD_SIZE];
        let message_type: MessageType = match self {
            Self::Creation(info, _) => {
                let info_bytes = info.to_ne_bytes();
                payload[0..info_bytes.len()].copy_from_slice(&info_bytes);
                MessageType::ProcessCreationEvent
            },
            Self::Termination(info, _) => {
                let info_bytes = info.to_ne_bytes();
                payload[0..info_bytes.len()].copy_from_slice(&info_bytes);
                MessageType::ProcessTerminationEvent
            },
            Self::ThreadTermination(info, _) => {
                let info_bytes = info.to_ne_bytes();
                payload[0..info_bytes.len()].copy_from_slice(&info_bytes);
                MessageType::ThreadTerminationEvent
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
// Lifecycle Reservation Pool
//==================================================================================================

/// Fixed-capacity reservation pool embedded in the delivery broker. Reservations carry no identity
/// because the credit types are linear and constructible only through [`DeliveryBroker`], so a
/// credit can be neither forged nor released twice.
struct LifecycleReservationPool<const CAPACITY: usize> {
    /// Number of reserved slots.
    in_use: usize,
}

impl<const CAPACITY: usize> LifecycleReservationPool<CAPACITY> {
    ///
    /// # Description
    ///
    /// Creates a fully available reservation pool.
    ///
    /// # Returns
    ///
    /// A reservation pool with no reserved slots.
    ///
    const fn new() -> Self {
        Self { in_use: 0 }
    }

    ///
    /// # Description
    ///
    /// Reserves one slot when capacity is available.
    ///
    /// # Returns
    ///
    /// `true` if a slot was reserved, otherwise `false`.
    ///
    fn try_reserve(&mut self) -> bool {
        if self.in_use >= CAPACITY {
            return false;
        }
        self.in_use += 1;
        true
    }

    ///
    /// # Description
    ///
    /// Releases one reserved slot.
    ///
    /// # Panics
    ///
    /// This function panics if no slot is currently reserved.
    ///
    fn release(&mut self) {
        self.in_use = match self.in_use.checked_sub(1) {
            Some(in_use) => in_use,
            None => unreachable!("lifecycle reservation was released twice"),
        };
    }

    ///
    /// # Description
    ///
    /// Reports how many slots are reserved.
    ///
    /// # Returns
    ///
    /// The number of reserved slots.
    ///
    #[cfg(feature = "test")]
    fn in_use(&self) -> usize {
        self.in_use
    }

    ///
    /// # Description
    ///
    /// Releases every reservation.
    ///
    /// # Returns
    ///
    /// The number of reservations that were retained before the release.
    ///
    fn release_all(&mut self) -> usize {
        mem::take(&mut self.in_use)
    }
}

//==================================================================================================
// Lifecycle Queue
//==================================================================================================

/// Lifecycle record and its delivery sequence.
type LifecycleRecord = (DeliverySequence, LifecycleNotification);

/// Number of lifecycle records stored in each independently allocated queue chunk, derived so that
/// a chunk fills the kernel heap's largest slab.
const LIFECYCLE_QUEUE_CHUNK_CAPACITY: usize =
    ::config::kernel::MAX_SLAB_SIZE / mem::size_of::<Option<LifecycleRecord>>();

::static_assert::assert_eq!(LIFECYCLE_QUEUE_CHUNK_CAPACITY >= 1);

/// Number of queue chunks required to hold every lifecycle reservation.
const LIFECYCLE_QUEUE_CHUNKS: usize = LIFECYCLE_CAPACITY.div_ceil(LIFECYCLE_QUEUE_CHUNK_CAPACITY);

/// Slab-sized portion of the preallocated lifecycle queue.
struct LifecycleQueueChunk {
    /// Record slots owned by this chunk.
    records: [Option<LifecycleRecord>; LIFECYCLE_QUEUE_CHUNK_CAPACITY],
}

impl LifecycleQueueChunk {
    ///
    /// # Description
    ///
    /// Creates an empty lifecycle queue chunk.
    ///
    /// # Returns
    ///
    /// A chunk whose record slots are all vacant.
    ///
    fn new() -> Self {
        Self {
            records: [const { None }; LIFECYCLE_QUEUE_CHUNK_CAPACITY],
        }
    }
}

::static_assert::assert_eq!(
    mem::size_of::<LifecycleQueueChunk>() <= ::config::kernel::MAX_SLAB_SIZE
);
::static_assert::assert_eq!(
    LIFECYCLE_QUEUE_CHUNKS * mem::size_of::<Box<LifecycleQueueChunk>>()
        <= ::config::kernel::MAX_SLAB_SIZE
);

/// Fixed-capacity ring whose backing chunks are fully allocated during kernel initialization.
struct LifecycleQueue {
    /// Preallocated queue chunks. Each chunk is boxed separately so every allocation fits the
    /// kernel heap's maximum slab size.
    #[allow(clippy::vec_box)]
    chunks: Vec<Box<LifecycleQueueChunk>>,
    /// Logical index of the oldest record.
    head: usize,
    /// Number of records currently stored.
    len: usize,
}

impl LifecycleQueue {
    ///
    /// # Description
    ///
    /// Fallibly preallocates every chunk of an empty lifecycle queue.
    ///
    /// # Returns
    ///
    /// Upon success, an empty lifecycle queue with all backing chunks allocated. Otherwise, an
    /// error is returned instead.
    ///
    /// # Errors
    ///
    /// This function returns an error if any queue chunk cannot be allocated.
    ///
    fn new() -> Result<Self, Error> {
        let mut chunks: Vec<Box<LifecycleQueueChunk>> =
            try_vec_with_capacity(LIFECYCLE_QUEUE_CHUNKS)?;
        for _ in 0..LIFECYCLE_QUEUE_CHUNKS {
            chunks.push(try_box(LifecycleQueueChunk::new())?);
        }

        Ok(Self {
            chunks,
            head: 0,
            len: 0,
        })
    }

    ///
    /// # Description
    ///
    /// Reports how many records are buffered.
    ///
    /// # Returns
    ///
    /// The number of buffered records.
    ///
    #[cfg(feature = "test")]
    fn len(&self) -> usize {
        self.len
    }

    ///
    /// # Description
    ///
    /// Reports whether no records are buffered.
    ///
    /// # Returns
    ///
    /// `true` if the queue is empty, otherwise `false`.
    ///
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    ///
    /// # Description
    ///
    /// Returns the slot that backs a ring index.
    ///
    /// # Parameters
    ///
    /// - `index`: Ring index whose backing slot is resolved.
    ///
    /// # Returns
    ///
    /// A shared reference to the backing slot.
    ///
    fn slot(&self, index: usize) -> &Option<LifecycleRecord> {
        &self.chunks[index / LIFECYCLE_QUEUE_CHUNK_CAPACITY].records
            [index % LIFECYCLE_QUEUE_CHUNK_CAPACITY]
    }

    ///
    /// # Description
    ///
    /// Returns the slot that backs a ring index, for mutation.
    ///
    /// # Parameters
    ///
    /// - `index`: Ring index whose backing slot is resolved.
    ///
    /// # Returns
    ///
    /// A mutable reference to the backing slot.
    ///
    fn slot_mut(&mut self, index: usize) -> &mut Option<LifecycleRecord> {
        &mut self.chunks[index / LIFECYCLE_QUEUE_CHUNK_CAPACITY].records
            [index % LIFECYCLE_QUEUE_CHUNK_CAPACITY]
    }

    ///
    /// # Description
    ///
    /// Returns a shared reference to the oldest record.
    ///
    /// # Returns
    ///
    /// The oldest buffered record, or [`None`] if the queue is empty.
    ///
    /// # Panics
    ///
    /// This function panics if the queue is non-empty but its head slot is vacant, which indicates
    /// a broken ring invariant.
    ///
    fn front(&self) -> Option<&LifecycleRecord> {
        if self.is_empty() {
            return None;
        }
        match self.slot(self.head) {
            Some(record) => Some(record),
            None => unreachable!("lifecycle queue head is vacant"),
        }
    }

    ///
    /// # Description
    ///
    /// Appends a record without allocating.
    ///
    /// # Parameters
    ///
    /// - `record`: Record to append at the tail of the queue.
    ///
    /// # Panics
    ///
    /// This function panics if the preallocated queue is full or its tail slot is occupied, which
    /// indicates a broken ring invariant.
    ///
    fn push_back(&mut self, record: LifecycleRecord) {
        if self.len >= LIFECYCLE_CAPACITY {
            unreachable!("preallocated lifecycle queue is full");
        }

        let tail: usize = (self.head + self.len) % LIFECYCLE_CAPACITY;
        let slot: &mut Option<LifecycleRecord> = self.slot_mut(tail);
        if slot.is_some() {
            unreachable!("lifecycle queue tail is occupied");
        }
        *slot = Some(record);
        self.len += 1;
    }

    ///
    /// # Description
    ///
    /// Removes and returns the oldest record.
    ///
    /// # Returns
    ///
    /// The oldest buffered record, or [`None`] if the queue is empty.
    ///
    /// # Panics
    ///
    /// This function panics if the queue is non-empty but its head slot is vacant, which indicates
    /// a broken ring invariant.
    ///
    fn pop_front(&mut self) -> Option<LifecycleRecord> {
        if self.is_empty() {
            return None;
        }

        let head: usize = self.head;
        self.head = (self.head + 1) % LIFECYCLE_CAPACITY;
        self.len -= 1;
        match self.slot_mut(head).take() {
            Some(record) => Some(record),
            None => unreachable!("lifecycle queue head is vacant"),
        }
    }
}

//==================================================================================================
// Delivery Broker
//==================================================================================================

/// Accounting for lifecycle state disposed during terminal shutdown.
pub(super) struct LifecycleDisposal {
    /// Queued records discarded without being delivered.
    pub(super) undelivered_records: usize,
    /// Reservations still retained outside queued records.
    pub(super) retained_reservations: usize,
}

///
/// # Description
///
/// Orders the delivery domain shared by local IPC and lifecycle notifications: it reserves
/// lifecycle capacity, allocates the sequence number stamped on every committed item, holds records
/// in production-sequence order, and tracks whether the lifecycle owner needs a deferred wakeup.
///
pub(super) struct DeliveryBroker {
    /// Sequence number to allocate to the next committed item.
    next_sequence: Option<DeliverySequence>,
    /// Lifecycle records in production-sequence order.
    lifecycle: LifecycleQueue,
    /// Capacity reserved by pending or queued process-creation records.
    creation_reservations: LifecycleReservationPool<PROCESS_CREATION_RESERVATIONS>,
    /// Capacity reserved by live processes or queued process-termination records.
    termination_reservations: LifecycleReservationPool<PROCESS_TERMINATION_RESERVATIONS>,
    /// Capacity reserved by live threads or queued thread-termination records.
    thread_termination_reservations: LifecycleReservationPool<THREAD_TERMINATION_RESERVATIONS>,
    /// Does the lifecycle owner need a deferred wakeup attempt?
    lifecycle_wakeup_pending: bool,
    /// Has terminal shutdown disposal already run?
    disposed: bool,
}

impl DeliveryBroker {
    ///
    /// # Description
    ///
    /// Creates an empty delivery broker with preallocated lifecycle storage.
    ///
    /// # Returns
    ///
    /// Upon success, an empty delivery broker is returned. Otherwise, an error is returned.
    ///
    /// # Errors
    ///
    /// This function returns an error if the lifecycle queue cannot be preallocated.
    ///
    pub(super) fn new() -> Result<Self, Error> {
        let lifecycle: LifecycleQueue = LifecycleQueue::new().inspect_err(|error| {
            error!("failed to preallocate lifecycle queue (error={error:?})");
        })?;

        Ok(Self {
            next_sequence: Some(DeliverySequence::new(0)),
            lifecycle,
            creation_reservations: LifecycleReservationPool::new(),
            termination_reservations: LifecycleReservationPool::new(),
            thread_termination_reservations: LifecycleReservationPool::new(),
            lifecycle_wakeup_pending: false,
            disposed: false,
        })
    }

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
    /// The number of lifecycle slots committed or reserved, or [`None`] if accounting overflows.
    ///
    #[cfg(feature = "test")]
    pub(super) fn capacity_in_use(&self) -> Option<usize> {
        self.creation_reservations
            .in_use()
            .checked_add(self.termination_reservations.in_use())
            .and_then(|capacity| {
                capacity.checked_add(self.thread_termination_reservations.in_use())
            })
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
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        if !self.creation_reservations.try_reserve() {
            return Err(Self::reservation_error());
        }
        if !self.termination_reservations.try_reserve() {
            self.creation_reservations.release();
            return Err(Self::reservation_error());
        }

        Ok(LifecycleCreationReservation::new())
    }

    ///
    /// # Description
    ///
    /// Builds the deterministic lifecycle-capacity backpressure error.
    ///
    /// # Returns
    ///
    /// An [`ErrorCode::OutOfMemory`] error describing exhausted lifecycle capacity.
    ///
    fn reservation_error() -> Error {
        let reason: &str = "process lifecycle capacity exhausted";
        error!("{reason}");
        Error::new(ErrorCode::OutOfMemory, reason)
    }

    ///
    /// # Description
    ///
    /// Cancels a process-creation reservation.
    ///
    /// # Parameters
    ///
    /// - `reservation`: Reservation to cancel.
    ///
    pub(super) fn cancel_creation(&mut self, _reservation: LifecycleCreationReservation) {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        self.creation_reservations.release();
        self.termination_reservations.release();
    }

    ///
    /// # Description
    ///
    /// Reserves capacity for the future termination of a thread being created.
    ///
    /// # Returns
    ///
    /// Upon success, a thread lifecycle reservation is returned. Otherwise, an error is returned.
    ///
    /// # Errors
    ///
    /// This function returns an [`ErrorCode::OutOfMemory`] error if thread lifecycle capacity is
    /// exhausted.
    ///
    /// # Panics
    ///
    /// This function panics if lifecycle delivery was already disposed.
    ///
    pub(super) fn try_reserve_thread_creation(
        &mut self,
    ) -> Result<ThreadLifecycleReservation, Error> {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        if !self.thread_termination_reservations.try_reserve() {
            let reason: &str = "thread lifecycle capacity exhausted";
            error!("{reason}");
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }

        Ok(ThreadLifecycleReservation::new())
    }

    ///
    /// # Description
    ///
    /// Cancels the lifecycle reservation for a thread whose creation did not commit.
    ///
    /// # Parameters
    ///
    /// - `_reservation`: Thread lifecycle reservation to cancel.
    ///
    /// # Panics
    ///
    /// This function panics if lifecycle delivery was already disposed or if the reservation pool
    /// is empty.
    ///
    pub(super) fn cancel_thread_creation(&mut self, _reservation: ThreadLifecycleReservation) {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        self.thread_termination_reservations.release();
    }

    ///
    /// # Description
    ///
    /// Transfers a lifecycle reservation into a committed live thread.
    ///
    /// # Parameters
    ///
    /// - `reservation`: Reservation to transfer into the committed thread.
    ///
    /// # Returns
    ///
    /// A capacity credit for the future termination of the committed thread.
    ///
    /// # Panics
    ///
    /// This function panics if lifecycle delivery was already disposed.
    ///
    pub(super) fn commit_thread_creation(
        &mut self,
        reservation: ThreadLifecycleReservation,
    ) -> ThreadLifecycleTerminationCredit {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        reservation.into_credit()
    }

    ///
    /// # Description
    ///
    /// Commits a process-creation record in the ordered delivery domain.
    ///
    /// # Parameters
    ///
    /// - `reservation`: Reservation that supplies capacity for this creation and its termination.
    /// - `info`: The process-creation record to queue.
    ///
    /// # Returns
    ///
    /// A capacity credit for the future termination of the created process.
    ///
    pub(super) fn commit_creation(
        &mut self,
        reservation: LifecycleCreationReservation,
        info: ProcessCreationInfo,
    ) -> LifecycleTerminationCredit {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        let (creation_credit, termination_credit) = reservation.into_credits();
        let sequence: DeliverySequence = self.allocate_sequence();
        self.lifecycle
            .push_back((sequence, LifecycleNotification::Creation(info, creation_credit)));
        self.lifecycle_wakeup_pending = true;
        termination_credit
    }

    ///
    /// # Description
    ///
    /// Commits a process-termination record in the ordered delivery domain.
    ///
    /// # Parameters
    ///
    /// - `credit`: Capacity credit reserved when the process was created.
    /// - `info`: The process-termination record to queue.
    ///
    pub(super) fn commit_termination(
        &mut self,
        credit: LifecycleTerminationCredit,
        info: ProcessTerminationInfo,
    ) {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        let sequence: DeliverySequence = self.allocate_sequence();
        self.lifecycle
            .push_back((sequence, LifecycleNotification::Termination(info, credit)));
        self.lifecycle_wakeup_pending = true;
    }

    ///
    /// # Description
    ///
    /// Commits a thread-termination record in the ordered delivery domain.
    ///
    /// # Parameters
    ///
    /// - `credit`: Capacity credit reserved when the thread was created.
    /// - `info`: Thread-termination record to queue.
    ///
    /// # Panics
    ///
    /// This function panics if lifecycle delivery was already disposed, the delivery sequence is
    /// exhausted, or the preallocated lifecycle queue is full.
    ///
    pub(super) fn commit_thread_termination(
        &mut self,
        credit: ThreadLifecycleTerminationCredit,
        info: ThreadTerminationInfo,
    ) {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        let sequence: DeliverySequence = self.allocate_sequence();
        self.lifecycle
            .push_back((sequence, LifecycleNotification::ThreadTermination(info, credit)));
        self.lifecycle_wakeup_pending = true;
    }

    ///
    /// # Description
    ///
    /// Releases a termination credit when a test process terminates without producing a lifecycle
    /// record.
    ///
    /// # Parameters
    ///
    /// - `credit`: Process-termination capacity credit to release.
    ///
    /// # Panics
    ///
    /// This function panics if lifecycle delivery was already disposed or if the reservation pool
    /// is empty.
    ///
    #[cfg(feature = "test")]
    pub(super) fn release_termination(&mut self, _credit: LifecycleTerminationCredit) {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        self.termination_reservations.release();
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
    /// Peeks the oldest buffered lifecycle record and serializes it into a message addressed to a
    /// process without removing it.
    ///
    /// # Parameters
    ///
    /// - `owner`: Identifier of the process that owns the lifecycle scheduling-event class.
    ///
    /// # Returns
    ///
    /// The delivery sequence and serialized lifecycle message, or [`None`] if no lifecycle record
    /// is buffered.
    ///
    pub(super) fn peek_lifecycle(
        &self,
        owner: ProcessIdentifier,
    ) -> Option<(DeliverySequence, Message)> {
        self.lifecycle
            .front()
            .map(|(sequence, notification)| (*sequence, notification.to_message(owner)))
    }

    ///
    /// # Description
    ///
    /// Reports whether a delivery sequence identifies the lifecycle record at the head of the
    /// broker queue.
    ///
    /// # Parameters
    ///
    /// - `sequence`: Delivery sequence captured by a lifecycle token.
    ///
    /// # Returns
    ///
    /// `true` if `sequence` identifies the current lifecycle record, otherwise `false`.
    ///
    fn lifecycle_token_is_current(&self, sequence: DeliverySequence) -> bool {
        self.lifecycle_sequence() == Some(sequence)
    }

    ///
    /// # Description
    ///
    /// Exposes the lifecycle-token invariant to in-kernel tests.
    ///
    /// # Parameters
    ///
    /// - `sequence`: Delivery sequence captured by the token under test.
    ///
    /// # Returns
    ///
    /// `true` if `sequence` identifies the current lifecycle record, otherwise `false`.
    ///
    #[cfg(feature = "test")]
    pub(super) fn test_lifecycle_token_is_current(&self, sequence: DeliverySequence) -> bool {
        self.lifecycle_token_is_current(sequence)
    }

    ///
    /// # Description
    ///
    /// Commits delivery of the lifecycle record identified by a previously selected sequence,
    /// removing it from the broker queue.
    ///
    /// # Parameters
    ///
    /// - `sequence`: Delivery sequence returned by [`Self::peek_lifecycle`].
    ///
    /// # Panics
    ///
    /// This function panics if `sequence` is stale or does not identify the current lifecycle
    /// record. Such a token indicates a kernel invariant violation under serialized delivery.
    ///
    pub(super) fn commit_lifecycle(&mut self, sequence: DeliverySequence) {
        assert!(!self.disposed, "lifecycle delivery used after disposal");
        assert!(self.lifecycle_token_is_current(sequence), "stale lifecycle delivery token");
        let (_, notification): LifecycleRecord = match self.lifecycle.pop_front() {
            Some(record) => record,
            None => unreachable!("lifecycle delivery token identifies a missing record"),
        };
        self.release_notification(notification);
        if self.lifecycle.is_empty() {
            self.lifecycle_wakeup_pending = false;
        }
    }

    ///
    /// # Description
    ///
    /// Releases the reservation transferred into a queued lifecycle record.
    ///
    /// # Parameters
    ///
    /// - `notification`: Lifecycle record whose reservation is released.
    ///
    /// # Panics
    ///
    /// This function panics if the reservation pool corresponding to `notification` is empty.
    ///
    fn release_notification(&mut self, notification: LifecycleNotification) {
        match notification {
            LifecycleNotification::Creation(..) => self.creation_reservations.release(),
            LifecycleNotification::Termination(..) => self.termination_reservations.release(),
            LifecycleNotification::ThreadTermination(..) => {
                self.thread_termination_reservations.release()
            },
        }
    }

    ///
    /// # Description
    ///
    /// Disposes all lifecycle state during terminal shutdown. Queued records are removed as
    /// undelivered and release the exact reservations they own. Any reservation that remains in a
    /// pending creation, live process, or live thread is then released because no further lifecycle
    /// transition can occur after this terminal operation.
    ///
    /// # Returns
    ///
    /// Disposal accounting that distinguishes undelivered records from externally retained
    /// reservations.
    ///
    /// # Panics
    ///
    /// This function panics if lifecycle delivery was already disposed or if lifecycle reservation
    /// accounting is inconsistent.
    ///
    pub(super) fn dispose(&mut self) -> LifecycleDisposal {
        assert!(!self.disposed, "lifecycle delivery was already disposed");
        self.disposed = true;

        let mut undelivered_records: usize = 0;
        while let Some((_, notification)) = self.lifecycle.pop_front() {
            self.release_notification(notification);
            undelivered_records += 1;
        }
        self.lifecycle_wakeup_pending = false;

        let creation_reservations: usize = self.creation_reservations.release_all();
        let termination_reservations: usize = self.termination_reservations.release_all();
        let thread_termination_reservations: usize =
            self.thread_termination_reservations.release_all();
        let process_reservations: usize =
            match creation_reservations.checked_add(termination_reservations) {
                Some(reservations) => reservations,
                None => unreachable!("lifecycle disposal accounting overflow"),
            };
        let retained_reservations: usize =
            match process_reservations.checked_add(thread_termination_reservations) {
                Some(reservations) => reservations,
                None => unreachable!("lifecycle disposal accounting overflow"),
            };

        LifecycleDisposal {
            undelivered_records,
            retained_reservations,
        }
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

///
/// # Description
///
/// Creates a delivery sequence fixture through the production owner layer for in-kernel tests.
///
/// # Parameters
///
/// - `value`: Raw value to store in the fixture.
///
/// # Returns
///
/// A delivery sequence containing `value`.
///
#[cfg(feature = "test")]
pub(crate) const fn new_test_delivery_sequence(value: u64) -> DeliverySequence {
    DeliverySequence::new(value)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(feature = "test")]
mod test {
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
            let reservation: ThreadLifecycleReservation = match broker.try_reserve_thread_creation()
            {
                Ok(reservation) => reservation,
                Err(error) => {
                    error!("thread lifecycle capacity exhausted too early (error={error:?})");
                    return false;
                },
            };
            let credit: ThreadLifecycleTerminationCredit =
                broker.commit_thread_creation(reservation);
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

        let terminated_reservation: LifecycleCreationReservation =
            match broker.try_reserve_creation() {
                Ok(reservation) => reservation,
                Err(error) => {
                    error!("failed to reserve terminated-process fixture (error={error:?})");
                    return false;
                },
            };
        let terminated_credit: LifecycleTerminationCredit =
            broker.commit_creation(terminated_reservation, creation);
        broker.commit_termination(terminated_credit, termination);

        let _pending_reservation: LifecycleCreationReservation = match broker.try_reserve_creation()
        {
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
