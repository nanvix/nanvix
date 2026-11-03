// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Thread identifier used as the message sender in the part-transport round trips.
const TEST_TID: i32 = 1;
/// Destination process identifier used in the part-transport round trips.
const TEST_DEST: i32 = 2;

/// Reassembles the `SystemCallMessagePart`s carried by the IPC messages produced by
/// `into_parts()`, asserting every part is tagged as a `SelectRequestPart`.
fn extract_parts(messages: Vec<Message>) -> Vec<SystemCallMessagePart> {
    messages
        .into_iter()
        .map(|msg| {
            let daemon_msg: SystemCallMessage =
                SystemCallMessage::try_from_bytes(msg.payload).expect("valid SystemCallMessage");
            // Copy the header out of the packed message before comparing it.
            let header: SystemCallMessageKind = daemon_msg.kind();
            assert_eq!(header, SystemCallMessageKind::SelectRequestPart, "unexpected part header");
            SystemCallMessagePart::from_bytes(daemon_msg.payload)
        })
        .collect()
}

/// Builds an `fd_set` with the given file descriptors set.
fn fd_set_with(bits: &[usize]) -> fd_set {
    let mut set: fd_set = fd_set::default();
    for &fd in bits {
        set.set_bit(fd).expect("fd within range");
    }
    set
}

/// Serializes `request`, splits it into parts, reassembles them, and asserts the decoded
/// request matches the original byte-for-byte.
fn assert_round_trip(request: SelectRequest) {
    // Snapshot the request fields before the value is consumed by `into_parts()`.
    let nfds: u8 = request.nfds;
    let readfds: Option<[u8; fd_set::WIRE_SIZE]> = request.readfds.map(|s| s.to_bytes());
    let writefds: Option<[u8; fd_set::WIRE_SIZE]> = request.writefds.map(|s| s.to_bytes());
    let errorfds: Option<[u8; fd_set::WIRE_SIZE]> = request.errorfds.map(|s| s.to_bytes());
    let timeout: Option<[u8; timeval::WIRE_SIZE]> = request.timeout;

    // The fixed-width wire layout always serializes to exactly `WIRE_SIZE` bytes.
    assert_eq!(request.to_bytes().len(), SelectRequest::WIRE_SIZE, "unexpected wire size");

    // Split into a `SelectRequestPart` stream and reassemble it.
    let messages: Vec<Message> = request
        .into_parts(
            ThreadIdentifier::from(TEST_TID),
            ProcessIdentifier::from(TEST_DEST),
            MessageType::Ikc,
        )
        .expect("partition should succeed");

    // A 45-byte request spans two parts on the current ABI; this exercises the part boundary.
    let expected_parts: usize =
        SelectRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
    assert_eq!(messages.len(), expected_parts, "unexpected number of parts");

    let parts: Vec<SystemCallMessagePart> = extract_parts(messages);
    let decoded: SelectRequest =
        SelectRequest::from_parts(&parts).expect("reassembly should succeed");

    assert_eq!(decoded.nfds, nfds, "nfds mismatch");
    assert_eq!(decoded.readfds.map(|s| s.to_bytes()), readfds, "readfds mismatch");
    assert_eq!(decoded.writefds.map(|s| s.to_bytes()), writefds, "writefds mismatch");
    assert_eq!(decoded.errorfds.map(|s| s.to_bytes()), errorfds, "errorfds mismatch");
    assert_eq!(decoded.timeout, timeout, "timeout mismatch");
}

/// Round-trips a request with every descriptor set and the timeout present.
#[test]
fn round_trip_all_sets_present() {
    let mut readfds: fd_set = fd_set_with(&[0, 3, 7]);
    let mut writefds: fd_set = fd_set_with(&[1, 2]);
    let mut errorfds: fd_set = fd_set_with(&[FD_SETSIZE - 1]);
    let timeout: timeval = timeval {
        tv_sec: 5,
        tv_usec: 123,
    };
    let request: SelectRequest = SelectRequest::new(
        8,
        &Some(&mut readfds),
        &Some(&mut writefds),
        &Some(&mut errorfds),
        &Some(timeout),
    )
    .expect("valid request");
    assert_round_trip(request);
}

/// Round-trips a request with every optional field absent.
#[test]
fn round_trip_all_sets_absent() {
    let request: SelectRequest =
        SelectRequest::new(0, &None, &None, &None, &None).expect("valid request");
    assert_round_trip(request);
}

/// `new()` rejects an `nfds` greater than `FD_SETSIZE`.
#[test]
fn new_rejects_nfds_above_setsize() {
    assert!(
        SelectRequest::new(FD_SETSIZE + 1, &None, &None, &None, &None).is_err(),
        "nfds above FD_SETSIZE must be rejected"
    );
}

/// `new()` accepts the maximum representable descriptor set size.
#[test]
fn new_accepts_nfds_at_setsize() {
    let request: SelectRequest =
        SelectRequest::new(FD_SETSIZE, &None, &None, &None, &None).expect("valid request");
    assert_eq!(request.nfds, FD_SETSIZE as u8, "nfds mismatch");
}

/// `try_from_bytes()` rejects a buffer shorter than the fixed wire layout.
#[test]
fn try_from_bytes_rejects_short_buffer() {
    let short: Vec<u8> = ::alloc::vec![0u8; SelectRequest::WIRE_SIZE - 1];
    assert!(SelectRequest::try_from_bytes(&short).is_err(), "short buffer must be rejected");
}

/// `try_from_bytes()` rejects unexpected trailing bytes.
#[test]
fn try_from_bytes_rejects_long_buffer() {
    let long: Vec<u8> = ::alloc::vec![0u8; SelectRequest::WIRE_SIZE + 1];
    assert!(SelectRequest::try_from_bytes(&long).is_err(), "long buffer must be rejected");
}
