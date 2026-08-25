// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use self::{
    delivery_epoch::DeliveryEpoch,
    event_sequence::EventSequence,
};
use crate::{
    hal::{
        arch::{
            ContextInformation,
            ExceptionInformation,
            InterruptNumber,
            SignalCpuContext,
        },
        Hal,
    },
    mm::VirtMemoryManager,
    pm::{
        exception_to_signal,
        sync::condvar::Condvar,
        ExceptionGuard,
        InterruptReason,
        ProcessManager,
        SleepError,
        SyncSignalOutcome,
        UncommittedMessage,
        UncommittedMessageToken,
    },
};
use ::alloc::{
    boxed::Box,
    collections::{
        LinkedList,
        VecDeque,
    },
};

/// Payload stored per pending exception: sequence number, descriptor, info, and condvar.
type PendingException = (EventSequence, EventDescriptor, ExceptionEventInformation, Condvar);
use ::arch::cpu::excp;
use ::core::cell::{
    RefCell,
    RefMut,
};
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
// Modules
//==================================================================================================

mod delivery_epoch;
mod event_sequence;
/// In-kernel tests for the event manager.
#[cfg(feature = "test")]
pub(super) mod test;

//==================================================================================================
// Structures
//==================================================================================================

static mut MANAGER: Option<EventManager> = None;

struct ExceptionEventInformation {
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    info: ExceptionInformation,
}

/// Event-class eligibility masks for one receive attempt.
#[derive(Clone, Copy)]
struct EventMasks {
    interrupts: usize,
    exceptions: usize,
    scheduling: usize,
}

/// Message and private token selected for transactional delivery.
pub(crate) struct PendingDelivery {
    /// Message to copy to user space.
    message: Message,
    /// Token that commits the selected item after a successful copy.
    token: DeliveryToken,
}

/// Private token identifying an item selected for delivery.
enum DeliveryToken {
    /// Selected interrupt queue head.
    Interrupt {
        epoch: DeliveryEpoch,
        index: usize,
        sequence: EventSequence,
    },
    /// Selected exception queue head.
    Exception {
        epoch: DeliveryEpoch,
        index: usize,
        sequence: EventSequence,
    },
    /// Selected mailbox or lifecycle message.
    Message {
        epoch: DeliveryEpoch,
        token: UncommittedMessageToken,
    },
}

impl PendingDelivery {
    ///
    /// # Description
    ///
    /// Creates a pending delivery that pairs a message with the private token required to commit
    /// it.
    ///
    /// # Parameters
    ///
    /// - `message`: Message selected for delivery.
    /// - `token`: Private token that identifies the selected item.
    ///
    /// # Returns
    ///
    /// A pending delivery containing `message` and `token`.
    ///
    fn new(message: Message, token: DeliveryToken) -> Self {
        Self { message, token }
    }

    ///
    /// # Description
    ///
    /// Returns the message selected for delivery without exposing its commit token.
    ///
    /// # Returns
    ///
    /// A reference to the selected message.
    ///
    pub(crate) fn message(&self) -> &Message {
        &self.message
    }
}

impl DeliveryToken {
    ///
    /// # Description
    ///
    /// Returns the delivery epoch captured when this token was created.
    ///
    /// # Returns
    ///
    /// The token's delivery epoch.
    ///
    fn epoch(&self) -> DeliveryEpoch {
        match self {
            Self::Interrupt { epoch, .. }
            | Self::Exception { epoch, .. }
            | Self::Message { epoch, .. } => *epoch,
        }
    }

    ///
    /// # Description
    ///
    /// Returns the service class that selected this token.
    ///
    /// # Returns
    ///
    /// The index of the interrupt, exception, or message service class.
    ///
    fn class(&self) -> usize {
        match self {
            Self::Interrupt { .. } => EventManagerInner::INTERRUPT_CLASS,
            Self::Exception { .. } => EventManagerInner::EXCEPTION_CLASS,
            Self::Message { .. } => EventManagerInner::MESSAGE_CLASS,
        }
    }

    ///
    /// # Description
    ///
    /// Computes the service cursor value to install after committing this token.
    ///
    /// # Returns
    ///
    /// The service class that follows the class represented by this token.
    ///
    fn next_cursor(&self) -> usize {
        (self.class() + 1) % EventManagerInner::NUMBER_EVENT_CLASSES
    }
}

///
/// # Description
///
/// RAII guard that removes a thread from the event manager's waiting list on drop.
///
struct WaitingThreadGuard {
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
}

impl WaitingThreadGuard {
    /// Registers the thread in the event manager's waiting list and returns a guard that removes
    /// it on drop.
    fn register(pid: ProcessIdentifier, tid: ThreadIdentifier) -> Result<Self, Error> {
        EventManager::get()?
            .try_borrow_mut()?
            .waiting_threads
            .push_back((pid, tid));
        Ok(Self { pid, tid })
    }
}

impl Drop for WaitingThreadGuard {
    fn drop(&mut self) {
        match EventManager::get() {
            Ok(em) => match em.try_borrow_mut() {
                Ok(mut inner) => {
                    inner
                        .waiting_threads
                        .retain(|&(p, t)| p != self.pid || t != self.tid);
                },
                Err(e) => {
                    error!(
                        "WaitingThreadGuard::drop(): failed to borrow event manager (pid={:?}, \
                         tid={:?}, error={:?})",
                        self.pid, self.tid, e
                    );
                },
            },
            Err(e) => {
                error!(
                    "WaitingThreadGuard::drop(): failed to get event manager (pid={:?}, tid={:?}, \
                     error={:?})",
                    self.pid, self.tid, e
                );
            },
        }
    }
}

pub struct EventOwnership {
    ev: Event,
}

/// Result of changing event ownership through [`EventManager::evctrl`].
pub(crate) enum EventCtrlOutcome {
    /// Ownership was newly acquired and the guard must be stored by the process.
    Acquired(EventOwnership),
    /// An idempotent registration left existing ownership unchanged.
    Unchanged,
    /// Ownership was released and the process's matching guard must be removed.
    Released,
}

impl EventOwnership {
    pub fn event(&self) -> &Event {
        &self.ev
    }
}

impl Drop for EventOwnership {
    fn drop(&mut self) {
        let em: &EventManager = match EventManager::get() {
            Ok(em) => em,
            Err(error) => {
                error!("failed to get event manager while releasing ownership: {error:?}");
                return;
            },
        };
        match em.try_borrow_mut() {
            Ok(mut em) => em.release_ownership(self.ev),
            Err(error) => error!("failed to borrow event manager: {error:?}"),
        }
    }
}

struct EventManagerInner {
    interrupt_capable: bool,
    /// Full-width sequence assigned to the latest generated event and used as the FIFO ordering key
    /// in `try_wait()` independently of the truncated [`EventDescriptor`] identifier.
    event_sequence: EventSequence,
    /// Epoch stamped into the next delivery token and advanced once per successful commit.
    delivery_epoch: DeliveryEpoch,
    wait: Option<Condvar>,
    waiting_threads: VecDeque<(ProcessIdentifier, ThreadIdentifier)>,
    interrupt_ownership: Box<[Option<ProcessIdentifier>]>,
    pending_interrupts: Box<[LinkedList<(EventSequence, EventDescriptor)>]>,
    exception_ownership: Box<[Option<ProcessIdentifier>]>,
    pending_exceptions: Box<[LinkedList<PendingException>]>,
    scheduling_owner: Option<ProcessIdentifier>,
}

impl EventManagerInner {
    const NUMBER_EVENT_CLASSES: usize = 3;
    /// Service class of pending interrupts.
    const INTERRUPT_CLASS: usize = 0;
    /// Service class of pending exceptions.
    const EXCEPTION_CLASS: usize = 1;
    /// Service class of ordered message-like items.
    const MESSAGE_CLASS: usize = 2;

    ///
    /// # Description
    ///
    /// Advances the event sequence and returns the value assigned to the next generated interrupt
    /// or exception.
    ///
    /// # Returns
    ///
    /// The next event sequence number.
    ///
    /// # Panics
    ///
    /// This function panics if the event sequence is exhausted, because continuing would silently
    /// reorder pending events.
    ///
    fn next_event_sequence(&mut self) -> EventSequence {
        let sequence: EventSequence = match self.event_sequence.checked_next() {
            Some(sequence) => sequence,
            None => unreachable!("event sequence exhausted"),
        };
        self.event_sequence = sequence;
        sequence
    }

    /// Releases an event ownership without performing registration-time capability checks.
    fn release_ownership(&mut self, ev: Event) {
        match ev {
            Event::Interrupt(ev) => self.interrupt_ownership[usize::from(ev)] = None,
            Event::Exception(ev) => self.exception_ownership[usize::from(ev)] = None,
            Event::Scheduling(_) => self.scheduling_owner = None,
        }
    }

    ///
    /// # Description
    ///
    /// Reports whether an exception vector has been claimed by an exception owner via `evctrl()`.
    ///
    /// The exception path consults this to enforce signal-generation precedence: an owner-claimed
    /// vector is forwarded to that owner, while an unclaimed vector may instead be mapped to a
    /// synchronous signal on the faulting thread.
    ///
    /// # Parameters
    ///
    /// - `vector`: The CPU exception vector number.
    ///
    /// # Returns
    ///
    /// `true` if the vector is currently owned, `false` otherwise (including out-of-range vectors,
    /// which can never be owned).
    ///
    fn exception_has_owner(&self, vector: u32) -> bool {
        match self.exception_ownership.get(vector as usize) {
            Some(owner) => owner.is_some(),
            None => false,
        }
    }

    fn do_evctrl_interrupt(
        &mut self,
        pm: &ProcessManager,
        pid: Option<ProcessIdentifier>,
        ev: InterruptEvent,
        req: EventCtrlRequest,
    ) -> Result<(), Error> {
        // Check if target interrupt is already owned by another process.
        let idx: usize = usize::from(ev);
        if self.interrupt_ownership[idx].is_some() {
            let reason: &str = "interrupt is already owned by another process";
            error!("reason={:?}", reason);
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        // Handle request.
        match req {
            EventCtrlRequest::Register => {
                // Check if PID is valid.
                if let Some(pid) = pid {
                    // Ensure that the process has the required capabilities.
                    if !pm.has_capability(pid, Capability::InterruptControl)? {
                        let reason: &str = "process does not have interrupt control capability";
                        error!("reason={:?}", reason);
                        return Err(Error::new(ErrorCode::PermissionDenied, reason));
                    }

                    // Check if target interrupt is already owned by another process.
                    if self.interrupt_ownership[idx].is_some() {
                        let reason: &str = "interrupt is already owned by another process";
                        error!("reason={:?}", reason);
                        return Err(Error::new(ErrorCode::ResourceBusy, reason));
                    }

                    // Register interrupt.
                    self.interrupt_ownership[idx] = Some(pid);

                    return Ok(());
                }

                let reason: &str = "invalid process identifier";
                error!("reason={:?}", reason);
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
            EventCtrlRequest::Unregister => {
                // If PID was supplied, check if it matches the current owner.
                if let Some(pid) = pid {
                    if self.interrupt_ownership[idx] != Some(pid) {
                        let reason: &str = "process does not own interrupt";
                        error!("reason={:?}", reason);
                        return Err(Error::new(ErrorCode::PermissionDenied, reason));
                    }
                }

                // Unregister interrupt.
                self.interrupt_ownership[idx] = None;

                Ok(())
            },
        }
    }

    fn do_evctrl_exception(
        &mut self,
        pm: &ProcessManager,
        pid: Option<ProcessIdentifier>,
        ev: ExceptionEvent,
        req: EventCtrlRequest,
    ) -> Result<(), Error> {
        let idx: usize = usize::from(ev);

        // Handle request.
        match req {
            EventCtrlRequest::Register => {
                // Check if PID is valid.
                if let Some(pid) = pid {
                    // Ensure that the process has the required capabilities.
                    if !pm.has_capability(pid, Capability::ExceptionControl)? {
                        let reason: &str = "process does not have exception control capability";
                        error!("reason={:?}", reason);
                        return Err(Error::new(ErrorCode::PermissionDenied, reason));
                    }

                    // Check if target exception is already owned by another process.
                    if self.exception_ownership[idx].is_some() {
                        let reason: &str = "exception is already owned by another process";
                        error!("reason={:?}", reason);
                        return Err(Error::new(ErrorCode::ResourceBusy, reason));
                    }

                    // Register exception.
                    self.exception_ownership[idx] = Some(pid);

                    return Ok(());
                }

                let reason: &str = "invalid process identifier";
                error!("reason={:?}", reason);
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
            EventCtrlRequest::Unregister => {
                // If PID was supplied, check if it matches the current owner.
                if let Some(pid) = pid {
                    if self.exception_ownership[idx] != Some(pid) {
                        let reason: &str = "process does not own exception";
                        error!("reason={:?}", reason);
                        return Err(Error::new(ErrorCode::PermissionDenied, reason));
                    }
                }

                // Unregister exception.
                self.exception_ownership[idx] = None;

                Ok(())
            },
        }
    }

    ///
    /// Returns `true` when ownership of the scheduling-event class was newly acquired by this
    /// request (i.e. the class previously had no owner). Returns `false` when the request was a
    /// no-op with respect to ownership (idempotent re-registration by the current owner, or any
    /// unregistration). Callers use this to ensure that a single ownership guard is handed out per
    /// acquisition, so that an idempotent re-registration does not release the class prematurely
    /// when its (spurious) guard is dropped.
    fn do_evctrl_scheduling(
        &mut self,
        pm: &ProcessManager,
        pid: Option<ProcessIdentifier>,
        _ev: SchedulingEvent,
        req: EventCtrlRequest,
    ) -> Result<bool, Error> {
        // Scheduling events are owned as a single class: a process either owns every scheduling
        // event or none of them. The specific event in `_ev` therefore does not influence
        // arbitration.

        // Handle request.
        match req {
            EventCtrlRequest::Register => {
                // Check if PID is valid.
                if let Some(pid) = pid {
                    // Ensure that the process has the required capabilities.
                    if !pm.has_capability(pid, Capability::ProcessManagement)? {
                        let reason: &str = "process does not have process management capability";
                        error!("reason={:?}", reason);
                        return Err(Error::new(ErrorCode::PermissionDenied, reason));
                    }

                    // Check if the scheduling-event class is already owned by another process.
                    let newly_acquired: bool = match self.scheduling_owner {
                        Some(owner) => {
                            if owner != pid {
                                let reason: &str =
                                    "scheduling events are already owned by another process";
                                error!("reason={:?}", reason);
                                return Err(Error::new(ErrorCode::ResourceBusy, reason));
                            }
                            false
                        },
                        None => true,
                    };

                    // Claim ownership of the scheduling-event class.
                    self.scheduling_owner = Some(pid);

                    return Ok(newly_acquired);
                }

                let reason: &str = "invalid process identifier";
                error!("reason={:?}", reason);
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
            EventCtrlRequest::Unregister => {
                // If PID was supplied, check if it matches the current owner.
                if let Some(pid) = pid {
                    if self.scheduling_owner != Some(pid) {
                        let reason: &str = "process does not own scheduling events";
                        error!("reason={:?}", reason);
                        return Err(Error::new(ErrorCode::PermissionDenied, reason));
                    }
                }

                // Release ownership of the scheduling-event class.
                self.scheduling_owner = None;

                Ok(false)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to wait on an event.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the target thread.
    /// - `pid`: Identifier of the target process.
    ///
    /// # Returns
    ///
    /// The selected pending delivery, [`None`] if no item is eligible, or an error if selection
    /// fails.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    unsafe fn try_wait(
        &mut self,
        tid: ThreadIdentifier,
        pid: ProcessIdentifier,
    ) -> Result<Option<PendingDelivery>, Error> {
        // SAFETY: the caller holds no reference to the process manager.
        let cursor: usize = unsafe { ProcessManager::delivery_cursor() };
        self.try_wait_with(tid, pid, cursor, |tid, lifecycle_eligible| unsafe {
            ProcessManager::peek_recv(tid, lifecycle_eligible)
        })
    }

    ///
    /// # Description
    ///
    /// Returns event-class eligibility masks based on the events currently owned by a process.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process whose event ownership is queried.
    ///
    /// # Returns
    ///
    /// The event-class eligibility masks for the process's current event ownership.
    ///
    fn event_masks(&self, pid: ProcessIdentifier) -> EventMasks {
        let mut interrupts: usize = 0;
        for (idx, owner) in self.interrupt_ownership.iter().enumerate() {
            if *owner == Some(pid) {
                interrupts |= 1usize << idx;
            }
        }

        let mut exceptions: usize = 0;
        for (idx, owner) in self.exception_ownership.iter().enumerate() {
            if *owner == Some(pid) {
                exceptions |= 1usize << idx;
            }
        }

        // Scheduling events are owned as a single class: the owner waits on every scheduling event
        // at once.
        let scheduling: usize = if self.scheduling_owner == Some(pid) {
            (1usize << SchedulingEvent::NUMBER_EVENTS) - 1
        } else {
            0
        };

        EventMasks {
            interrupts,
            exceptions,
            scheduling,
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to select an event or mailbox message using the process's current event ownership
    /// and `try_recv` for mailbox delivery.
    ///
    /// This is the behavior-preserving selection core used by [`Self::try_wait`]. Accepting the
    /// mailbox receiver as a dependency lets in-kernel tests exercise event and mailbox selection
    /// together without accessing the global process manager.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the target thread.
    /// - `pid`: Identifier of the target process.
    /// - `cursor`: Receiver's service cursor, advanced past the class that delivered a message.
    /// - `try_recv`: Selector for the ordered message-like class, given the thread and whether
    ///   lifecycle records are eligible.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the selected message is returned, or [`None`] when no class had
    /// an eligible item. Otherwise, an error is returned instead.
    ///
    fn try_wait_with<F>(
        &mut self,
        tid: ThreadIdentifier,
        pid: ProcessIdentifier,
        cursor: usize,
        try_recv: F,
    ) -> Result<Option<PendingDelivery>, Error>
    where
        F: FnMut(ThreadIdentifier, bool) -> Result<Option<UncommittedMessage>, Error>,
    {
        let masks: EventMasks = self.event_masks(pid);
        self.select_with(tid, pid, masks, cursor, try_recv)
    }

    ///
    /// # Description
    ///
    /// Selects an event or mailbox message using explicit event-class eligibility masks and
    /// `try_recv` for mailbox delivery.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the target thread.
    /// - `pid`: Identifier of the target process.
    /// - `masks`: Event-class eligibility masks for this selection attempt.
    /// - `cursor`: Receiver's service cursor, advanced past the class that delivered a message.
    /// - `try_recv`: Selector for the ordered message-like class, given the thread and whether
    ///   lifecycle records are eligible.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the selected message is returned, or [`None`] when no class had
    /// an eligible item. Otherwise, an error is returned instead.
    ///
    fn select_with<F>(
        &mut self,
        tid: ThreadIdentifier,
        pid: ProcessIdentifier,
        masks: EventMasks,
        cursor: usize,
        mut try_recv: F,
    ) -> Result<Option<PendingDelivery>, Error>
    where
        F: FnMut(ThreadIdentifier, bool) -> Result<Option<UncommittedMessage>, Error>,
    {
        for offset in 0..Self::NUMBER_EVENT_CLASSES {
            let class: usize = (cursor + offset) % Self::NUMBER_EVENT_CLASSES;

            // Check if any interrupts were triggered.
            if class == Self::INTERRUPT_CLASS {
                // Deliver the oldest eligible interrupt first. This prevents a continuously
                // refilled low-numbered bit from starving an older high-numbered one.
                let selected: Option<usize> =
                    Self::smallest_pending_front(masks.interrupts, |bit| {
                        self.pending_interrupts[bit].front().map(|(seq, _)| *seq)
                    });
                if let Some(idx) = selected {
                    if let Some((sequence, _event)) = self.pending_interrupts[idx].front() {
                        let message: Message = Message {
                            source: MessageSender::KERNEL,
                            destination: MessageReceiver::new(pid, ThreadIdentifier::NONE),
                            message_type: MessageType::Interrupt,
                            ..Message::default()
                        };
                        return Ok(Some(PendingDelivery::new(
                            message,
                            DeliveryToken::Interrupt {
                                epoch: self.delivery_epoch,
                                index: idx,
                                sequence: *sequence,
                            },
                        )));
                    }
                }
            }

            // Check if any exceptions were triggered.
            if class == Self::EXCEPTION_CLASS {
                // Deliver the oldest eligible exception first, using the same FIFO-by-sequence rule
                // as interrupts.
                let selected: Option<usize> =
                    Self::smallest_pending_front(masks.exceptions, |bit| {
                        self.pending_exceptions[bit]
                            .front()
                            .map(|(seq, _, _, _)| *seq)
                    });
                if let Some(idx) = selected {
                    if let Some(entry) = self.pending_exceptions[idx].front() {
                        let mut info: EventInformation = EventInformation::default();
                        info.id = entry.1.clone();
                        info.pid = entry.2.pid;
                        info.number = Some(entry.2.info.num() as usize);
                        info.code = Some(entry.2.info.code() as usize);
                        info.address = Some(entry.2.info.addr() as usize);
                        info.instruction = Some(entry.2.info.instruction() as usize);

                        let mut message: Message = Message::from(info);
                        message.destination = MessageReceiver::new(pid, ThreadIdentifier::NONE);
                        message.message_type = MessageType::Exception;

                        return Ok(Some(PendingDelivery::new(
                            message,
                            DeliveryToken::Exception {
                                epoch: self.delivery_epoch,
                                index: idx,
                                sequence: entry.0,
                            },
                        )));
                    }
                }
            }

            // Check if any ordered message-like item is eligible for delivery.
            if class == Self::MESSAGE_CLASS {
                if let Some(delivery) = try_recv(tid, masks.scheduling != 0)? {
                    let (message, token): (Message, UncommittedMessageToken) =
                        delivery.into_parts();
                    return Ok(Some(PendingDelivery::new(
                        message,
                        DeliveryToken::Message {
                            epoch: self.delivery_epoch,
                            token,
                        },
                    )));
                }
            }
        }

        Ok(None)
    }

    ///
    /// # Description
    ///
    /// Reports whether an interrupt token still identifies the head of its selected queue.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the selected interrupt queue.
    /// - `sequence`: Event sequence captured by the token.
    ///
    /// # Returns
    ///
    /// `true` if `sequence` identifies the current queue head, otherwise `false`.
    ///
    fn interrupt_token_is_current(&self, index: usize, sequence: EventSequence) -> bool {
        self.pending_interrupts[index]
            .front()
            .map(|(selected_sequence, _)| *selected_sequence)
            == Some(sequence)
    }

    ///
    /// # Description
    ///
    /// Reports whether an exception token still identifies the head of its selected queue.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the selected exception queue.
    /// - `sequence`: Event sequence captured by the token.
    ///
    /// # Returns
    ///
    /// `true` if `sequence` identifies the current queue head, otherwise `false`.
    ///
    fn exception_token_is_current(&self, index: usize, sequence: EventSequence) -> bool {
        self.pending_exceptions[index]
            .front()
            .map(|(selected_sequence, _, _, _)| *selected_sequence)
            == Some(sequence)
    }

    ///
    /// # Description
    ///
    /// Reports whether a token was selected in the current delivery epoch.
    ///
    /// # Parameters
    ///
    /// - `epoch`: Delivery epoch captured by the token.
    ///
    /// # Returns
    ///
    /// `true` if `epoch` is current, otherwise `false`.
    ///
    fn delivery_epoch_is_current(&self, epoch: DeliveryEpoch) -> bool {
        self.delivery_epoch == epoch
    }

    ///
    /// # Description
    ///
    /// Commits a selected delivery token. Interrupts are removed, replayable exceptions are moved
    /// to the back of their queue, and mailbox or lifecycle tokens are delegated to
    /// `commit_message`.
    ///
    /// # Parameters
    ///
    /// - `token`: Token identifying the selected delivery.
    /// - `commit_message`: Operation that commits a mailbox or lifecycle token.
    ///
    /// # Panics
    ///
    /// This function panics if the delivery epoch is stale or exhausted, or if the token no longer
    /// identifies the selected queue entry. Any such condition indicates a kernel invariant
    /// violation.
    ///
    fn commit_with<F>(&mut self, token: DeliveryToken, commit_message: F)
    where
        F: FnOnce(UncommittedMessageToken),
    {
        assert!(self.delivery_epoch_is_current(token.epoch()), "stale or duplicate delivery token");
        let next_epoch: DeliveryEpoch = match self.delivery_epoch.checked_next() {
            Some(epoch) => epoch,
            None => unreachable!("delivery epoch exhausted"),
        };

        match token {
            DeliveryToken::Interrupt {
                index, sequence, ..
            } => {
                assert!(
                    self.interrupt_token_is_current(index, sequence),
                    "stale interrupt delivery token"
                );
                if self.pending_interrupts[index].pop_front().is_none() {
                    unreachable!("interrupt delivery token identifies a missing event");
                }
            },
            DeliveryToken::Exception {
                index, sequence, ..
            } => {
                assert!(
                    self.exception_token_is_current(index, sequence),
                    "stale exception delivery token"
                );
                let Some(entry) = self.pending_exceptions[index].pop_front() else {
                    unreachable!("exception delivery token identifies a missing event");
                };
                self.pending_exceptions[index].push_back(entry);
            },
            DeliveryToken::Message { token, .. } => {
                commit_message(token);
            },
        }
        self.delivery_epoch = next_epoch;
    }

    ///
    /// # Description
    ///
    /// Commits a selected delivery token, delegating mailbox and lifecycle mutations to the global
    /// process manager.
    ///
    /// # Parameters
    ///
    /// - `token`: Token identifying the selected delivery.
    ///
    /// # Panics
    ///
    /// This function panics if `token` is stale, duplicated, or no longer identifies its selected
    /// item.
    ///
    fn commit(&mut self, token: DeliveryToken) {
        self.commit_with(token, |token| {
            // SAFETY: selection and commit execute serially without holding a process-manager
            // reference.
            unsafe { ProcessManager::commit_recv(token) };
        });
    }

    ///
    /// # Description
    ///
    /// Selects the queue that should deliver next under FIFO-by-sequence ordering: among the bits
    /// set in `mask`, returns the index whose pending-queue head carries the smallest event
    /// sequence number, or [`None`] when no selected bit has a pending entry.
    ///
    /// Delivering the smallest-sequence head first makes per-class delivery starvation-free: every
    /// queued event has a fixed position in the global sequence order and is therefore served within
    /// a bounded number of calls, regardless of which bit it occupies. The sequence number is the
    /// full-width [`EventSequence`] stamped at generation time, so ordering is stable even where
    /// the truncated [`EventDescriptor`] id would wrap on the kernel's 32-bit target (see issue
    /// #2674).
    ///
    /// # Parameters
    ///
    /// - `mask`: Bit mask of queues eligible for delivery.
    /// - `front_seq`: Returns the event sequence number at the head of the queue for a given bit, or
    ///   [`None`] when that queue is empty.
    ///
    /// # Returns
    ///
    /// The index of the queue to deliver from, or [`None`] when no eligible queue has a pending
    /// entry.
    ///
    fn smallest_pending_front<F>(mask: usize, mut front_seq: F) -> Option<usize>
    where
        F: FnMut(usize) -> Option<EventSequence>,
    {
        (0..usize::BITS as usize)
            .filter(|&bit| (mask & (1usize << bit)) != 0)
            .filter_map(|bit| front_seq(bit).map(|seq| (bit, seq)))
            .min_by_key(|&(_, seq)| seq)
            .map(|(bit, _)| bit)
    }

    ///
    /// # Description
    ///
    /// Resumes execution after an exception.
    ///
    /// # Parameters
    ///
    /// - `evdesc`: Full event descriptor (id + event type) identifying the pending exception.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    unsafe fn resume_exception(&mut self, evdesc: EventDescriptor) -> Result<(), Error> {
        let idx: usize = match evdesc.event() {
            Event::Exception(ev) => usize::from(ev),
            other => {
                let reason: &str = "event descriptor does not refer to an exception";
                error!("reason={:?}, event={:?}", reason, other);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        // Check that the exception has an owner.
        if self.exception_ownership[idx].is_none() {
            let reason: &str = "no owner for exception";
            error!("reason={:?}", reason);
            unimplemented!("terminate process")
        }

        // Search and remove event from pending exceptions by full descriptor (id + event).
        if let Some(entry) = self.pending_exceptions[idx]
            .iter()
            .position(|(_seq, pending_evdesc, _info, _resume)| *pending_evdesc == evdesc)
        {
            let (_seq, _eventinfo, excpinfo, resume) = self.pending_exceptions[idx].remove(entry);

            if let Err(error) = resume.notify_thread(excpinfo.tid) {
                // The faulting thread may have already been terminated by the exception owner .
                // This is a legitimate outcome, not an error.
                warn!("faulting thread already gone (tid={:?}, error={:?})", excpinfo.tid, error);
            }
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Wakes up a process that is waiting on an interrupt.
    ///
    /// # Parameters
    ///
    /// - `interrupts`: Bit mask of interrupts.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    unsafe fn wakeup_interrupt(&mut self, interrupts: usize) -> Result<(), Error> {
        // Check if an spurious interrupt was received.
        if self.interrupt_capable {
            let reason: &str = "interrupt manager is not capable of handling ginterrupts";
            error!("reason={:?}", reason);
            return Err(Error::new(ErrorCode::OperationNotSupported, reason));
        }

        let sequence: EventSequence = self.next_event_sequence();
        let idx: usize = interrupts.trailing_zeros() as usize;
        let ev = Event::from(sys::event::InterruptEvent::try_from(idx)?);
        let descriptor: EventDescriptor = EventDescriptor::new(sequence.descriptor_id(), ev);
        self.pending_interrupts[idx].push_back((sequence, descriptor));

        // Get interrupt owner.
        let pid: ProcessIdentifier = match self.interrupt_ownership[idx] {
            Some(owner) => owner,
            None => {
                let reason: &str = "no owner for interrupt";
                error!("reason={:?}", reason);
                return Err(Error::new(ErrorCode::NoSuchProcess, reason));
            },
        };

        self.notify_all_process_threads(pid)
    }

    ///
    /// # Description
    ///
    /// Wakes up a process that is waiting on an exception.
    ///
    /// # Parameters
    ///
    /// - `exceptions`: Bit mask of exceptions.
    /// - `pid`: Identifier of the faulting process.
    /// - `tid`: Identifier of the faulting thread.
    /// - `info`: Exception information.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a condition variable is returned. Otherwise, an error is returned
    /// instead.
    ///
    /// # Safety
    ///
    /// This function panics if the process manager is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if an only if the following conditions are met:
    ///
    /// - The process manager is initialized.
    ///
    unsafe fn wakeup_exception(
        &mut self,
        exceptions: usize,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        info: &ExceptionInformation,
    ) -> Result<Condvar, Error> {
        trace!("exceptions={:#x}, pid={:?}, tid={:?}, info={:?}", exceptions, pid, tid, info);
        let sequence: EventSequence = self.next_event_sequence();
        let idx: usize = exceptions.trailing_zeros() as usize;
        let ev: Event = Event::from(ExceptionEvent::try_from(idx)?);
        let descriptor: EventDescriptor = EventDescriptor::new(sequence.descriptor_id(), ev);
        let resume: Condvar = Condvar::new();
        self.pending_exceptions[idx].push_back((
            sequence,
            descriptor,
            ExceptionEventInformation {
                pid,
                tid,
                info: info.clone(),
            },
            resume.clone(),
        ));

        // Get exception owner.
        let pid: ProcessIdentifier = match self.exception_ownership[idx] {
            Some(owner) => owner,
            None => {
                let reason: &str = "no owner for exception";
                error!("{reason}");
                return Err(Error::new(ErrorCode::NoSuchProcess, reason));
            },
        };

        // Notify exception owner.
        self.notify_all_process_threads(pid)?;

        Ok(resume)
    }

    fn post_message(
        &mut self,
        pm: &mut ProcessManager,
        receiver: MessageReceiver,
        message: Message,
    ) -> Result<(), Error> {
        pm.post_message(receiver, message)?;

        if receiver.tid.is_none() {
            // SAFETY: the calling process does not hold mutable reference to the inner state of the process manager.
            unsafe { self.notify_all_process_threads(receiver.pid) }
        } else {
            // SAFETY: the calling process does not hold mutable reference to the inner state of the process manager.
            unsafe { self.get_wait().notify_thread(receiver.tid) }
        }
    }

    ///
    /// # Description
    ///
    /// Wakes the process that owns lifecycle scheduling events, if one is registered.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function panics if the process manager is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if an only if the following conditions are met:
    ///
    /// - The process manager is initialized.
    ///
    unsafe fn notify_lifecycle_owner(&mut self) -> Result<(), Error> {
        match self.scheduling_owner {
            Some(pid) => {
                trace!("waking lifecycle owner: pid={:?}", pid);
                self.notify_all_process_threads(pid)
            },
            None => {
                trace!("lifecycle records buffered with no owner");
                Ok(())
            },
        }
    }

    fn get_wait(&self) -> &Condvar {
        // NOTE: it is safe to unwrap because the wait field is always Some.
        self.wait.as_ref().unwrap()
    }

    ///
    /// # Description
    ///
    /// Wakes up all threads of a process that are waiting on the event manager.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the target process.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    unsafe fn notify_all_process_threads(&self, pid: ProcessIdentifier) -> Result<(), Error> {
        let mut first_error: Option<Error> = None;

        // Wake up all threads of the target process in the waiting list.
        for (_p, tid) in self.waiting_threads.iter().filter(|(p, _)| *p == pid) {
            if let Err(error) = self.get_wait().notify_thread(*tid) {
                warn!("{error:?} (pid={pid:?}, tid={tid:?})");
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

//==================================================================================================
// Event Manager
//==================================================================================================

pub struct EventManager(RefCell<EventManagerInner>);

impl EventManager {
    ///
    /// # Description
    ///
    /// Resumes an execution after an event.
    ///
    /// # Parameters
    ///
    /// - `evdesc`: Event descriptor.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn resume(evdesc: EventDescriptor) -> Result<(), Error> {
        trace!("evdesc={:?}", evdesc);
        match evdesc.event() {
            Event::Interrupt(_ev) => {
                // No further action is required for interrupts.
                Ok(())
            },
            Event::Exception(_ev) => EventManager::get()?
                .try_borrow_mut()?
                .resume_exception(evdesc),
            Event::Scheduling(_ev) => {
                // No further action is required for scheduling events.
                Ok(())
            },
        }
    }

    ///
    /// # Description
    ///
    /// Waits for an event to be delivered.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the waiting thread.
    /// - `pid`: Identifier of the waiting process.
    ///
    /// # Returns
    ///
    /// The selected pending delivery, or a sleep error if registration or waiting fails.
    ///
    /// # Safety
    ///
    /// This function panics if the kernel process tries to sleep.
    ///
    /// This function is unsafe because it blocks the calling thread until it is woken up by another
    /// thread.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    /// - This function is invoked without holding any resources.
    ///
    pub(crate) unsafe fn wait(
        tid: ThreadIdentifier,
        pid: ProcessIdentifier,
    ) -> Result<PendingDelivery, SleepError> {
        let wait: Condvar = EventManager::get()
            .map_err(SleepError::Generic)?
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .get_wait()
            .clone();

        // Register this thread in the waiting list so producers can find it by pid.
        let _guard: WaitingThreadGuard =
            WaitingThreadGuard::register(pid, tid).map_err(SleepError::Generic)?;

        loop {
            let delivery: Option<PendingDelivery> = EventManager::get()
                .map_err(SleepError::Generic)?
                .try_borrow_mut()
                .map_err(SleepError::Generic)?
                .try_wait(tid, pid)
                .map_err(SleepError::Generic)?;

            if let Some(delivery) = delivery {
                break Ok(delivery);
            }

            // Wait for an event to be delivered.
            wait.wait(None)?;
        }
    }

    ///
    /// # Description
    ///
    /// Commits a pending delivery after its message was copied successfully, removing the selected
    /// item and advancing the service cursor of the receiving process.
    ///
    /// # Parameters
    ///
    /// - `delivery`: Pending delivery returned by [`EventManager::wait`].
    ///
    /// # Panics
    ///
    /// This function panics if the token no longer identifies the selected item, which indicates a
    /// stale or duplicate commit and thus a kernel bug.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The caller holds no reference to the event manager nor to the process manager.
    /// - `delivery` was returned by the last [`EventManager::wait`] and was not committed yet.
    ///
    pub(crate) unsafe fn commit(delivery: PendingDelivery) {
        let next_cursor: usize = delivery.token.next_cursor();
        let Ok(manager) = EventManager::get() else {
            unreachable!("event manager disappeared before delivery commit");
        };
        let Ok(mut inner) = manager.try_borrow_mut() else {
            unreachable!("event manager was borrowed during delivery commit");
        };
        inner.commit(delivery.token);
        drop(inner);

        // SAFETY: commit does not retain a reference to the process manager.
        unsafe { ProcessManager::set_delivery_cursor(next_cursor) };
    }

    ///
    /// # Description
    ///
    /// Changes ownership of an event class on behalf of a process.
    ///
    /// # Parameters
    ///
    /// - `pm`: Reference to the process manager.
    /// - `pid`: Identifier of the requesting process.
    /// - `ev`: Event whose ownership is being changed.
    /// - `req`: Whether ownership is being acquired or released.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the resulting ownership outcome is returned. Otherwise, an error
    /// is returned instead.
    ///
    pub fn evctrl(
        pm: &mut ProcessManager,
        pid: ProcessIdentifier,
        ev: Event,
        req: EventCtrlRequest,
    ) -> Result<EventCtrlOutcome, Error> {
        trace!("ev={:?}, req={:?}", ev, req);

        let em: &EventManager = EventManager::get()?;

        let newly_acquired: bool = match ev {
            Event::Interrupt(interrupt_event) => {
                // Check if the interrupt manager is capable of handling interrupts.
                if !em.try_borrow_mut()?.interrupt_capable {
                    let reason: &str = "interrupt manager is not capable of handling ginterrupts";
                    error!("{:?} (reason={:?})", reason, req);
                    return Err(Error::new(ErrorCode::OperationNotSupported, reason));
                }
                em.try_borrow_mut()?
                    .do_evctrl_interrupt(pm, Some(pid), interrupt_event, req)?;
                // Interrupt registration always acquires fresh ownership: it errors out when the
                // interrupt is already owned.
                true
            },
            Event::Exception(exception_event) => {
                em.try_borrow_mut()?
                    .do_evctrl_exception(pm, Some(pid), exception_event, req)?;
                // Exception registration always acquires fresh ownership: it errors out when the
                // exception is already owned.
                true
            },
            Event::Scheduling(scheduling_event) => {
                let newly_acquired: bool = em.try_borrow_mut()?.do_evctrl_scheduling(
                    pm,
                    Some(pid),
                    scheduling_event,
                    req,
                )?;
                if matches!(req, EventCtrlRequest::Register) && pm.has_pending_lifecycle() {
                    pm.request_lifecycle_wakeup();
                }
                newly_acquired
            },
        };

        match req {
            // Hand out an ownership guard only when ownership was newly acquired. An idempotent
            // re-registration by the current owner must not produce a guard, otherwise dropping it
            // would release the entire class while other guards (and the registration intent)
            // remain.
            EventCtrlRequest::Register if newly_acquired => {
                Ok(EventCtrlOutcome::Acquired(EventOwnership { ev }))
            },
            EventCtrlRequest::Register => Ok(EventCtrlOutcome::Unchanged),
            EventCtrlRequest::Unregister => Ok(EventCtrlOutcome::Released),
        }
    }

    pub fn post_message(
        pm: &mut ProcessManager,
        receiver: MessageReceiver,
        message: Message,
    ) -> Result<(), Error> {
        Self::get_mut()?
            .try_borrow_mut()?
            .post_message(pm, receiver, message)
    }

    ///
    /// # Description
    ///
    /// Wakes the process that owns lifecycle scheduling events, if one is registered.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    ///
    /// # Safety
    ///
    /// This function panics if the process manager is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if an only if the following conditions are met:
    ///
    /// - The process manager is initialized.
    ///
    pub unsafe fn notify_process_lifecycle() -> Result<(), Error> {
        Self::get_mut()?.try_borrow_mut()?.notify_lifecycle_owner()
    }

    fn try_borrow_mut(&self) -> Result<RefMut<'_, EventManagerInner>, Error> {
        match self.0.try_borrow_mut() {
            Ok(em) => Ok(em),
            Err(e) => {
                let reason: &str = "failed to borrow event manager";
                error!("{:?} (error={:?})", reason, e);
                Err(Error::new(ErrorCode::PermissionDenied, reason))
            },
        }
    }

    fn get<'a>() -> Result<&'a EventManager, Error> {
        unsafe {
            match MANAGER {
                Some(ref em) => Ok(em),
                None => {
                    let reason: &str = "event manager is not initialized";
                    error!("reason={:?}", reason);
                    Err(Error::new(ErrorCode::TryAgain, reason))
                },
            }
        }
    }

    fn get_mut<'a>() -> Result<&'a mut EventManager, Error> {
        unsafe {
            match MANAGER {
                Some(ref mut em) => Ok(em),
                None => {
                    let reason: &str = "event manager is not initialized";
                    error!("reason={:?}", reason);
                    Err(Error::new(ErrorCode::TryAgain, reason))
                },
            }
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn interrupt_handler(intnum: InterruptNumber) {
    trace!("intnum={:?}", intnum);
    match EventManager::get_mut() {
        Ok(em) => match em.try_borrow_mut() {
            // SAFETY: the calling process does not hold a mutable reference to the inner state of the process manager.
            Ok(mut em) => match unsafe { em.wakeup_interrupt(1 << intnum as usize) } {
                Ok(()) => {},
                Err(e) => {
                    error!("failed to wake up event manager: {:?}", e);
                },
            },
            Err(e) => {
                error!("failed to borrow event manager: {:?}", e);
            },
        },
        Err(e) => {
            error!("failed to get event manager: {:?}", e);
        },
    }
}

fn do_exception_handler(
    info: &ExceptionInformation,
    ctx: &mut ContextInformation,
) -> Result<(), SleepError> {
    trace!("info={:?}", info);

    // SAFETY: This is the only thread running, thus access to the memory manager is synchronized.
    let pid: ProcessIdentifier = unsafe { ProcessManager::get() }.get_pid();

    // Check if exception was triggered by the kernel.
    if pid == ProcessIdentifier::KERNEL {
        error!("{:?}", info);
        error!("{:?}", ctx);
        panic!("the kernel triggered an exception");
    }

    let tid: ThreadIdentifier = unsafe { ProcessManager::get() }.get_tid();

    // Handle page faults: demand-page user stack pages.
    if info.num() == ::arch::cpu::excp::Exception::PageFault as u32 {
        // SAFETY: This is the only thread running, thus access to the managers is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
        let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };

        let error_code: excp::ErrorCode = excp::ErrorCode::new(info.code());

        // Dispatch by the hardware-reported P (present) bit, which strictly partitions the
        // two handlers below:
        //
        //   - Copy-on-write fault: user-mode write to a *present* page with the AVL CoW bit
        //     set (P=1, W=1, U=1). The CoW handler accepts only when `error_code.is_present()`
        //     is true.
        //   - Stack demand-paging:  access to an *absent* page (P=0). The stack handler only
        //     fires when the page is not present.
        //
        // Routing on `is_present()` makes the disjointedness explicit and avoids relying on the
        // order of the two handlers.
        if error_code.is_present() {
            if pm
                .handle_cow_page_fault(mm, info.addr() as usize, error_code)
                .map_err(SleepError::Generic)?
            {
                return Ok(());
            }
        } else if pm
            .handle_stack_page_fault(mm, info.addr() as usize, error_code)
            .map_err(SleepError::Generic)?
        {
            return Ok(());
        }
    }

    // Handle FPU Exceptions.
    if info.num() == ::arch::cpu::excp::Exception::CoprocessorNotAvailable as u32 {
        // SAFETY: This is the only thread running, thus access to the process manager is synchronized.
        let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };

        // Handle FPU exception.
        pm.handle_fpu_exception().map_err(SleepError::Generic)?;

        return Ok(());
    }

    // Synchronous-signal precedence. The fault is now known to be unresolved by the kernel. If the
    // faulting vector maps to a signal and no exception owner has claimed it via `evctrl()`, and the
    // fault was taken in user mode, generate the signal on the faulting thread. An owner-claimed
    // vector (or a vector that maps to no signal, or a kernel-mode fault) falls through to the
    // owner-forwarding path below, which terminates the process when no owner exists.
    let vector: u32 = info.num();
    if let Some(signum) = exception_to_signal(vector) {
        let has_owner: bool = EventManager::get()
            .map_err(SleepError::Generic)?
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .exception_has_owner(vector);

        if !has_owner && ctx.returns_to_user() {
            // SAFETY: This is the only thread running, thus access to the process manager is
            // synchronized, and no reference to it is currently held.
            let pm: &mut ProcessManager = unsafe { ProcessManager::get_mut() };
            let cpu: SignalCpuContext = ctx.to_signal_context();
            match pm.try_deliver_synchronous_signal(signum, cpu) {
                SyncSignalOutcome::Delivered {
                    entry,
                    frame_top,
                    info_ptr,
                    ctx_ptr,
                } => {
                    // Redirect the faulting context into the handler; on return to user mode the
                    // thread enters the handler on its freshly built signal frame. The signal number
                    // and, for SA_SIGINFO handlers, the siginfo/context pointers are placed in the
                    // handler's argument registers (register-argument ABIs) or were already written
                    // to the frame (stack-argument ABIs).
                    ctx.redirect_to_signal_handler(entry, frame_top, signum, info_ptr, ctx_ptr);
                    return Ok(());
                },
                SyncSignalOutcome::Terminate => {
                    let reason: &str = "terminated by synchronous signal default action";
                    error!("{reason} (signum={signum}, pid={pid:?})");
                    return Err(SleepError::Generic(Error::new(ErrorCode::Interrupted, reason)));
                },
            }
        }
    }

    // SAFETY: the calling process does hold a mutable reference to the inner state of the process manager.
    let resume: Condvar = unsafe {
        EventManager::get()
            .map_err(SleepError::Generic)?
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .wakeup_exception(1 << info.num() as usize, pid, tid, info)
            .map_err(SleepError::Generic)?
    };

    // SAFETY: The calling thread is not the kernel and no resources are held.
    unsafe { resume.wait(None) }
}

fn exception_handler(info: &ExceptionInformation, ctx: &mut ContextInformation) {
    let _guard: ExceptionGuard = ProcessManager::enter_exception_handler();
    if let Err(sleep_error) = do_exception_handler(info, ctx) {
        let status: ErrorCode = match sleep_error {
            SleepError::Generic(generic_error) => {
                error!("killing process ({:?})", generic_error.reason);
                generic_error.code
            },
            SleepError::Interrupted(InterruptReason::Killed) => {
                error!("killing process (interrupted by signal)");
                ErrorCode::Interrupted
            },
            SleepError::Interrupted(InterruptReason::Signaled) => {
                // A caught signal interrupted a thread parked in exception handling. There is no
                // kernel call to restart on this path, so the process is torn down with EINTR.
                error!("killing process (interrupted by signal during exception handling)");
                ErrorCode::Interrupted
            },
            SleepError::Interrupted(InterruptReason::TimedOut) => {
                error!("killing process (timed out)");
                ErrorCode::OperationTimedOut
            },
        };

        error!("{info:?}");
        error!("{ctx:?}");

        // SAFETY: the calling process is not the kernel.
        unsafe {
            let error: Error = ProcessManager::exit(status.into()).unwrap_err();
            panic!("failed to exit() (error={:?})", error);
        }
    }
}

///
/// # Description
///
/// Initializes the global event manager, its pending queues, and the platform event handlers.
///
/// # Returns
///
/// Empty on success, or an error if a platform exception or interrupt handler cannot be
/// registered.
///
pub fn init() -> Result<(), Error> {
    // Allocate per-bit tables on the heap to keep this frame small; on 64-bit `usize::BITS`
    // doubles these tables, which would otherwise overflow the kernel boot stack-frame budget.
    let pending_interrupts: Box<[LinkedList<(EventSequence, EventDescriptor)>]> =
        (0..usize::BITS).map(|_| LinkedList::default()).collect();

    let interrupt_ownership: Box<[Option<ProcessIdentifier>]> =
        (0..usize::BITS).map(|_| None).collect();

    let pending_exceptions: Box<[LinkedList<PendingException>]> =
        (0..usize::BITS).map(|_| LinkedList::default()).collect();

    let exception_ownership: Box<[Option<ProcessIdentifier>]> =
        (0..usize::BITS).map(|_| None).collect();

    let scheduling_owner: Option<ProcessIdentifier> = None;

    let mut interrupt_capable: bool = true;

    // SAFETY: the hardware abstraction layer is initialized and access is synchronized.
    let hal: &mut Hal = unsafe { Hal::get_mut() };

    // TODO: add comments about safety.
    unsafe {
        hal.excpman().register_handler(exception_handler)?;
    }

    if let Some(intman) = hal.intman() {
        for intnum in InterruptNumber::VALUES {
            // Timer has a dedicated handler registered in pm::init().
            if intnum == InterruptNumber::Timer {
                continue;
            }
            // IKC has a dedicated handler registered in pm::init() (microvm only).
            // Unmask the interrupt here so the guest can receive IRQ 9 notifications.
            #[cfg(feature = "microvm")]
            if intnum == InterruptNumber::Ikc {
                if let Err(e) = intman.unmask(intnum) {
                    warn!("failed to unmask IKC interrupt: {:?}", e);
                }
                continue;
            }
            match intman.register_handler(intnum, interrupt_handler) {
                Ok(()) => {
                    if let Err(e) = intman.unmask(intnum) {
                        warn!("failed to mask interrupt: {:?}", e);
                    }
                },
                Err(e) => warn!("failed to register interrupt handler: {:?}", e),
            }
        }
    } else {
        warn!("no interrupt manager found, disabling interrupt support");
        interrupt_capable = false;
    }

    let em: RefCell<EventManagerInner> = RefCell::new(EventManagerInner {
        interrupt_capable,
        event_sequence: EventSequence::default(),
        delivery_epoch: DeliveryEpoch::default(),
        pending_interrupts,
        interrupt_ownership,
        pending_exceptions,
        exception_ownership,
        scheduling_owner,
        waiting_threads: VecDeque::new(),
        wait: Some(Condvar::new()),
    });

    let manager: EventManager = EventManager(em);

    unsafe {
        MANAGER = Some(manager);
    }

    Ok(())
}
