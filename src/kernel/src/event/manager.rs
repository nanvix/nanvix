// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::{
            ContextInformation,
            ExceptionInformation,
            InterruptNumber,
        },
        Hal,
    },
    mm::VirtMemoryManager,
    pm::{
        sync::condvar::Condvar,
        ExceptionGuard,
        InterruptReason,
        ProcessManager,
        SleepError,
    },
};
use ::alloc::collections::{
    LinkedList,
    VecDeque,
};
use ::arch::cpu::excp;
use ::core::{
    cell::{
        RefCell,
        RefMut,
    },
    mem,
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
        ProcessCreationInfo,
        ProcessTerminationInfo,
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
// Structures
//==================================================================================================

static mut MANAGER: Option<EventManager> = None;

/// Size, in bytes, of the payload carried by a scheduling-event notification. All scheduling
/// notifications serialize to a fixed-size payload.
const SCHEDULING_INFO_SIZE: usize = mem::size_of::<ProcessTerminationInfo>();
::static_assert::assert_eq_size!(ProcessCreationInfo, SCHEDULING_INFO_SIZE);

///
/// # Description
///
/// Payload carried by a pending scheduling-event notification. Each variant maps to a distinct
/// [`SchedulingEvent`] and is serialized into the corresponding [`MessageType`] when delivered.
///
enum SchedulingNotification {
    /// Process-termination notification.
    Termination(ProcessTerminationInfo),
    /// Process-creation notification.
    Creation(ProcessCreationInfo),
}

impl SchedulingNotification {
    /// Returns the [`SchedulingEvent`] that this notification corresponds to.
    fn event(&self) -> SchedulingEvent {
        match self {
            Self::Termination(_) => SchedulingEvent::ProcessTermination,
            Self::Creation(_) => SchedulingEvent::ProcessCreation,
        }
    }
}

struct ExceptionEventInformation {
    pid: ProcessIdentifier,
    tid: ThreadIdentifier,
    info: ExceptionInformation,
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
    em: &'static mut EventManager,
}

impl EventOwnership {
    pub fn event(&self) -> &Event {
        &self.ev
    }
}

impl Drop for EventOwnership {
    fn drop(&mut self) {
        match self.em.try_borrow_mut() {
            Ok(mut em) => match self.ev {
                Event::Interrupt(ev) => {
                    // SAFETY: This is the only thread running, thus access to the memory manager is synchronized.
                    let pm: &ProcessManager = unsafe { ProcessManager::get() };
                    if let Err(e) =
                        em.do_evctrl_interrupt(pm, None, ev, EventCtrlRequest::Unregister)
                    {
                        error!("failed to unregister interrupt: {:?}", e);
                    }
                },
                Event::Exception(ev) => {
                    // SAFETY: This is the only thread running, thus access to the memory manager is synchronized.
                    let pm: &ProcessManager = unsafe { ProcessManager::get() };
                    if let Err(e) =
                        em.do_evctrl_exception(pm, None, ev, EventCtrlRequest::Unregister)
                    {
                        error!("failed to unregister exception: {:?}", e);
                    }
                },
                Event::Scheduling(ev) => {
                    // SAFETY: This is the only thread running, thus access to the memory manager is synchronized.
                    let pm: &ProcessManager = unsafe { ProcessManager::get() };
                    if let Err(e) =
                        em.do_evctrl_scheduling(pm, None, ev, EventCtrlRequest::Unregister)
                    {
                        error!("failed to unregister scheduling event: {:?}", e);
                    }
                },
            },
            Err(e) => {
                error!("failed to borrow event manager: {:?}", e);
            },
        }
    }
}

struct EventManagerInner {
    interrupt_capable: bool,
    nevents: usize,
    wait: Option<Condvar>,
    waiting_threads: VecDeque<(ProcessIdentifier, ThreadIdentifier)>,
    interrupt_ownership: [Option<ProcessIdentifier>; InterruptEvent::NUMBER_EVENTS],
    pending_interrupts: [LinkedList<EventDescriptor>; InterruptEvent::NUMBER_EVENTS],
    exception_ownership: [Option<ProcessIdentifier>; ExceptionEvent::NUMBER_EVENTS],
    pending_exceptions: [LinkedList<(EventDescriptor, ExceptionEventInformation, Condvar)>;
        ExceptionEvent::NUMBER_EVENTS],
    scheduling_owner: Option<ProcessIdentifier>,
    pending_scheduling:
        [LinkedList<(EventDescriptor, SchedulingNotification)>; SchedulingEvent::NUMBER_EVENTS],
    /// Round-robin cursor selecting which scheduling sub-queue is scanned first on the next
    /// delivery. Advanced past a sub-queue each time an event is delivered from it, so successive
    /// deliveries alternate between sub-queues and neither process-creation nor process-termination
    /// events can starve the other.
    scheduling_cursor: usize,
}

impl EventManagerInner {
    const NUMBER_EVENTS: usize = 3;

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
    /// - `interrupts`: Bit mask of interrupts.
    /// - `exceptions`: Bit mask of exceptions.
    /// - `scheduling`: Bit mask of scheduling events.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the function returns the message that was delivered. Otherwise,
    /// an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn try_wait(
        &mut self,
        tid: ThreadIdentifier,
        pid: ProcessIdentifier,
        interrupts: usize,
        exceptions: usize,
        scheduling: usize,
    ) -> Result<Option<Message>, Error> {
        for i in 0..Self::NUMBER_EVENTS {
            // Check if any interrupts were triggered.
            if (self.nevents + i).is_multiple_of(Self::NUMBER_EVENTS) {
                // FIXME(#2558): starvation. This inner scan always starts at bit 0, so a
                // low-numbered interrupt can starve a high-numbered one. The FIFO-by-event-id
                // delivery tracked in #2558 would resolve this uniformly across event classes.
                for i in 0..InterruptEvent::NUMBER_EVENTS {
                    if (interrupts & (1 << i)) != 0 {
                        if let Some(_event) = self.pending_interrupts[i].pop_front() {
                            let message: Message = Message {
                                source: MessageSender::from(ProcessIdentifier::KERNEL),
                                destination: MessageReceiver::from(pid),
                                message_type: MessageType::Interrupt,
                                ..Message::default()
                            };
                            return Ok(Some(message));
                        }
                    }
                }
            }

            // Check if any exceptions were triggered.
            if ((self.nevents + i) % Self::NUMBER_EVENTS) == 1 {
                // FIXME(#2558): starvation. This inner scan always starts at bit 0, so a
                // low-numbered exception can starve a high-numbered one. The FIFO-by-event-id
                // delivery tracked in #2558 would resolve this uniformly across event classes.
                for i in 0..ExceptionEvent::NUMBER_EVENTS {
                    if (exceptions & (1 << i)) != 0 {
                        if let Some(entry) = self.pending_exceptions[i].pop_front() {
                            let mut info: EventInformation = EventInformation::default();
                            info.id = entry.0.clone();
                            info.pid = entry.1.pid;
                            info.number = Some(entry.1.info.num() as usize);
                            info.code = Some(entry.1.info.code() as usize);
                            info.address = Some(entry.1.info.addr() as usize);
                            info.instruction = Some(entry.1.info.instruction() as usize);

                            let mut message: Message = Message::from(info);
                            message.destination = MessageReceiver::from(pid);
                            message.message_type = MessageType::Exception;

                            self.pending_exceptions[i].push_back(entry);

                            return Ok(Some(message));
                        }
                    }
                }
            }

            // Check if any scheduling events were triggered.
            if ((self.nevents + i) % Self::NUMBER_EVENTS) == 2 {
                // Deliver scheduling events with a round-robin sub-queue scan rather than a fixed
                // priority, so neither process-creation nor process-termination events can starve
                // the other. The scan starts at `scheduling_cursor`, which is advanced past a
                // sub-queue each time an event is delivered from it (below). Because the cursor
                // advances on delivery rather than on event generation, successive deliveries
                // alternate between sub-queues even while a subscriber drains an already-queued
                // backlog with no new events being generated: a sustained stream of one event class
                // cannot indefinitely delay delivery of the other. The scan returns on the first
                // hit, so at most one scheduling event is delivered per call.
                //
                // Delivery order is an optimization, not a correctness requirement: this scan does
                // not prefer creations, so a termination can be delivered before a creation that is
                // queued at the same time (including on the first delivery, when the cursor still
                // starts at slot 0). `procd` records a child's lineage from its creation event;
                // if it instead observes an orphan termination first, it buffers the exit status in
                // `early_terminations` and reconciles it once the creation arrives. Publishing
                // creations ahead of terminations in the kernel main loop only biases delivery
                // toward the creation-first order and keeps that buffering window small in the
                // common case, while the round-robin here guarantees terminations are never starved
                // behind a continuous burst of creations (and vice versa).
                //
                // FIXME(#2558): this round-robin only approximates fairness. Replacing it with
                // FIFO delivery ordered by the global event id stamped on each queued entry would
                // be structurally starvation-free without cursor bookkeeping. Tracked as follow-up
                // work.
                for k in 0..SchedulingEvent::NUMBER_EVENTS {
                    let slot: usize = (self.scheduling_cursor + k) % SchedulingEvent::NUMBER_EVENTS;
                    if (scheduling & (1 << slot)) != 0 {
                        if let Some((_ev, notification)) = self.pending_scheduling[slot].pop_front()
                        {
                            // Derive the delivered message type and payload bytes from the
                            // notification variant.
                            let (message_type, info_bytes): (
                                MessageType,
                                [u8; SCHEDULING_INFO_SIZE],
                            ) = match notification {
                                SchedulingNotification::Termination(info) => {
                                    (MessageType::ProcessTerminationEvent, info.to_ne_bytes())
                                },
                                SchedulingNotification::Creation(info) => {
                                    (MessageType::ProcessCreationEvent, info.to_ne_bytes())
                                },
                            };

                            let message: Message = Message {
                                source: MessageSender::from(ProcessIdentifier::KERNEL),
                                destination: MessageReceiver::from(pid),
                                message_type,
                                status: 0,
                                payload: {
                                    let mut payload: [u8; Message::PAYLOAD_SIZE] =
                                        [0u8; Message::PAYLOAD_SIZE];
                                    payload[0..SCHEDULING_INFO_SIZE].copy_from_slice(&info_bytes);
                                    payload
                                },
                            };

                            // Advance the round-robin cursor past the sub-queue just delivered
                            // from, so the next delivery scans the other sub-queue first and the
                            // two sub-queues take turns.
                            self.scheduling_cursor = (slot + 1) % SchedulingEvent::NUMBER_EVENTS;

                            return Ok(Some(message));
                        }
                    }
                }
            }
        }

        // FIXME(#2558): Delivery of IPC messages will starve if the exception / interrupt rate is
        // too high. Unlike the in-class scan starvation above, this is cross-category starvation
        // (IPC vs. event classes); tracked alongside the other delivery-fairness work in #2558.

        // Check if any messages were delivered.
        match ProcessManager::try_recv(tid) {
            Ok(Some(message)) => Ok(Some(message)),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
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
            .position(|(pending_evdesc, _info, _resume)| *pending_evdesc == evdesc)
        {
            let (_eventinfo, excpinfo, resume) = self.pending_exceptions[idx].remove(entry);

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

        self.nevents += 1;
        let idx: usize = interrupts.trailing_zeros() as usize;
        let ev = Event::from(sys::event::InterruptEvent::try_from(idx)?);
        let eventid: EventDescriptor = EventDescriptor::new(self.nevents, ev);
        self.pending_interrupts[idx].push_back(eventid);

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
        self.nevents += 1;
        let idx: usize = exceptions.trailing_zeros() as usize;
        let ev: Event = Event::from(ExceptionEvent::try_from(idx)?);
        let eventid: EventDescriptor = EventDescriptor::new(self.nevents, ev);
        let resume: Condvar = Condvar::new();
        self.pending_exceptions[idx].push_back((
            eventid,
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

        match receiver.as_id() {
            Ok(pid) => {
                // SAFETY: the calling process does not hold mutable reference to the inner state of the process manager.
                unsafe { self.notify_all_process_threads(pid) }
            },
            Err(tid) => {
                // SAFETY: the calling process does not hold mutable reference to the inner state of the process manager.
                unsafe { self.get_wait().notify_thread(tid) }
            },
        }
    }

    ///
    /// # Description
    ///
    /// Notifies the event manager that a scheduling event has occurred, queuing the notification
    /// for delivery and waking up the threads of the owning process.
    ///
    /// # Parameters
    ///
    /// - `notification`: The scheduling-event notification to deliver.
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
    unsafe fn notify_scheduling(
        &mut self,
        notification: SchedulingNotification,
    ) -> Result<(), Error> {
        let event: SchedulingEvent = notification.event();
        let idx: usize = event as usize;

        // Buffer the notification for delivery. The queue is bounded by the maximum number of
        // processes so that pending entries cannot accumulate without limit while no subscriber
        // drains them; once the queue is full, further notifications are dropped.
        if self.pending_scheduling[idx].len() >= ::config::kernel::MAX_PROCESSES {
            let reason: &str = "scheduling-event queue is full";
            error!("reason={:?}, event={:?}", reason, event);
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }

        self.nevents += 1;
        let eventid: EventDescriptor = EventDescriptor::new(self.nevents, Event::from(event));
        self.pending_scheduling[idx].push_back((eventid, notification));

        // Wake the owning process if the scheduling-event class is currently owned. When there is
        // no owner yet, the notification stays buffered and is delivered once a subscriber
        // registers.
        match self.scheduling_owner {
            Some(pid) => {
                trace!("pid={:?}, event={:?}", pid, event);
                if let Err(e) = self.notify_all_process_threads(pid) {
                    // Waking the owner failed. Remove the notification we just buffered so that no
                    // error path of this function leaves a partially-delivered entry behind. This
                    // lets callers safely retry without risking a duplicate buffered event.
                    self.pending_scheduling[idx].pop_back();
                    return Err(e);
                }
            },
            None => {
                trace!("buffered scheduling event with no owner: event={:?}", event);
            },
        }

        Ok(())
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
    pub unsafe fn wait(
        tid: ThreadIdentifier,
        pid: ProcessIdentifier,
    ) -> Result<Message, SleepError> {
        // Get the interrupts that the process owns.
        let mut interrupts: usize = 0;
        for i in 0..InterruptEvent::NUMBER_EVENTS {
            if let Some(p) = EventManager::get()
                .map_err(SleepError::Generic)?
                .try_borrow_mut()
                .map_err(SleepError::Generic)?
                .interrupt_ownership[i]
            {
                if p == pid {
                    interrupts |= 1 << i;
                }
            }
        }

        // Get the exceptions that the process owns.
        let mut exceptions: usize = 0;
        for i in 0..ExceptionEvent::NUMBER_EVENTS {
            if let Some(p) = EventManager::get()
                .map_err(SleepError::Generic)?
                .try_borrow_mut()
                .map_err(SleepError::Generic)?
                .exception_ownership[i]
            {
                if p == pid {
                    exceptions |= 1 << i;
                }
            }
        }

        // Get the scheduling events that the process owns. Scheduling events are owned as a single
        // class, so the owner waits on every scheduling event at once.
        let mut scheduling: usize = 0;
        if let Some(p) = EventManager::get()
            .map_err(SleepError::Generic)?
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .scheduling_owner
        {
            if p == pid {
                scheduling = (1 << SchedulingEvent::NUMBER_EVENTS) - 1;
            }
        }

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
            let message: Option<Message> = EventManager::get()
                .map_err(SleepError::Generic)?
                .try_borrow_mut()
                .map_err(SleepError::Generic)?
                .try_wait(tid, pid, interrupts, exceptions, scheduling)
                .map_err(SleepError::Generic)?;

            if let Some(message) = message {
                break Ok(message);
            }

            // Wait for an event to be delivered.
            wait.wait(None)?;
        }
    }

    pub fn evctrl(
        pm: &mut ProcessManager,
        pid: ProcessIdentifier,
        ev: Event,
        req: EventCtrlRequest,
    ) -> Result<Option<EventOwnership>, Error> {
        trace!("ev={:?}, req={:?}", ev, req);

        let em: &'static mut EventManager = EventManager::get_mut()?;

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
                em.try_borrow_mut()?
                    .do_evctrl_scheduling(pm, Some(pid), scheduling_event, req)?
            },
        };

        match req {
            // Hand out an ownership guard only when ownership was newly acquired. An idempotent
            // re-registration by the current owner must not produce a guard, otherwise dropping it
            // would release the entire class while other guards (and the registration intent)
            // remain.
            EventCtrlRequest::Register if newly_acquired => Ok(Some(EventOwnership { ev, em })),
            EventCtrlRequest::Register => Ok(None),
            EventCtrlRequest::Unregister => Ok(None),
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
    /// Notifies the event manager that a process has terminated.
    ///
    /// # Parameters
    ///
    /// - `info`: Information about the process termination.
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
    pub unsafe fn notify_process_termination(info: ProcessTerminationInfo) -> Result<(), Error> {
        Self::get_mut()?
            .try_borrow_mut()?
            .notify_scheduling(SchedulingNotification::Termination(info))
    }

    ///
    /// # Description
    ///
    /// Notifies the event manager that a process has been created.
    ///
    /// # Parameters
    ///
    /// - `info`: Information about the process creation.
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
    pub unsafe fn notify_process_creation(info: ProcessCreationInfo) -> Result<(), Error> {
        Self::get_mut()?
            .try_borrow_mut()?
            .notify_scheduling(SchedulingNotification::Creation(info))
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
    ctx: &ContextInformation,
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

        // On x86_64, code() returns u64 while ErrorCode::new() takes u32.
        // The cast is architecturally correct (x86 error codes are 32-bit)
        // but redundant on x86 where code() already returns u32.
        #[allow(clippy::unnecessary_cast)]
        let error_code: excp::ErrorCode = excp::ErrorCode::new(info.code() as u32);

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

fn exception_handler(info: &ExceptionInformation, ctx: &ContextInformation) {
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

pub fn init() -> Result<(), Error> {
    let mut pending_interrupts: [LinkedList<EventDescriptor>; InterruptEvent::NUMBER_EVENTS] =
        unsafe { mem::zeroed() };
    for list in pending_interrupts.iter_mut() {
        *list = LinkedList::default();
    }

    let mut interrupt_ownership: [Option<ProcessIdentifier>; InterruptEvent::NUMBER_EVENTS] =
        unsafe { mem::zeroed() };
    for entry in interrupt_ownership.iter_mut() {
        *entry = None;
    }

    let mut pending_exceptions: [LinkedList<(EventDescriptor, ExceptionEventInformation, Condvar)>;
        ExceptionEvent::NUMBER_EVENTS] = unsafe { mem::zeroed() };
    for list in pending_exceptions.iter_mut() {
        *list = LinkedList::default();
    }

    let mut exception_ownership: [Option<ProcessIdentifier>; ExceptionEvent::NUMBER_EVENTS] =
        unsafe { mem::zeroed() };
    for entry in exception_ownership.iter_mut() {
        *entry = None;
    }

    let mut pending_scheduling: [LinkedList<(EventDescriptor, SchedulingNotification)>;
        SchedulingEvent::NUMBER_EVENTS] = unsafe { mem::zeroed() };
    for list in pending_scheduling.iter_mut() {
        *list = LinkedList::default();
    }

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
        nevents: 0,
        pending_interrupts,
        interrupt_ownership,
        pending_exceptions,
        exception_ownership,
        pending_scheduling,
        scheduling_owner,
        scheduling_cursor: 0,
        waiting_threads: VecDeque::new(),
        wait: Some(Condvar::new()),
    });

    let manager: EventManager = EventManager(em);

    unsafe {
        MANAGER = Some(manager);
    }

    Ok(())
}
