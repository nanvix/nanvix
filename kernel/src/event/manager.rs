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
    pm::{
        sync::condvar::Condvar,
        ProcessManager,
    },
};
use ::alloc::{
    collections::LinkedList,
    rc::Rc,
};
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
    },
    ipc::Message,
    pm::{
        Capability,
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

static mut MANAGER: EventManager = unsafe { mem::zeroed() };

struct ExceptionEventInformation {
    pid: ProcessIdentifier,
    info: ExceptionInformation,
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
                    if let Err(e) =
                        em.do_evctrl_interrupt(None, None, ev, EventCtrlRequest::Unregister)
                    {
                        error!("failed to unregister interrupt: {:?}", e);
                    }
                },
                Event::Exception(ev) => {
                    if let Err(e) =
                        em.do_evctrl_exception(None, None, ev, EventCtrlRequest::Unregister)
                    {
                        error!("failed to unregister exception: {:?}", e);
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
    nevents: usize,
    wait: Option<Rc<Condvar>>,
    interrupt_ownership: [Option<ThreadIdentifier>; usize::BITS as usize],
    pending_interrupts: [LinkedList<EventDescriptor>; usize::BITS as usize],
    exception_ownership: [Option<ThreadIdentifier>; usize::BITS as usize],
    pending_exceptions: [LinkedList<(EventDescriptor, ExceptionEventInformation, Rc<Condvar>)>;
        usize::BITS as usize],
}

impl EventManagerInner {
    const NUMBER_EVENTS: usize = 2;

    fn do_evctrl_interrupt(
        &mut self,
        pid: Option<ProcessIdentifier>,
        tid: Option<ThreadIdentifier>,
        ev: InterruptEvent,
        req: EventCtrlRequest,
    ) -> Result<(), Error> {
        // Check if target interrupt is already owned by another process.
        let idx: usize = usize::from(ev);
        if self.interrupt_ownership[idx].is_some() {
            let reason: &str = "interrupt is already owned by another process";
            error!("do_evctrl_interrupt(): reason={:?}", reason);
            return Err(Error::new(ErrorCode::ResourceBusy, &reason));
        }

        // Handle request.
        match req {
            EventCtrlRequest::Register => {
                // Check if PID is valid.
                if let Some(pid) = pid {
                    // Check if TID is valid.
                    if let Some(tid) = tid {
                        // Ensure that the process has the required capabilities.
                        if !ProcessManager::has_capability(pid, Capability::InterruptControl)? {
                            let reason: &str = "process does not have interrupt control capability";
                            error!("do_evctrl_interrupt(): reason={:?}", reason);
                            return Err(Error::new(ErrorCode::PermissionDenied, &reason));
                        }

                        // Check if target interrupt is already owned by another process.
                        if self.interrupt_ownership[idx].is_some() {
                            let reason: &str = "interrupt is already owned by another process";
                            error!("do_evctrl_interrupt(): reason={:?}", reason);
                            return Err(Error::new(ErrorCode::ResourceBusy, &reason));
                        }

                        // Register interrupt.
                        self.interrupt_ownership[idx] = Some(tid);

                        return Ok(());
                    }

                    let reason: &str = "invalid thread identifier";
                    error!("do_evctrl_interrupt(): reason={:?}", reason);
                    return Err(Error::new(ErrorCode::InvalidArgument, &reason));
                }

                let reason: &str = "invalid process identifier";
                error!("do_evctrl_interrupt(): reason={:?}", reason);
                Err(Error::new(ErrorCode::InvalidArgument, &reason))
            },
            EventCtrlRequest::Unregister => {
                // If PID was supplied, check if it matches the current owner.
                if let Some(tid) = tid {
                    if self.interrupt_ownership[idx] != Some(tid) {
                        let reason: &str = "process does not own interrupt";
                        error!("do_evctrl_interrupt(): reason={:?}", reason);
                        return Err(Error::new(ErrorCode::PermissionDenied, &reason));
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
        pid: Option<ProcessIdentifier>,
        tid: Option<ThreadIdentifier>,
        ev: ExceptionEvent,
        req: EventCtrlRequest,
    ) -> Result<(), Error> {
        let idx: usize = usize::from(ev);

        // Handle request.
        match req {
            EventCtrlRequest::Register => {
                // Check if PID is valid.
                if let Some(pid) = pid {
                    if let Some(tid) = tid {
                        // Ensure that the process has the required capabilities.
                        if !ProcessManager::has_capability(pid, Capability::ExceptionControl)? {
                            let reason: &str = "process does not have exception control capability";
                            error!("do_evctrl_exception(): reason={:?}", reason);
                            return Err(Error::new(ErrorCode::PermissionDenied, &reason));
                        }

                        // Check if target exception is already owned by another process.
                        if self.exception_ownership[idx].is_some() {
                            let reason: &str = "exception is already owned by another process";
                            error!("do_evctrl_exception(): reason={:?}", reason);
                            return Err(Error::new(ErrorCode::ResourceBusy, &reason));
                        }

                        // Register exception.
                        self.exception_ownership[idx] = Some(tid);

                        return Ok(());
                    }

                    let reason: &str = "invalid thread identifier";
                    error!("do_evctrl_exception(): reason={:?}", reason);
                    return Err(Error::new(ErrorCode::InvalidArgument, &reason));
                }

                let reason: &str = "invalid process identifier";
                error!("do_evctrl_exception(): reason={:?}", reason);
                Err(Error::new(ErrorCode::InvalidArgument, &reason))
            },
            EventCtrlRequest::Unregister => {
                // If PID was supplied, check if it matches the current owner.
                if let Some(pid) = tid {
                    if self.exception_ownership[idx] != Some(pid) {
                        let reason: &str = "process does not own exception";
                        error!("do_evctrl_exception(): reason={:?}", reason);
                        return Err(Error::new(ErrorCode::PermissionDenied, &reason));
                    }
                }

                // Unregister exception.
                self.exception_ownership[idx] = None;

                Ok(())
            },
        }
    }

    pub fn try_wait(
        &mut self,
        interrupts: usize,
        exceptions: usize,
    ) -> Result<Option<Message>, Error> {
        for i in 0..Self::NUMBER_EVENTS {
            // Check if any interrupts were triggered.
            if ((self.nevents + i) % Self::NUMBER_EVENTS) == 0 {
                // FIXME: starvation.
                for i in 0..usize::BITS {
                    if (interrupts & (1 << i)) != 0 {
                        let idx: usize = i as usize;
                        if let Some(_event) = self.pending_interrupts[idx].pop_front() {
                            return Ok(Some(Message::default()));
                        }
                    }
                }
            }

            // Check if any exceptions were triggered.
            if ((self.nevents + i) % Self::NUMBER_EVENTS) == 1 {
                // FIXME: starvation.
                for i in 0..usize::BITS {
                    if (exceptions & (1 << i)) != 0 {
                        let idx: usize = i as usize;
                        if let Some(entry) = self.pending_exceptions[idx].pop_front() {
                            let mut info: EventInformation = EventInformation::default();
                            info.id = entry.0.clone();
                            info.pid = entry.1.pid;
                            info.number = Some(entry.1.info.num() as usize);
                            info.code = Some(entry.1.info.code() as usize);
                            info.address = Some(entry.1.info.addr() as usize);
                            info.instruction = Some(entry.1.info.instruction() as usize);

                            let mut message: Message = Message::default();
                            let size: usize = core::mem::size_of::<EventInformation>();

                            let src = unsafe {
                                ::core::slice::from_raw_parts(
                                    &info as *const EventInformation as *const u8,
                                    size,
                                )
                            };

                            // Copy bytes from src to message payload.
                            for (i, byte) in src.iter().enumerate() {
                                message.payload[i] = *byte;
                            }

                            self.pending_exceptions[idx].push_back(entry);

                            return Ok(Some(message));
                        }
                    }
                }
            }

            // Check if any messages were delivered.
            match ProcessManager::try_recv() {
                Ok(Some(message)) => return Ok(Some(message)),
                Ok(None) => {},
                Err(e) => return Err(e),
            }
        }

        Ok(None)
    }

    fn resume_exception(&mut self, ev: ExceptionEvent) -> Result<(), Error> {
        let idx: usize = usize::from(ev);

        let is_pending_exception = |evdesc: &EventDescriptor, ev: &ExceptionEvent| -> bool {
            match evdesc.event() {
                Event::Exception(ev2) => &ev2 == ev,
                _ => false,
            }
        };

        // Get exception owner.
        let tid: ThreadIdentifier = match self.exception_ownership[idx] {
            Some(owner) => owner,
            None => {
                let reason: &str = "no owner for exception";
                error!("resume_exception(): reason={:?}", reason);
                unimplemented!("terminate process")
            },
        };

        // Search and remove event from pending exceptions.
        if let Some(entry) = self.pending_exceptions[idx]
            .iter()
            .position(|(evdesc, _info, _resume)| is_pending_exception(evdesc, &ev))
        {
            let (_enventinfo, _excpinfo, resume) = self.pending_exceptions[idx].remove(entry);

            if let Err(e) = resume.notify(tid) {
                warn!("failed to notify all: {:?}", e);
                unimplemented!("terminate process")
            }
        }

        Ok(())
    }

    fn wakeup_interrupt(&mut self, interrupts: usize) -> Result<(), Error> {
        self.nevents += 1;
        let idx: usize = interrupts.trailing_zeros() as usize;
        let ev = Event::from(sys::event::InterruptEvent::try_from(idx)?);
        let eventid: EventDescriptor = EventDescriptor::new(self.nevents, ev);
        self.pending_interrupts[idx].push_back(eventid);

        // Get interrupt owner.
        let tid: ThreadIdentifier = match self.interrupt_ownership[idx] {
            Some(owner) => owner,
            None => {
                let reason: &str = "no owner for interrupt";
                error!("wakeup_interrupt(): reason={:?}", reason);
                return Err(Error::new(ErrorCode::NoSuchProcess, &reason));
            },
        };

        self.get_wait().notify(tid)
    }

    fn wakeup_exception(
        &mut self,
        exceptions: usize,
        pid: ProcessIdentifier,
        info: &ExceptionInformation,
    ) -> Result<Rc<Condvar>, Error> {
        trace!("wakeup_exception(): exceptions={:#x}, pid={:?}, info={:?}", exceptions, pid, info);
        self.nevents += 1;
        let idx: usize = exceptions.trailing_zeros() as usize;
        let ev = Event::from(ExceptionEvent::try_from(idx)?);
        let eventid: EventDescriptor = EventDescriptor::new(self.nevents, ev);
        let resume: Rc<Condvar> = Rc::new(Condvar::new());
        self.pending_exceptions[idx].push_back((
            eventid,
            ExceptionEventInformation {
                pid,
                info: info.clone(),
            },
            resume.clone(),
        ));

        // Get exception owner.
        let tid: ThreadIdentifier = match self.exception_ownership[idx] {
            Some(owner) => owner,
            None => {
                let reason: &str = "no owner for exception";
                error!("wakeup_exception(): reason={:?}", reason);
                return Err(Error::new(ErrorCode::NoSuchProcess, &reason));
            },
        };

        trace!("wakeup_exception(): tid={:?}", tid);
        let _ = self.get_wait().notify(tid);

        Ok(resume)
    }

    fn post_message(
        &mut self,
        pm: &mut ProcessManager,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        message: Message,
    ) -> Result<(), Error> {
        pm.post_message(pid, message)?;

        self.get_wait().notify(tid)
    }

    fn get_wait(&self) -> &Rc<Condvar> {
        // NOTE: it is safe to unwrap because the wait field is always Some.
        self.wait.as_ref().unwrap()
    }
}

//==================================================================================================
// Event Manager
//==================================================================================================

pub struct EventManager(RefCell<EventManagerInner>);

impl EventManager {
    pub fn resume(evdesc: EventDescriptor) -> Result<(), Error> {
        trace!("do_resume(): evdesc={:?}", evdesc);
        match evdesc.event() {
            Event::Interrupt(_ev) => {
                // No further action is required for interrupts.
                Ok(())
            },
            Event::Exception(ev) => {
                return EventManager::get().try_borrow_mut()?.resume_exception(ev);
            },
        }
    }

    pub fn wait() -> Result<Message, Error> {
        trace!("do_wait()");

        let mytid: ThreadIdentifier = ProcessManager::get_tid()?;

        // Get the interrupts that the process owns.
        let mut interrupts: usize = 0;
        for i in 0..usize::BITS {
            let idx: usize = i as usize;
            if let Some(tid) = EventManager::get().try_borrow_mut()?.interrupt_ownership[idx] {
                if tid == mytid {
                    interrupts |= 1 << i;
                }
            }
        }

        // Get the exceptions that the process owns.
        let mut exceptions: usize = 0;
        for i in 0..usize::BITS {
            let idx: usize = i as usize;
            if let Some(tid) = EventManager::get().try_borrow_mut()?.exception_ownership[idx] {
                if tid == mytid {
                    exceptions |= 1 << i;
                }
            }
        }

        let wait: Rc<Condvar> = EventManager::get().try_borrow_mut()?.get_wait().clone();

        loop {
            let message: Option<Message> = EventManager::get()
                .try_borrow_mut()?
                .try_wait(interrupts, exceptions)?;

            if let Some(message) = message {
                break Ok(message);
            }

            wait.wait()?;
        }
    }

    pub fn evctrl(
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        ev: Event,
        req: EventCtrlRequest,
    ) -> Result<Option<EventOwnership>, Error> {
        trace!("do_evctrl(): ev={:?}, req={:?}", ev, req);

        let em: &'static mut EventManager = EventManager::get_mut();

        match ev {
            Event::Interrupt(interrupt_event) => {
                em.try_borrow_mut()?.do_evctrl_interrupt(
                    Some(pid),
                    Some(tid),
                    interrupt_event,
                    req,
                )?;
            },
            Event::Exception(exception_event) => {
                em.try_borrow_mut()?.do_evctrl_exception(
                    Some(pid),
                    Some(tid),
                    exception_event,
                    req,
                )?;
            },
        }

        match req {
            EventCtrlRequest::Register => Ok(Some(EventOwnership { ev, em })),
            EventCtrlRequest::Unregister => Ok(None),
        }
    }

    pub fn post_message(
        pm: &mut ProcessManager,
        pid: ProcessIdentifier,
        message: Message,
    ) -> Result<(), Error> {
        let tid: ThreadIdentifier = ThreadIdentifier::from(usize::from(pid));
        Self::get_mut()
            .try_borrow_mut()?
            .post_message(pm, pid, tid, message)
    }

    fn try_borrow_mut(&self) -> Result<RefMut<EventManagerInner>, Error> {
        match self.0.try_borrow_mut() {
            Ok(em) => Ok(em),
            Err(e) => {
                let reason: &str = "failed to borrow event manager";
                error!("try_borrow_mut(): {:?} (error={:?})", reason, e);
                Err(Error::new(ErrorCode::PermissionDenied, &reason))
            },
        }
    }

    fn get() -> &'static EventManager {
        unsafe { &MANAGER }
    }

    fn get_mut() -> &'static mut EventManager {
        unsafe { &mut MANAGER }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn interrupt_handler(intnum: InterruptNumber) {
    trace!("interrupt_handler(): intnum={:?}", intnum);
    if let Ok(mut em) = EventManager::get().try_borrow_mut() {
        match em.wakeup_interrupt(1 << intnum as usize) {
            Ok(_) => {},
            Err(e) => error!("failed to wake up event manager: {:?}", e),
        }
    }
}

fn exception_handler(info: &ExceptionInformation, _ctx: &ContextInformation) {
    trace!("exception_handler(): info={:?}", info);
    let pid: ProcessIdentifier = match ProcessManager::get_pid() {
        Ok(pid) => pid,
        Err(e) => {
            error!("failed to get process identifier: {:?}", e);
            return;
        },
    };

    let resume: Rc<Condvar> = match EventManager::get().try_borrow_mut() {
        Ok(mut em) => match em.wakeup_exception(1 << info.num() as usize, pid, info) {
            Ok(resume) => resume,
            Err(e) => {
                error!("failed to wake up event manager: {:?}", e);
                return;
            },
        },
        Err(e) => {
            error!("failed to borrow event manager: {:?}", e);
            return;
        },
    };

    if let Err(_) = resume.wait() {
        if let Err(e) = ProcessManager::exit(-1) {
            unreachable!("failed to terminate process (error={:?})", e);
        }
    }
}

pub fn init(hal: &mut Hal) {
    let mut pending_interrupts: [LinkedList<EventDescriptor>; usize::BITS as usize] =
        unsafe { mem::zeroed() };
    for list in pending_interrupts.iter_mut() {
        *list = LinkedList::default();
    }

    let mut interrupt_ownership: [Option<ThreadIdentifier>; usize::BITS as usize] =
        unsafe { mem::zeroed() };
    for entry in interrupt_ownership.iter_mut() {
        *entry = None;
    }

    let mut pending_exceptions: [LinkedList<(
        EventDescriptor,
        ExceptionEventInformation,
        Rc<Condvar>,
    )>; usize::BITS as usize] = unsafe { mem::zeroed() };
    for list in pending_exceptions.iter_mut() {
        *list = LinkedList::default();
    }

    let mut exception_ownership: [Option<ThreadIdentifier>; usize::BITS as usize] =
        unsafe { mem::zeroed() };
    for entry in exception_ownership.iter_mut() {
        *entry = None;
    }

    let em: RefCell<EventManagerInner> = RefCell::new(EventManagerInner {
        nevents: 0,
        pending_interrupts,
        interrupt_ownership,
        pending_exceptions,
        exception_ownership,
        wait: Some(Rc::new(Condvar::new())),
    });

    unsafe {
        MANAGER = EventManager(em);
    }

    hal.excpman.register_handler(exception_handler);

    for intnum in InterruptNumber::VALUES {
        if intnum == InterruptNumber::Timer {
            continue;
        }
        match hal.intman.register_handler(intnum, interrupt_handler) {
            Ok(()) => {
                if let Err(e) = hal.intman.unmask(intnum) {
                    warn!("failed to mask interrupt: {:?}", e);
                }
            },
            Err(e) => warn!("failed to register interrupt handler: {:?}", e),
        }
    }
}
