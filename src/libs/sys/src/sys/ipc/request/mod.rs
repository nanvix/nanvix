// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Correlates IPC responses with requests issued by a thread.
//!
//! [`RequestToken`] is the public lifecycle object. Allocating or activating a token registers a
//! [`RequestIdentifier`] and its expected responder in the thread's private `RequestState`.
//! Incoming responses are matched against that state and described by [`ResponseDisposition`]; a
//! response for another active token is stashed until that token receives it. Dropping a token
//! releases its identifier and any responses still stashed for it.
//!
//! # Request Flow
//!
//! ```text
//! +-----------------------------+
//! | RequestToken                |
//! | public RAII lifecycle       |
//! +--------------+--------------+
//!                | allocate / activate / take / classify / drop
//!                v
//! +--------------+-----------------------+   owns   +----------------------------+
//! | RequestState (one per active thread) |--------->| RequestIdentifierAllocator |
//! | active IDs, responders, reply stash  |<---------| nonzero ID candidates      |
//! +---------+--------------------+-------+ candidate +----------------------------+
//!           | tracks             | classify(Message)
//!           v                    v
//! +-------------------+   +------------------------------+
//! | RequestIdentifier |   | ResponseDisposition          |
//! | wire correlation  |   | match / stash / reject       |
//! +-------------------+   +------------------------------+
//!           ^
//!           | encoded in the shared IPC message prefix
//!
//! RequestStateForkGuard -- locks RequestState[] across fork()
//! child RequestToken ----- resets the table and reactivates its surviving identifier
//! ```
//!
//! # Submodule Responsibilities
//!
//! - `identifier` defines the identifier's wire encoding and reserved uncorrelated value using the
//!   shared layout constants owned by the IPC message module.
//! - `identifier_allocator` generates nonzero identifier candidates independently for each
//!   `RequestState`; the state rejects candidates that are still active after wraparound.
//! - `response_disposition` describes whether a response matched, was stashed, is stale, or came
//!   from an unexpected process.
//! - `state` owns synchronized per-thread request state, including active identifiers, expected
//!   responders, and out-of-order response storage.
//! - `state_fork_guard` provides [`RequestStateForkGuard`], which holds request-state synchronization
//!   across `fork()` so the child can safely discard inherited state.
//! - `token` provides [`RequestToken`], coordinates state transitions, and rebinds the surviving
//!   request in a freshly forked child.

//==================================================================================================
// Modules
//==================================================================================================

mod identifier;
mod identifier_allocator;
mod response_disposition;
mod state;
mod state_fork_guard;
mod token;

#[cfg(test)]
mod tests;

//==================================================================================================
// Exports
//==================================================================================================

pub use identifier::RequestIdentifier;
pub use response_disposition::ResponseDisposition;
pub use state::{
    clear_request_state,
    has_active_requests,
};
pub use state_fork_guard::RequestStateForkGuard;
pub use token::RequestToken;

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
