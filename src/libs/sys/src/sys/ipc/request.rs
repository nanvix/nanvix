// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::Message;
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
// Constants
//==================================================================================================

/// Maximum number of requests that may be active in one thread.
pub const MAX_ACTIVE_REQUESTS: usize = 4;

/// Maximum number of out-of-order responses that may be held for one thread.
///
/// The largest current response stream is `getdents()`: 1024 maximum-sized directory entries use
/// fewer than 9K message parts. A thread can have at most three other requests active while it
/// waits for the fourth, so 32K entries retain every valid combination with room for framing
/// changes. Storage is allocated lazily and released when the thread has no active requests.
pub const RESPONSE_STASH_CAPACITY: usize = 32 * 1024;

//==================================================================================================
// Request Identifier
//==================================================================================================

/// Identifies one request within a thread's in-flight request window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RequestIdentifier(u32);

impl RequestIdentifier {
    /// Identifier used by messages that do not expect a response.
    pub const NONE: Self = Self(0);

    /// Byte offset of the identifier in a request/response message payload.
    pub const OFFSET: usize = 2;

    /// Size of a request identifier in bytes.
    pub const SIZE: usize = core::mem::size_of::<u32>();

    /// Creates an identifier from its wire representation.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the wire representation of this identifier.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Reads an identifier from a message payload.
    pub fn read_from(message: &Message) -> Self {
        let bytes: [u8; Self::SIZE] = message.payload[Self::OFFSET..Self::OFFSET + Self::SIZE]
            .try_into()
            .expect("request identifier slice has a fixed size");
        Self(u32::from_ne_bytes(bytes))
    }

    /// Writes this identifier to a message payload.
    pub fn write_to(self, message: &mut Message) {
        message.payload[Self::OFFSET..Self::OFFSET + Self::SIZE]
            .copy_from_slice(&self.0.to_ne_bytes());
    }
}

/// Classification of one response received while waiting for a request token.
pub enum ResponseDisposition {
    /// The response matches the request currently waiting for it.
    Matched(Message),
    /// The response belongs to another active request and was stashed.
    Stashed,
    /// The response does not belong to any active request and must be logged and dropped.
    Stale(RequestIdentifier),
    /// The response carries an active identifier, but came from the wrong process.
    UnexpectedSource {
        /// Identifier carried by the response.
        identifier: RequestIdentifier,
        /// Process expected to answer the request.
        expected: ProcessIdentifier,
        /// Process that sent the response.
        actual: ProcessIdentifier,
    },
}

//==================================================================================================
// Request Identifier Allocator
//==================================================================================================

struct RequestIdentifierAllocator {
    next: u32,
}

impl RequestIdentifierAllocator {
    const fn new() -> Self {
        Self { next: 1 }
    }

    fn allocate(&mut self) -> RequestIdentifier {
        let identifier: RequestIdentifier = RequestIdentifier(self.next);
        self.next = self.next.wrapping_add(1);
        if self.next == RequestIdentifier::NONE.raw() {
            self.next = 1;
        }
        identifier
    }
}

//==================================================================================================
// Request State
//==================================================================================================

struct RequestState {
    owner: ThreadIdentifier,
    allocator: RequestIdentifierAllocator,
    active: [RequestIdentifier; MAX_ACTIVE_REQUESTS],
    responders: [ProcessIdentifier; MAX_ACTIVE_REQUESTS],
    stash: VecDeque<Message>,
    stash_capacity: usize,
    overflowed: bool,
}

impl RequestState {
    const fn new() -> Self {
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

    #[cfg(test)]
    fn with_stash_capacity(stash_capacity: usize) -> Self {
        let mut state: Self = Self::new();
        state.stash = VecDeque::with_capacity(stash_capacity);
        state.stash_capacity = stash_capacity;
        state
    }

    fn claim(&mut self, owner: ThreadIdentifier) {
        *self = Self::new();
        self.owner = owner;
    }

    fn is_idle(&self) -> bool {
        self.active
            .iter()
            .all(|identifier| *identifier == RequestIdentifier::NONE)
    }

    fn allocate(&mut self, responder: ProcessIdentifier) -> Result<RequestIdentifier, Error> {
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

    fn activate(
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

    fn release(&mut self, identifier: RequestIdentifier) -> Option<VecDeque<Message>> {
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

    fn take(&mut self, identifier: RequestIdentifier) -> Result<Option<Message>, Error> {
        if self.overflowed {
            return Err(Error::new(ErrorCode::NoBufferSpace, "IPC response stash overflowed"));
        }
        let response: Option<usize> = self
            .stash
            .iter()
            .position(|message| RequestIdentifier::read_from(message) == identifier);
        Ok(response.and_then(|index| self.stash.remove(index)))
    }

    fn stash(&mut self, message: Message) -> Result<bool, Error> {
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

    fn classify(
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

static REQUEST_STATES: Mutex<[RequestState; ::config::kernel::MAX_THREADS]> =
    Mutex::new([const { RequestState::new() }; ::config::kernel::MAX_THREADS]);

/// Holds the request-state mutex across a process duplication.
pub struct RequestStateForkGuard {
    _private: (),
}

impl RequestStateForkGuard {
    /// Locks request state before duplicating the calling process.
    ///
    /// # Safety
    ///
    /// Signal delivery must be blocked, and the caller must not access request state until this
    /// guard is dropped in both the parent and child.
    pub unsafe fn acquire() -> Self {
        let guard = REQUEST_STATES.lock();
        core::mem::forget(guard);
        Self { _private: () }
    }
}

impl Drop for RequestStateForkGuard {
    fn drop(&mut self) {
        // SAFETY: `acquire()` forgot the unique guard that locked this process's copy. After fork,
        // the parent and child each unlock their own private copy exactly once.
        unsafe {
            REQUEST_STATES.force_unlock();
        }
    }
}

fn with_request_state<T>(
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

fn with_active_request_state<T>(
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
fn prepare_nested_request(owner: ThreadIdentifier) -> Result<(), Error> {
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

//==================================================================================================
// Request Token
//==================================================================================================

/// Keeps a request identifier active until its response window closes.
pub struct RequestToken {
    owner: ThreadIdentifier,
    identifier: RequestIdentifier,
    responder: ProcessIdentifier,
}

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
                .find(|state| state.owner == self.owner)
                .and_then(|state| state.release(self.identifier))
        };
        drop(stash);
    }
}

/// Clears all request state owned by one thread.
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

/// Returns whether `owner` has any request awaiting a response.
pub fn has_active_requests(owner: ThreadIdentifier) -> bool {
    let states = REQUEST_STATES.lock();
    states
        .iter()
        .find(|state| state.owner == owner)
        .is_some_and(|state| !state.is_idle())
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::{
        MessageReceiver,
        MessageSender,
        MessageType,
    };
    use ::alloc::collections::VecDeque;

    fn message(identifier: RequestIdentifier) -> Message {
        let mut message: Message = Message::new(
            MessageSender::KERNEL,
            MessageReceiver::KERNEL,
            MessageType::Ipc,
            None,
            [0; Message::PAYLOAD_SIZE],
        );
        identifier.write_to(&mut message);
        message
    }

    #[test]
    fn request_identifier_generation_wraps_and_skips_zero() {
        let mut allocator: RequestIdentifierAllocator =
            RequestIdentifierAllocator { next: u32::MAX };
        assert_eq!(allocator.allocate(), RequestIdentifier::from_raw(u32::MAX));
        assert_eq!(allocator.allocate(), RequestIdentifier::from_raw(1));
    }

    #[test]
    fn request_identifier_generation_skips_active_id_after_wraparound() {
        let mut state: RequestState = RequestState::new();
        let active: RequestIdentifier = RequestIdentifier::from_raw(u32::MAX);
        state
            .activate(active, ProcessIdentifier::KERNEL)
            .expect("identifier should become active");
        state.allocator.next = u32::MAX;

        assert_eq!(
            state
                .allocate(ProcessIdentifier::KERNEL)
                .expect("another identifier should be available"),
            RequestIdentifier::from_raw(1)
        );
    }

    #[test]
    fn request_state_stashes_active_responses_and_rejects_stale_responses() {
        let mut state: RequestState = RequestState::with_stash_capacity(1);
        let active: RequestIdentifier = RequestIdentifier::from_raw(7);
        state
            .activate(active, ProcessIdentifier::KERNEL)
            .expect("identifier should become active");
        assert!(state
            .stash(message(active))
            .expect("active response should be stashed"));
        assert!(!state
            .stash(message(RequestIdentifier::from_raw(8)))
            .expect("stale response should be rejected"));
        assert!(state
            .take(active)
            .expect("stash lookup should succeed")
            .is_some());
    }

    #[test]
    fn request_state_stashes_multipart_response_beyond_active_request_count() {
        let mut state: RequestState = RequestState::with_stash_capacity(MAX_ACTIVE_REQUESTS + 1);
        let active: RequestIdentifier = RequestIdentifier::from_raw(7);
        state
            .activate(active, ProcessIdentifier::KERNEL)
            .expect("identifier should become active");

        for part_number in 0..=MAX_ACTIVE_REQUESTS {
            let mut response: Message = message(active);
            response.status = part_number as i32;
            state
                .stash(response)
                .expect("multipart response should fit in the stash");
        }

        for part_number in 0..=MAX_ACTIVE_REQUESTS {
            let response: Message = state
                .take(active)
                .expect("stash lookup should succeed")
                .expect("multipart response part should be retained");
            let status: i32 = response.status;
            assert_eq!(status, part_number as i32);
        }
    }

    #[test]
    fn nested_request_token_stashes_multipart_response_beyond_active_request_count() {
        let owner: ThreadIdentifier = ThreadIdentifier::from(0x4000_0000);
        let outer: RequestToken = RequestToken::allocate(owner, ProcessIdentifier::KERNEL)
            .expect("outer request token should be allocated");
        let nested: RequestToken = RequestToken::allocate(owner, ProcessIdentifier::KERNEL)
            .expect("nested request token should reserve the response stash");

        for part_number in 0..=MAX_ACTIVE_REQUESTS {
            let mut response: Message = message(outer.identifier());
            response.status = part_number as i32;
            assert!(matches!(
                nested
                    .classify_response(response)
                    .expect("multipart response should fit in the stash"),
                ResponseDisposition::Stashed
            ));
        }

        for part_number in 0..=MAX_ACTIVE_REQUESTS {
            let response: Message = outer
                .take_stashed()
                .expect("stash lookup should succeed")
                .expect("multipart response part should be retained");
            let status: i32 = response.status;
            assert_eq!(status, part_number as i32);
        }
    }

    #[test]
    fn stale_response_is_classified_for_logging_and_drop() {
        let mut state: RequestState = RequestState::new();
        let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
        let stale: RequestIdentifier = RequestIdentifier::from_raw(8);
        state
            .activate(expected, ProcessIdentifier::KERNEL)
            .expect("identifier should become active");

        match state
            .classify(expected, message(stale))
            .expect("response classification should succeed")
        {
            ResponseDisposition::Stale(identifier) => assert_eq!(identifier, stale),
            _ => panic!("stale response should be classified for logging and drop"),
        }
    }

    #[test]
    fn stale_response_is_logged_and_skipped() {
        let owner: ThreadIdentifier = ThreadIdentifier::from(0x4000_1000);
        let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
        let stale: RequestIdentifier = RequestIdentifier::from_raw(8);
        let token: RequestToken =
            RequestToken::activate(owner, expected, ProcessIdentifier::KERNEL)
                .expect("request token should become active");
        let mut responses: VecDeque<Message> = VecDeque::from([message(stale), message(expected)]);
        let mut stale_logs: Vec<RequestIdentifier> = Vec::new();

        let matched: Message = token
            .receive_response_with(
                || {
                    responses.pop_front().ok_or_else(|| {
                        Error::new(ErrorCode::InvalidArgument, "response queue is empty")
                    })
                },
                |identifier| stale_logs.push(identifier),
                |_, _, _| panic!("response source should match"),
            )
            .expect("matching response should be returned");

        assert_eq!(RequestIdentifier::read_from(&matched), expected);
        assert_eq!(stale_logs, Vec::from([stale]));
        assert!(responses.is_empty());
    }

    #[test]
    fn uncorrelated_response_is_never_stashed() {
        let mut state: RequestState = RequestState::new();
        let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
        state
            .activate(expected, ProcessIdentifier::KERNEL)
            .expect("identifier should become active");

        assert!(!state
            .stash(message(RequestIdentifier::NONE))
            .expect("uncorrelated response should be rejected"));
        match state
            .classify(expected, message(RequestIdentifier::NONE))
            .expect("response classification should succeed")
        {
            ResponseDisposition::Stale(identifier) => {
                assert_eq!(identifier, RequestIdentifier::NONE)
            },
            _ => panic!("uncorrelated response should be classified for logging and drop"),
        }
    }

    #[test]
    fn request_state_reports_stash_overflow() {
        const TEST_STASH_CAPACITY: usize = 4;

        let mut state: RequestState = RequestState::with_stash_capacity(TEST_STASH_CAPACITY);
        let identifier: RequestIdentifier = RequestIdentifier::from_raw(7);
        state
            .activate(identifier, ProcessIdentifier::KERNEL)
            .expect("identifier should become active");
        for _ in 0..TEST_STASH_CAPACITY {
            state
                .stash(message(identifier))
                .expect("response should fit in stash");
        }
        let error: Error = state
            .stash(message(identifier))
            .expect_err("one response beyond capacity should overflow");
        assert_eq!(error.code, ErrorCode::NoBufferSpace);
        let error: Error = state
            .take(identifier)
            .expect_err("overflow should poison active requests");
        assert_eq!(error.code, ErrorCode::NoBufferSpace);
    }

    #[test]
    fn response_from_unexpected_source_is_rejected() {
        let mut state: RequestState = RequestState::new();
        let expected: RequestIdentifier = RequestIdentifier::from_raw(7);
        state
            .activate(expected, ProcessIdentifier::VFSD)
            .expect("identifier should become active");

        match state
            .classify(expected, message(expected))
            .expect("response classification should succeed")
        {
            ResponseDisposition::UnexpectedSource {
                identifier,
                expected: expected_source,
                actual,
            } => {
                assert_eq!(identifier, expected);
                assert_eq!(expected_source, ProcessIdentifier::VFSD);
                assert_eq!(actual, ProcessIdentifier::KERNEL);
            },
            _ => panic!("response from an unexpected source should be rejected"),
        }
    }

    #[test]
    fn request_states_generate_identifiers_independently() {
        let mut first: RequestState = RequestState::new();
        let mut second: RequestState = RequestState::new();

        assert_eq!(
            first
                .allocate(ProcessIdentifier::KERNEL)
                .expect("first state should allocate an identifier"),
            RequestIdentifier::from_raw(1)
        );
        assert_eq!(
            second
                .allocate(ProcessIdentifier::KERNEL)
                .expect("second state should allocate an identifier"),
            RequestIdentifier::from_raw(1)
        );
        assert_eq!(
            first
                .allocate(ProcessIdentifier::KERNEL)
                .expect("first state should advance independently"),
            RequestIdentifier::from_raw(2)
        );
    }

    #[test]
    fn active_request_query_tracks_token_lifetime() {
        let owner: ThreadIdentifier = ThreadIdentifier::from(0x4000_2000);
        assert!(!has_active_requests(owner));

        let token: RequestToken = RequestToken::allocate(owner, ProcessIdentifier::KERNEL)
            .expect("request token should be allocated");
        assert!(has_active_requests(owner));

        drop(token);
        assert!(!has_active_requests(owner));
    }
}
