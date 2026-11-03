// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Drains all available bytes from a read end into a freshly allocated vector.
fn read_all(end: &PipeEnd, max: usize) -> alloc::vec::Vec<u8> {
    let mut buf: alloc::vec::Vec<u8> = alloc::vec![0u8; max];
    match end.read(&mut buf).expect("read end") {
        PipeReadOutcome::Read(n) => {
            buf.truncate(n);
            buf
        },
        _ => alloc::vec::Vec::new(),
    }
}

/// Tests a basic write-then-read round-trip preserving byte order.
#[test]
fn round_trip_preserves_bytes() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    let data: [u8; 5] = [1, 2, 3, 4, 5];
    match w.write(&data).expect("write end") {
        PipeWriteOutcome::Wrote(n) => assert_eq!(n, 5, "should write all 5 bytes"),
        _ => panic!("write should succeed"),
    }
    assert_eq!(read_all(&r, 16), data, "read bytes should match written bytes");
}

/// Tests that peeking preserves bytes until the caller explicitly consumes them.
#[test]
fn peek_then_consume_commits_prefix() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    w.write(&[1, 2, 3, 4]).expect("write end");
    let mut buf: [u8; 3] = [0; 3];

    assert!(
        matches!(r.peek(&mut buf).expect("read end"), PipeReadOutcome::Read(3)),
        "peek should copy the requested prefix"
    );
    assert_eq!(buf, [1, 2, 3], "peeked bytes should preserve order");
    assert_eq!(r.consume(2).expect("read end"), 2, "consume should remove two bytes");
    assert_eq!(read_all(&r, 4), [3, 4], "only the committed prefix should be removed");
}

/// Tests that the two ends report the same, unique pipe identity.
#[test]
fn ends_share_identity_unique_across_pipes() {
    let (r1, w1): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    let (r2, _w2): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    assert_eq!(r1.pipe_id(), w1.pipe_id(), "both ends share one identity");
    assert_ne!(r1.pipe_id(), r2.pipe_id(), "distinct pipes have distinct identities");
    assert!(!r1.is_write(), "read end must not be the write end");
    assert!(w1.is_write(), "write end must be the write end");
}

/// Tests that a read on an empty pipe with an open writer would block.
#[test]
fn empty_with_writer_would_block() {
    let (r, _w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    let mut buf: [u8; 4] = [0; 4];
    assert!(
        matches!(r.read(&mut buf).expect("read end"), PipeReadOutcome::WouldBlock),
        "empty pipe with a writer should block"
    );
}

/// Tests that a read on an empty pipe with no writers returns EOF.
#[test]
fn empty_without_writer_is_eof() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    drop(w);
    let mut buf: [u8; 4] = [0; 4];
    assert!(
        matches!(r.read(&mut buf).expect("read end"), PipeReadOutcome::Eof),
        "empty pipe with no writers should report EOF"
    );
}

/// Tests that buffered data is still readable after all writers close (drain-then-EOF).
#[test]
fn buffered_data_readable_after_writer_close() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    w.write(&[9, 8, 7]).expect("write end");
    drop(w);
    assert_eq!(read_all(&r, 8), alloc::vec![9, 8, 7], "buffered bytes must survive writer close");
    let mut buf: [u8; 4] = [0; 4];
    assert!(
        matches!(r.read(&mut buf).expect("read end"), PipeReadOutcome::Eof),
        "after draining, an empty pipe with no writers reports EOF"
    );
}

/// Tests that a write to a pipe whose readers have all closed is a broken pipe.
#[test]
fn write_without_reader_is_broken_pipe() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    drop(r);
    assert!(
        matches!(w.write(&[1, 2, 3]).expect("write end"), PipeWriteOutcome::BrokenPipe),
        "writing with no readers should report a broken pipe"
    );
}

/// Tests `PIPE_BUF`-sized atomic writes: a write that does not fit entirely appends nothing.
#[test]
fn atomic_write_is_all_or_nothing() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    // Fill the buffer to within less than PIPE_BUF of capacity.
    let bulk: alloc::vec::Vec<u8> = alloc::vec![0u8; PIPE_CAPACITY - 16];
    match w.write(&bulk).expect("write end") {
        PipeWriteOutcome::Wrote(n) => assert_eq!(n, PIPE_CAPACITY - 16, "bulk fill should fit"),
        _ => panic!("bulk fill should succeed"),
    }
    // A PIPE_BUF-sized write cannot fit in the remaining 16 bytes, so nothing is written.
    let atomic: [u8; PIPE_BUF] = [7u8; PIPE_BUF];
    assert!(
        matches!(w.write(&atomic).expect("write end"), PipeWriteOutcome::WouldBlock),
        "an atomic write that does not fit must append nothing"
    );
    // Free space, then the same write succeeds atomically.
    let _ = r.read(&mut [0u8; 4096]).expect("read end");
    assert!(
        matches!(w.write(&atomic).expect("write end"), PipeWriteOutcome::Wrote(PIPE_BUF)),
        "the atomic write should succeed once space is available"
    );
}

/// Tests that a write larger than `PIPE_BUF` may be partially accepted up to capacity.
#[test]
fn large_write_is_partial() {
    let (_r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    let big: alloc::vec::Vec<u8> = alloc::vec![5u8; PIPE_CAPACITY + 4096];
    match w.write(&big).expect("write end") {
        PipeWriteOutcome::Wrote(n) => {
            assert_eq!(n, PIPE_CAPACITY, "a large write fills exactly to capacity")
        },
        _ => panic!("a large write into an empty pipe should make progress"),
    }
    // The pipe is now full: a further large write would block.
    assert!(
        matches!(w.write(&big).expect("write end"), PipeWriteOutcome::WouldBlock),
        "a full pipe should block further writes"
    );
}

/// Tests that using an end in the wrong direction is rejected.
#[test]
fn wrong_direction_is_rejected() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    assert_eq!(
        w.read(&mut [0u8; 4]).expect_err("read on write end"),
        PipeEndError::WrongDirection,
        "reading the write end must be rejected"
    );
    assert_eq!(
        r.write(&[0u8; 4]).expect_err("write on read end"),
        PipeEndError::WrongDirection,
        "writing the read end must be rejected"
    );
}

/// Tests ring-buffer wrap-around across many interleaved write/read cycles.
#[test]
fn ring_buffer_wraps_around() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    let chunk: [u8; 1000] = [3u8; 1000];
    // Many cycles push total bytes well past PIPE_CAPACITY, exercising wrap-around.
    for _ in 0..200 {
        assert!(
            matches!(w.write(&chunk).expect("write end"), PipeWriteOutcome::Wrote(1000)),
            "each 1000-byte chunk should fit after draining"
        );
        assert_eq!(read_all(&r, 1000).len(), 1000, "each cycle should read back 1000 bytes");
    }
}

/// Tests that a zero-length read returns `Read(0)` immediately and never blocks.
#[test]
fn zero_length_read_does_not_block() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    // Empty pipe with an open writer would normally block, but a zero-length read must not.
    assert!(
        matches!(r.read(&mut []).expect("read end"), PipeReadOutcome::Read(0)),
        "a zero-length read on an empty pipe with a writer must return Read(0)"
    );
    // The same holds once all writers have closed: still `Read(0)`, never `Eof`.
    drop(w);
    assert!(
        matches!(r.read(&mut []).expect("read end"), PipeReadOutcome::Read(0)),
        "a zero-length read on an empty pipe with no writers must return Read(0)"
    );
}

/// Tests read-end readiness for empty, populated, and hung-up pipes.
#[test]
fn poll_read_end_tracks_data_and_writer_close() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    assert_eq!(r.poll(POLLIN), 0, "an empty pipe with a writer is not readable");

    w.write(&[1]).expect("write end");
    assert_eq!(r.poll(POLLIN), POLLIN, "buffered data makes the read end readable");

    drop(w);
    assert_eq!(
        r.poll(POLLIN),
        POLLIN | POLLHUP,
        "buffered data remains readable and writer closure reports hangup"
    );
    assert_eq!(read_all(&r, 1), alloc::vec![1]);
    assert_eq!(r.poll(POLLIN), POLLIN | POLLHUP, "EOF is readable and the hangup state persists");
}

/// Tests write-end readiness for available capacity, a full buffer, and reader closure.
#[test]
fn poll_write_end_tracks_capacity_and_reader_close() {
    let (r, w): (PipeEnd, PipeEnd) = PipeEnd::new_pair();
    assert_eq!(w.poll(POLLOUT), POLLOUT, "an empty pipe is writable");

    let data: alloc::vec::Vec<u8> = alloc::vec![0; PIPE_CAPACITY];
    assert!(
        matches!(w.write(&data).expect("write end"), PipeWriteOutcome::Wrote(PIPE_CAPACITY)),
        "the write should fill the pipe"
    );
    assert_eq!(w.poll(POLLOUT), 0, "a full pipe is not writable");

    drop(r);
    assert_eq!(
        w.poll(POLLOUT),
        POLLOUT | POLLERR,
        "a closed read end reports the write error without suppressing writability"
    );
    assert_eq!(w.poll(0), POLLERR, "pipe errors are reported without being requested");
}
