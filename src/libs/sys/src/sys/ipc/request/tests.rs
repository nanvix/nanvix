// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    identifier_allocator::RequestIdentifierAllocator,
    state::RequestState,
    *,
};
use crate::{
    error::{
        Error,
        ErrorCode,
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
use ::alloc::{
    collections::VecDeque,
    vec::Vec,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

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

//==================================================================================================
// Tests
//==================================================================================================

#[test]
fn request_identifier_generation_wraps_and_skips_zero() {
    let mut allocator: RequestIdentifierAllocator = RequestIdentifierAllocator::with_next(u32::MAX);
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
    state.set_next_identifier(u32::MAX);

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
    let token: RequestToken = RequestToken::activate(owner, expected, ProcessIdentifier::KERNEL)
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
