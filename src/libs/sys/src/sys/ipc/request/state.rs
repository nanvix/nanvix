// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Manages synchronized request-correlation state for IPC client threads.
//!
//! Each active thread claims one [`RequestState`] entry in [`REQUEST_STATES`]. An entry tracks the
//! thread's active request identifiers, the process expected to answer each request, and responses
//! that arrived while a nested request was waiting. This permits signal handlers to issue IPC
//! requests without consuming responses that belong to the interrupted code.
//!
//! Access to the table is serialized by one spin mutex. Callbacks passed to
//! [`with_request_state`] and [`with_active_request_state`] execute while that mutex is held and
//! therefore must not allocate, deallocate, or otherwise re-enter request-state handling.
//! [`prepare_nested_request`] grows the response stash outside the critical section, while
//! [`RequestState::release`] and [`clear_request_state`] detach allocations so callers can drop
//! them after unlocking.
//!
//! A stash overflow poisons every request active in that thread until the final token is released.
//! Failing the complete response window prevents callers from silently continuing after one or
//! more out-of-order responses have been lost.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    super::Message,
    identifier::RequestIdentifier,
    identifier_allocator::RequestIdentifierAllocator,
    response_disposition::ResponseDisposition,
    MAX_ACTIVE_REQUESTS,
    RESPONSE_STASH_CAPACITY,
};
use crate::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::alloc::collections::VecDeque;
use ::spin::Mutex;

//==================================================================================================
// Structures
//==================================================================================================

/// Correlation state owned by one thread while it has active IPC requests.
pub(super) struct RequestState {
    /// Thread that owns this entry, or [`ThreadIdentifier::NONE`] when the entry is free.
    owner: ThreadIdentifier,
    /// Generates request identifiers for this owner.
    allocator: RequestIdentifierAllocator,
    /// Identifiers currently reserved by live request tokens.
    active: [RequestIdentifier; MAX_ACTIVE_REQUESTS],
    /// Process expected to answer each corresponding entry in `active`.
    responders: [ProcessIdentifier; MAX_ACTIVE_REQUESTS],
    /// Responses received for active requests other than the one currently waiting.
    stash: VecDeque<Message>,
    /// Maximum number of messages retained in `stash`.
    stash_capacity: usize,
    /// Whether a response was lost because `stash` reached its bound.
    overflowed: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RequestState {
    /// Creates an unclaimed request-state entry with no active requests.
    ///
    /// # Returns
    ///
    /// A request state that may be claimed by any thread.
    pub(super) const fn new() -> Self {
        Self {
            owner: ThreadIdentifier::NONE,
            allocator: RequestIdentifierAllocator::new(),
            active: [RequestIdentifier::NONE; MAX_ACTIVE_REQUESTS],
            responders: [ProcessIdentifier::KERNEL; MAX_ACTIVE_REQUESTS],
            stash: VecDeque::new(),
            stash_capacity: RESPONSE_STASH_CAPACITY,
            overflowed: false,
        }
    }

    /// Creates an unclaimed request state with a test-specific stash capacity.
    ///
    /// # Parameters
    ///
    /// - `stash_capacity`: Maximum number of out-of-order responses retained by the state.
    ///
    /// # Returns
    ///
    /// A request state whose stash has already reserved `stash_capacity` entries.
    #[cfg(test)]
    pub(super) fn with_stash_capacity(stash_capacity: usize) -> Self {
        let mut state: Self = Self::new();
        state.stash = VecDeque::with_capacity(stash_capacity);
        state.stash_capacity = stash_capacity;
        state
    }

    /// Replaces the next request-identifier candidate for a test.
    ///
    /// # Parameters
    ///
    /// - `next`: Raw identifier that the allocator should consider next.
    #[cfg(test)]
    pub(super) fn set_next_identifier(&mut self, next: u32) {
        self.allocator = RequestIdentifierAllocator::with_next(next);
    }

    /// Returns the thread that owns this request-state entry.
    ///
    /// # Returns
    ///
    /// The owning thread identifier, or [`ThreadIdentifier::NONE`] when the entry is free.
    pub(super) fn owner(&self) -> ThreadIdentifier {
        self.owner
    }

    /// Resets this entry and assigns it to `owner`.
    ///
    /// # Parameters
    ///
    /// - `owner`: Thread that will own the reset entry.
    fn claim(&mut self, owner: ThreadIdentifier) {
        *self = Self::new();
        self.owner = owner;
    }

    /// Tests whether this state has no active request identifiers.
    ///
    /// # Returns
    ///
    /// `true` when every active-request slot is free; otherwise, `false`.
    fn is_idle(&self) -> bool {
        self.active
            .iter()
            .all(|identifier| *identifier == RequestIdentifier::NONE)
    }

    /// Allocates and activates a request identifier for `responder`.
    ///
    /// Identifier candidates that remain active after allocator wraparound are skipped.
    ///
    /// # Parameters
    ///
    /// - `responder`: Process expected to send the response.
    ///
    /// # Returns
    ///
    /// The newly activated request identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::NoBufferSpace`] when every active-request slot is occupied.
    pub(super) fn allocate(
        &mut self,
        responder: ProcessIdentifier,
    ) -> Result<RequestIdentifier, Error> {
        if self
            .active
            .iter()
            .all(|identifier| *identifier != RequestIdentifier::NONE)
        {
            return Err(Error::new(ErrorCode::NoBufferSpace, "too many active IPC requests"));
        }

        loop {
            let identifier: RequestIdentifier = self.allocator.allocate();
            if !self.active.contains(&identifier) {
                self.activate(identifier, responder)?;
                return Ok(identifier);
            }
        }
    }

    /// Activates a specific request identifier for `responder`.
    ///
    /// # Parameters
    ///
    /// - `identifier`: Nonzero identifier to reserve.
    /// - `responder`: Process expected to send the response.
    ///
    /// # Returns
    ///
    /// Empty on success.
    ///
    /// # Errors
    ///
    /// - [`ErrorCode::InvalidArgument`] if `identifier` is reserved for uncorrelated messages.
    /// - [`ErrorCode::ResourceBusy`] if `identifier` is already active.
    /// - [`ErrorCode::NoBufferSpace`] if every active-request slot is occupied.
    pub(super) fn activate(
        &mut self,
        identifier: RequestIdentifier,
        responder: ProcessIdentifier,
    ) -> Result<(), Error> {
        if identifier == RequestIdentifier::NONE {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "request identifier zero is reserved",
            ));
        }
        if self.active.contains(&identifier) {
            return Err(Error::new(
                ErrorCode::ResourceBusy,
                "request identifier is already active",
            ));
        }
        let index: usize = self
            .active
            .iter()
            .position(|active| *active == RequestIdentifier::NONE)
            .ok_or_else(|| Error::new(ErrorCode::NoBufferSpace, "too many active IPC requests"))?;
        self.active[index] = identifier;
        self.responders[index] = responder;
        Ok(())
    }

    /// Releases an identifier and removes responses stashed for it.
    ///
    /// When this releases the final active identifier, it also clears the overflow state and
    /// detaches the stash allocation. The returned queue must be dropped after the request-state
    /// mutex is released.
    ///
    /// # Parameters
    ///
    /// - `identifier`: Identifier whose response window is closing.
    ///
    /// # Returns
    ///
    /// The detached stash when the state becomes idle; otherwise, `None`.
    pub(super) fn release(&mut self, identifier: RequestIdentifier) -> Option<VecDeque<Message>> {
        if let Some(index) = self.active.iter().position(|active| *active == identifier) {
            self.active[index] = RequestIdentifier::NONE;
            self.responders[index] = ProcessIdentifier::KERNEL;
        }
        self.stash
            .retain(|message| RequestIdentifier::read_from(message) != identifier);
        if self.is_idle() {
            self.overflowed = false;
            return Some(core::mem::take(&mut self.stash));
        }
        None
    }

    /// Removes the oldest stashed response for `identifier`.
    ///
    /// # Parameters
    ///
    /// - `identifier`: Active request whose response should be retrieved.
    ///
    /// # Returns
    ///
    /// The oldest matching response, or `None` if no response for `identifier` is stashed.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::NoBufferSpace`] when an earlier stash overflow poisoned the state.
    pub(super) fn take(&mut self, identifier: RequestIdentifier) -> Result<Option<Message>, Error> {
        if self.overflowed {
            return Err(Error::new(ErrorCode::NoBufferSpace, "IPC response stash overflowed"));
        }
        let response: Option<usize> = self
            .stash
            .iter()
            .position(|message| RequestIdentifier::read_from(message) == identifier);
        Ok(response.and_then(|index| self.stash.remove(index)))
    }

    /// Stashes a response when it belongs to an active request and came from its responder.
    ///
    /// # Parameters
    ///
    /// - `message`: Response to classify and potentially retain.
    ///
    /// # Returns
    ///
    /// `true` when `message` was stashed. Returns `false` for an inactive identifier, an
    /// uncorrelated message, or a response from a process other than the registered responder.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::NoBufferSpace`] when the bounded stash is full. In that case the stash
    /// is cleared and the state remains poisoned until its final active request is released.
    pub(super) fn stash(&mut self, message: Message) -> Result<bool, Error> {
        let identifier: RequestIdentifier = RequestIdentifier::read_from(&message);
        // Free slots in `active` hold `NONE`, so an uncorrelated message must never be looked up
        // there: it belongs to no request and is stale by definition.
        let Some(index): Option<usize> =
            self.active.iter().position(|active| *active == identifier)
        else {
            return Ok(false);
        };
        let source: ProcessIdentifier = message.source.pid;
        if identifier == RequestIdentifier::NONE || source != self.responders[index] {
            return Ok(false);
        }
        if self.stash.len() >= self.stash_capacity || self.stash.len() == self.stash.capacity() {
            self.stash.clear();
            self.overflowed = true;
            return Err(Error::new(ErrorCode::NoBufferSpace, "IPC response stash overflowed"));
        }
        self.stash.push_back(message);
        Ok(true)
    }

    /// Classifies a response relative to the request currently waiting for it.
    ///
    /// Responses for other active requests are stashed, while inactive identifiers and unexpected
    /// sources are reported to the caller without being retained.
    ///
    /// # Parameters
    ///
    /// - `expected`: Identifier of the request currently waiting for a response.
    /// - `message`: Response received from the thread's mailbox.
    ///
    /// # Returns
    ///
    /// A [`ResponseDisposition`] describing whether the response matched, was stashed, was stale,
    /// or came from an unexpected process.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorCode::NoBufferSpace`] if the state was already poisoned or stashing this
    /// response overflows its bounded queue.
    pub(super) fn classify(
        &mut self,
        expected: RequestIdentifier,
        message: Message,
    ) -> Result<ResponseDisposition, Error> {
        if self.overflowed {
            return Err(Error::new(ErrorCode::NoBufferSpace, "IPC response stash overflowed"));
        }
        let identifier: RequestIdentifier = RequestIdentifier::read_from(&message);
        let Some(index): Option<usize> =
            self.active.iter().position(|active| *active == identifier)
        else {
            return Ok(ResponseDisposition::Stale(identifier));
        };
        let actual: ProcessIdentifier = message.source.pid;
        let expected_source: ProcessIdentifier = self.responders[index];
        if actual != expected_source {
            return Ok(ResponseDisposition::UnexpectedSource {
                identifier,
                expected: expected_source,
                actual,
            });
        }
        if identifier == expected {
            return Ok(ResponseDisposition::Matched(message));
        }
        if self.stash(message)? {
            Ok(ResponseDisposition::Stashed)
        } else {
            Ok(ResponseDisposition::Stale(identifier))
        }
    }
}

//==================================================================================================
// Global Variables
//==================================================================================================

/// Fixed request-state table shared by all threads in the current process image.
///
/// The table has one possible entry per kernel thread. Every access must hold this mutex, and code
/// running under the lock must not allocate, deallocate, or re-enter request-state handling.
pub(super) static REQUEST_STATES: Mutex<[RequestState; ::config::kernel::MAX_THREADS]> =
    Mutex::new([const { RequestState::new() }; ::config::kernel::MAX_THREADS]);

//==================================================================================================
// Functions
//==================================================================================================

/// Runs `callback` with the request state owned by `owner`.
///
/// A free table entry is claimed when `owner` has no existing state. The request-state mutex stays
/// locked for the full callback invocation.
///
/// # Parameters
///
/// - `owner`: Thread whose state should be retrieved or claimed.
/// - `callback`: Operation to execute while holding exclusive access to that state.
///
/// # Returns
///
/// The value returned by `callback`.
///
/// # Errors
///
/// Returns [`ErrorCode::NoBufferSpace`] when the table has no free entry, or propagates an error
/// returned by `callback`.
pub(super) fn with_request_state<T>(
    owner: ThreadIdentifier,
    callback: impl FnOnce(&mut RequestState) -> Result<T, Error>,
) -> Result<T, Error> {
    let mut states = REQUEST_STATES.lock();
    let index: usize = states
        .iter()
        .position(|state| state.owner == owner)
        .or_else(|| states.iter().position(|state| state.owner.is_none()))
        .ok_or_else(|| Error::new(ErrorCode::NoBufferSpace, "no IPC request state available"))?;
    if states[index].owner != owner {
        states[index].claim(owner);
    }
    callback(&mut states[index])
}

/// Runs `callback` with the state containing an active request identifier.
///
/// Unlike [`with_request_state`], this function never claims a free entry. The request-state mutex
/// stays locked for the full callback invocation.
///
/// # Parameters
///
/// - `owner`: Thread expected to own the request state.
/// - `identifier`: Identifier that must still be active in that state.
/// - `callback`: Operation to execute while holding exclusive access to the state.
///
/// # Returns
///
/// The value returned by `callback`.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] when `owner` has no state or `identifier` is inactive,
/// or propagates an error returned by `callback`.
pub(super) fn with_active_request_state<T>(
    owner: ThreadIdentifier,
    identifier: RequestIdentifier,
    callback: impl FnOnce(&mut RequestState) -> Result<T, Error>,
) -> Result<T, Error> {
    let mut states = REQUEST_STATES.lock();
    let state: &mut RequestState = states
        .iter_mut()
        .find(|state| state.owner == owner)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "IPC request state is inactive"))?;
    if !state.active.contains(&identifier) {
        return Err(Error::new(ErrorCode::InvalidArgument, "IPC request token is inactive"));
    }
    callback(state)
}

/// Reserves the response stash before a nested request becomes active.
///
/// Allocation happens without holding `REQUEST_STATES`, so a signal delivered while the allocator
/// grows cannot re-enter the request matcher and deadlock on its own state lock.
///
/// # Parameters
///
/// - `owner`: Thread that is about to activate a nested request.
///
/// # Returns
///
/// Empty when no reservation is needed or the full stash capacity is installed.
///
/// # Errors
///
/// Returns [`ErrorCode::NoBufferSpace`] if request state is unavailable or the stash reservation
/// cannot be allocated. Other request-state errors are propagated.
pub(super) fn prepare_nested_request(owner: ThreadIdentifier) -> Result<(), Error> {
    let capacity: Option<usize> = with_request_state(owner, |state| {
        if state.is_idle() || state.stash.capacity() >= state.stash_capacity {
            Ok(None)
        } else {
            Ok(Some(state.stash_capacity))
        }
    })?;
    let Some(capacity): Option<usize> = capacity else {
        return Ok(());
    };

    let mut replacement: VecDeque<Message> = VecDeque::new();
    replacement.try_reserve_exact(capacity).map_err(|_| {
        Error::new(ErrorCode::NoBufferSpace, "failed to reserve IPC response stash")
    })?;
    let mut replacement: Option<VecDeque<Message>> = Some(replacement);
    let displaced: Option<VecDeque<Message>> = with_request_state(owner, |state| {
        if !state.is_idle() && state.stash.capacity() < state.stash_capacity {
            debug_assert!(state.stash.is_empty(), "an undersized response stash must be empty");
            let Some(replacement): Option<VecDeque<Message>> = replacement.take() else {
                return Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "replacement IPC response stash is unavailable",
                ));
            };
            Ok(Some(core::mem::replace(&mut state.stash, replacement)))
        } else {
            Ok(None)
        }
    })?;

    // A nested signal may have installed another queue while this allocation was in progress.
    // Drop whichever allocations were not retained only after releasing the request-state lock.
    drop(displaced);
    drop(replacement);
    Ok(())
}

/// Clears all request state owned by one thread.
///
/// Any stash allocation is detached while holding the mutex and dropped after unlocking so its
/// deallocation cannot re-enter request-state handling from a signal handler.
///
/// # Parameters
///
/// - `owner`: Thread whose request state should be cleared.
pub fn clear_request_state(owner: ThreadIdentifier) {
    let state: Option<RequestState> = {
        let mut states = REQUEST_STATES.lock();
        states
            .iter_mut()
            .find(|state| state.owner == owner)
            .map(|state| core::mem::replace(state, RequestState::new()))
    };
    drop(state);
}

/// Tests whether a thread has any request awaiting a response.
///
/// # Parameters
///
/// - `owner`: Thread whose request state should be inspected.
///
/// # Returns
///
/// `true` when `owner` has at least one active request; otherwise, `false`.
pub fn has_active_requests(owner: ThreadIdentifier) -> bool {
    let states = REQUEST_STATES.lock();
    states
        .iter()
        .find(|state| state.owner == owner)
        .is_some_and(|state| !state.is_idle())
}
