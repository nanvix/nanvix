// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    super::Message,
    identifier::RequestIdentifier,
    response_disposition::ResponseDisposition,
    state::{
        prepare_nested_request,
        with_active_request_state,
        with_request_state,
        RequestState,
        REQUEST_STATES,
    },
};
use crate::{
    error::Error,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::alloc::collections::VecDeque;

//==================================================================================================
// Structures
//==================================================================================================

/// Keeps a request identifier active until its response window closes.
pub struct RequestToken {
    owner: ThreadIdentifier,
    identifier: RequestIdentifier,
    responder: ProcessIdentifier,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RequestToken {
    /// Allocates and activates the next identifier for `owner` and `responder`.
    pub fn allocate(owner: ThreadIdentifier, responder: ProcessIdentifier) -> Result<Self, Error> {
        prepare_nested_request(owner)?;
        with_request_state(owner, |state| {
            let identifier: RequestIdentifier = state.allocate(responder)?;
            Ok(Self {
                owner,
                identifier,
                responder,
            })
        })
    }

    /// Rebinds an inherited token to the calling thread after `fork()`.
    ///
    /// A forked child inherits the parent's userspace request state, but the kernel assigns its
    /// only thread a new identifier. All other inherited state belongs to threads that do not
    /// exist in the child and is discarded before this token is activated under `owner`.
    ///
    /// # Safety
    ///
    /// This function must be called only in a freshly forked child, before it creates any other
    /// threads. Signal delivery must remain blocked, and the fork request-state guard must have
    /// been dropped in the child, so no signal handler or copied lock owner can access request state
    /// while inherited entries are reset.
    pub unsafe fn rebind_after_fork(mut self, owner: ThreadIdentifier) -> Result<Self, Error> {
        {
            let mut states = REQUEST_STATES.lock();
            for state in states.iter_mut() {
                *state = RequestState::new();
            }
        }
        self.owner = owner;
        with_request_state(owner, |state| state.activate(self.identifier, self.responder))?;
        Ok(self)
    }

    /// Activates a previously reserved identifier for `owner` and `responder`.
    pub fn activate(
        owner: ThreadIdentifier,
        identifier: RequestIdentifier,
        responder: ProcessIdentifier,
    ) -> Result<Self, Error> {
        prepare_nested_request(owner)?;
        with_request_state(owner, |state| {
            state.activate(identifier, responder)?;
            Ok(Self {
                owner,
                identifier,
                responder,
            })
        })
    }

    /// Returns this token's identifier.
    pub const fn identifier(&self) -> RequestIdentifier {
        self.identifier
    }

    /// Takes a response for this token from the thread's stash.
    pub fn take_stashed(&self) -> Result<Option<Message>, Error> {
        with_active_request_state(self.owner, self.identifier, |state| state.take(self.identifier))
    }

    /// Classifies a received response, stashing replies for other active requests.
    pub fn classify_response(&self, message: Message) -> Result<ResponseDisposition, Error> {
        with_active_request_state(self.owner, self.identifier, |state| {
            state.classify(self.identifier, message)
        })
    }

    /// Receives until a response for this token arrives, setting aside other active responses.
    pub fn receive_response_with(
        &self,
        mut receive: impl FnMut() -> Result<Message, Error>,
        mut log_stale: impl FnMut(RequestIdentifier),
        mut log_unexpected_source: impl FnMut(RequestIdentifier, ProcessIdentifier, ProcessIdentifier),
    ) -> Result<Message, Error> {
        if let Some(response) = self.take_stashed()? {
            return Ok(response);
        }

        loop {
            let response: Message = receive()?;
            match self.classify_response(response)? {
                ResponseDisposition::Matched(response) => return Ok(response),
                ResponseDisposition::Stashed => continue,
                ResponseDisposition::Stale(identifier) => log_stale(identifier),
                ResponseDisposition::UnexpectedSource {
                    identifier,
                    expected,
                    actual,
                } => log_unexpected_source(identifier, expected, actual),
            }
        }
    }
}

impl Drop for RequestToken {
    fn drop(&mut self) {
        let stash: Option<VecDeque<Message>> = {
            let mut states = REQUEST_STATES.lock();
            states
                .iter_mut()
                .find(|state| state.owner() == self.owner)
                .and_then(|state| state.release(self.identifier))
        };
        drop(stash);
    }
}
