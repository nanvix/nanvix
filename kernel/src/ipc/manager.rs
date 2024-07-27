// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//  Imports
//==================================================================================================

use crate::pm::{
    sync::condvar::Condvar,
    ProcessManager,
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
    ipc::Message,
    pm::ProcessIdentifier,
};

//==================================================================================================
// IPC Manager Inner
//==================================================================================================

struct IpcManagerInner {
    waiting: LinkedList<(ProcessIdentifier, Rc<Condvar>)>,
}

impl IpcManagerInner {
    fn new() -> Self {
        Self {
            waiting: LinkedList::new(),
        }
    }

    fn receive_message(&mut self, pid: ProcessIdentifier) -> Result<Message, Error> {
        // Check if process is already waiting for a message.
        for (p, _) in &self.waiting {
            if p == &pid {
                let reason: &str = "process is already waiting for a message";
                error!("wait() {}", reason);
                return Err(Error::new(ErrorCode::OperationAlreadyInProgress, reason));
            }
        }

        loop {
            match ProcessManager::try_recv() {
                Ok(Some(message)) => {
                    break Ok(message);
                },
                Ok(None) => {},
                Err(e) => break Err(e),
            }

            let condvar: Rc<Condvar> = Rc::new(Condvar::new());
            self.waiting.push_back((pid, condvar.clone()));

            if let Err(e) = condvar.wait() {
                break Err(e);
            }

            self.waiting.retain(|(p, _)| p != &pid);
        }
    }

    fn post_message(
        &mut self,
        pm: &mut ProcessManager,
        pid: ProcessIdentifier,
        message: Message,
    ) -> Result<(), Error> {
        pm.post_message(pid, message)?;

        // Wake up process.
        for (p, c) in &self.waiting {
            if p == &pid {
                c.notify()?;
                break;
            }
        }

        Ok(())
    }
}

//==================================================================================================
// IPC Manager
//==================================================================================================

pub struct IpcManager(RefCell<IpcManagerInner>);

static mut IPC_MANAGER: IpcManager = unsafe { mem::zeroed() };

impl IpcManager {
    pub fn receive_message(pid: ProcessIdentifier) -> Result<Message, Error> {
        Self::get_mut().try_borrow_mut()?.receive_message(pid)
    }

    pub fn post_message(
        pm: &mut ProcessManager,
        pid: ProcessIdentifier,
        message: Message,
    ) -> Result<(), Error> {
        Self::get_mut()
            .try_borrow_mut()?
            .post_message(pm, pid, message)
    }

    fn try_borrow_mut(&self) -> Result<RefMut<IpcManagerInner>, Error> {
        match self.0.try_borrow_mut() {
            Ok(inner) => Ok(inner),
            Err(_) => {
                let reason: &str = "IPC manager is already borrowed";
                error!("try_borrow_mut() {}", reason);
                Err(Error::new(ErrorCode::OperationAlreadyInProgress, reason))
            },
        }
    }

    fn get_mut() -> &'static mut IpcManager {
        unsafe { &mut IPC_MANAGER }
    }

    pub fn init() {
        unsafe {
            IPC_MANAGER = IpcManager(RefCell::new(IpcManagerInner::new()));
        }
    }
}
