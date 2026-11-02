// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

fn test_pid() -> ProcessIdentifier {
    ProcessIdentifier::from(10)
}

fn pending_request(tid: ThreadIdentifier, request_id: u32) -> PendingBulkRequest {
    pending_request_with_header(tid, request_id, SystemCallMessageKind::WriteRequest)
}

fn pending_request_with_header(
    tid: ThreadIdentifier,
    request_id: u32,
    header: SystemCallMessageKind,
) -> PendingBulkRequest {
    let message: SystemCallMessage =
        SystemCallMessage::new(header, [0u8; SystemCallMessage::PAYLOAD_SIZE]);
    let mut outer: Message = Message::new(
        MessageSender::KERNEL,
        MessageReceiver::KERNEL,
        MessageType::Ikc,
        None,
        message.into_bytes(),
    );
    RequestIdentifier::from_raw(request_id).write_to(&mut outer);
    let message: SystemCallMessage =
        SystemCallMessage::try_from_bytes(outer.payload).expect("message should decode");
    PendingBulkRequest {
        source: MessageSender::new(test_pid(), tid),
        message,
        queued_at: Instant::now(),
    }
}

fn bulk(tid: ThreadIdentifier, request_id: u32) -> DataChunk {
    let header: DataChunkHeader = DataChunkHeader::new_tagged(
        test_pid(),
        tid,
        ProcessIdentifier::KERNEL,
        ThreadIdentifier::KERNEL,
        0,
        0,
        request_id,
    );
    DataChunk::new(header, Vec::new())
}

#[test]
fn bulk_request_matching_handles_cross_thread_interleaving() {
    let first_tid: ThreadIdentifier = ThreadIdentifier::from(20);
    let second_tid: ThreadIdentifier = ThreadIdentifier::from(21);
    let mut pending: Vec<PendingBulkRequest> = vec![
        pending_request(first_tid, 1),
        pending_request(second_tid, 2),
    ];

    let selected: PendingBulkRequest = take_pending_bulk_request(&mut pending, &bulk(first_tid, 1))
        .expect("first thread should have a pending bulk request");
    let selected_id: u32 = selected.message.request_id().raw();
    assert_eq!(selected_id, 1);
    assert_eq!(pending.len(), 1);
    let remaining_id: u32 = pending[0].message.request_id().raw();
    assert_eq!(remaining_id, 2);
}

#[test]
fn bulk_request_matching_uses_lifo_order_for_nested_thread_requests() {
    let tid: ThreadIdentifier = ThreadIdentifier::from(20);
    let mut pending: Vec<PendingBulkRequest> =
        vec![pending_request(tid, 1), pending_request(tid, 2)];

    let selected: PendingBulkRequest = take_pending_bulk_request(&mut pending, &bulk(tid, 2))
        .expect("nested request should have a pending bulk frame");
    let selected_id: u32 = selected.message.request_id().raw();
    assert_eq!(selected_id, 2);
    assert_eq!(pending.len(), 1);
    let remaining_id: u32 = pending[0].message.request_id().raw();
    assert_eq!(remaining_id, 1);
}

#[test]
fn pending_bulk_request_queue_is_bounded() {
    let tid: ThreadIdentifier = ThreadIdentifier::from(20);
    let mut pending: Vec<PendingBulkRequest> = Vec::new();
    for request_id in 0..=MAX_ORDINARY_PENDING_BULK_REQUESTS {
        let request_id: u32 =
            u32::try_from(request_id).expect("pending request bound should fit u32");
        let result: Result<(), PendingBulkRequest> =
            queue_pending_bulk_request(&mut pending, pending_request(tid, request_id));
        if request_id
            < u32::try_from(MAX_ORDINARY_PENDING_BULK_REQUESTS)
                .expect("pending request bound should fit u32")
        {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }

    assert_eq!(pending.len(), MAX_ORDINARY_PENDING_BULK_REQUESTS);
    let first_id: u32 = pending[0].message.request_id().raw();
    assert_eq!(first_id, 0);
    let last_id: u32 = pending[MAX_ORDINARY_PENDING_BULK_REQUESTS - 1]
        .message
        .request_id()
        .raw();
    let expected_last: u32 = u32::try_from(MAX_ORDINARY_PENDING_BULK_REQUESTS - 1)
        .expect("pending request bound should fit u32");
    assert_eq!(last_id, expected_last);

    for index in 0..::sys::ipc::MAX_ACTIVE_REQUESTS {
        let request_id: u32 = u32::try_from(index).expect("active request bound should fit u32");
        let poll: PendingBulkRequest =
            pending_request_with_header(tid, request_id, SystemCallMessageKind::PollInputRequest);
        assert!(queue_pending_bulk_request(&mut pending, poll).is_ok());
    }
    assert_eq!(pending.len(), MAX_PENDING_BULK_REQUESTS);

    let excess_poll: PendingBulkRequest =
        pending_request_with_header(tid, u32::MAX, SystemCallMessageKind::PollInputRequest);
    assert!(queue_pending_bulk_request(&mut pending, excess_poll).is_err());
}

#[test]
fn pending_bulk_request_expiration_removes_only_expired_requests() {
    let tid: ThreadIdentifier = ThreadIdentifier::from(20);
    let now: Instant = Instant::now();
    let mut expired_request: PendingBulkRequest = pending_request(tid, 1);
    expired_request.queued_at = now - PENDING_BULK_REQUEST_TIMEOUT;
    let mut fresh_request: PendingBulkRequest = pending_request(tid, 2);
    fresh_request.queued_at = now;
    let mut poll_request: PendingBulkRequest =
        pending_request_with_header(tid, 3, SystemCallMessageKind::PollInputRequest);
    poll_request.queued_at = now - PENDING_BULK_REQUEST_TIMEOUT;
    let mut pending: Vec<PendingBulkRequest> = vec![expired_request, fresh_request, poll_request];

    let expired: Vec<PendingBulkRequest> = expire_pending_bulk_requests(&mut pending);

    assert_eq!(expired.len(), 1);
    let expired_id: u32 = expired[0].message.request_id().raw();
    assert_eq!(expired_id, 1);
    assert_eq!(pending.len(), 2);
    let pending_id: u32 = pending[0].message.request_id().raw();
    assert_eq!(pending_id, 2);
    let poll_id: u32 = pending[1].message.request_id().raw();
    assert_eq!(poll_id, 3);
}

/// Maximum time a test waits for the I/O handler to reply to a guest frame.
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// Number of bytes requested by test pulls.
const PULL_LEN: u32 = 16;

/// Guest-side channels of an I/O handler running without networking or HostFS.
struct IoHandlerHarness {
    guest_tx: mpsc::Sender<IkcFrame>,
    guest_rx: mpsc::Receiver<IkcFrame>,
    _output_rx: mpsc::Receiver<Vec<u8>>,
    _input_tx: mpsc::Sender<Vec<u8>>,
    handler: JoinHandle<()>,
}

impl IoHandlerHarness {
    fn spawn() -> Self {
        let (guest_tx, vm_stdout_rx): (mpsc::Sender<IkcFrame>, mpsc::Receiver<IkcFrame>) =
            mpsc::channel(CHANNEL_CAPACITY);
        let (vm_stdin_tx, guest_rx): (mpsc::Sender<IkcFrame>, mpsc::Receiver<IkcFrame>) =
            mpsc::channel(CHANNEL_CAPACITY);
        let (output_tx, output_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel(CHANNEL_CAPACITY);
        let (input_tx, input_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel(CHANNEL_CAPACITY);
        let handler: JoinHandle<()> = tokio::spawn(standalone_io_handler(
            vm_stdout_rx,
            vm_stdin_tx,
            output_tx,
            input_rx,
            MessageCounters::new(),
            NetworkingMode::Disabled,
            HostFilter::default(),
            None,
        ));
        Self {
            guest_tx,
            guest_rx,
            _output_rx: output_rx,
            _input_tx: input_tx,
            handler,
        }
    }

    async fn send(&self, frame: IkcFrame) {
        self.guest_tx
            .send(frame)
            .await
            .expect("I/O handler should accept guest frames");
    }

    async fn recv(&mut self) -> IkcFrame {
        tokio::time::timeout(REPLY_TIMEOUT, self.guest_rx.recv())
            .await
            .expect("I/O handler should reply before the timeout")
            .expect("I/O handler should keep its guest channel open")
    }

    /// Queues ordinary bulk metadata until the next ordinary request is rejected.
    async fn fill_pending_bulk_requests(&self) {
        let tid: ThreadIdentifier = ThreadIdentifier::from(20);
        for request_id in 0..MAX_ORDINARY_PENDING_BULK_REQUESTS {
            let request_id: u32 =
                u32::try_from(request_id).expect("pending request bound should fit u32");
            let write: Message = WriteRequest::build(
                tid,
                STDOUT_FILENO,
                1,
                [0u8; WriteRequest::BUFFER_SIZE],
                ProcessIdentifier::KERNEL,
                MessageType::Ikc,
            );
            self.send(guest_request(write, tid, request_id)).await;
        }
    }

    async fn shutdown(self) {
        drop(self.guest_tx);
        self.handler.await.expect("I/O handler should exit cleanly");
    }
}

/// Stamps a guest request with its kernel-attested sender and request identifier.
fn guest_request(mut message: Message, tid: ThreadIdentifier, request_id: u32) -> IkcFrame {
    message.source = MessageSender::new(test_pid(), tid);
    RequestIdentifier::from_raw(request_id).write_to(&mut message);
    IkcFrame::Message(message)
}

/// Builds the frame the VMM forwards when a guest thread pulls from the host.
fn pull_header(tid: ThreadIdentifier, request_id: u32, transfer_id: u32) -> IkcFrame {
    let header: DataChunkHeader = DataChunkHeader::new_tagged(
        test_pid(),
        tid,
        ProcessIdentifier::KERNEL,
        ThreadIdentifier::KERNEL,
        transfer_id,
        PULL_LEN,
        request_id,
    );
    IkcFrame::Bulk(DataChunk::new(header, Vec::new()))
}

/// Builds the frame the VMM forwards after copying a guest thread's host push.
fn push_data(tid: ThreadIdentifier, request_id: u32, data: &[u8]) -> IkcFrame {
    let data_len: u32 = u32::try_from(data.len()).expect("test push length should fit u32");
    let header: DataChunkHeader = DataChunkHeader::new_tagged(
        test_pid(),
        tid,
        ProcessIdentifier::KERNEL,
        ThreadIdentifier::KERNEL,
        0,
        data_len,
        request_id,
    );
    IkcFrame::Bulk(DataChunk::new(header, data.to_vec()))
}

/// Asserts that `frame` completes the identified pull with an empty transfer.
fn assert_empty_pull_completion(
    frame: IkcFrame,
    tid: ThreadIdentifier,
    request_id: u32,
    transfer_id: u32,
) {
    let reply: DataChunk = match frame {
        IkcFrame::Bulk(reply) => Some(reply),
        IkcFrame::Message(_) => None,
    }
    .expect("expected an empty pull completion, received a message");
    assert!(reply.data().is_empty(), "the completion should carry no data");
    assert_eq!(reply.header().data_len(), 0, "the completion should report no data");
    assert!(reply.header().source_pid() == test_pid(), "the completion should target the puller");
    assert!(reply.header().source_tid() == tid, "the completion should target the pull thread");
    assert_eq!(reply.header().tag(), request_id, "the completion should keep the pull tag");
    assert_eq!(reply.header().data_addr(), transfer_id, "the completion should keep the pull");
}

/// Asserts that `frame` reports a bulk-request rejection to the identified caller.
fn assert_bulk_rejection(frame: IkcFrame, tid: ThreadIdentifier, request_id: u32) {
    let reply: Message = match frame {
        IkcFrame::Message(reply) => Some(reply),
        IkcFrame::Bulk(_) => None,
    }
    .expect("expected a rejection message, received a bulk frame");
    let status: i32 = reply.status;
    let destination: MessageReceiver = reply.destination;
    assert_eq!(status, ErrorCode::NoBufferSpace.get(), "the rejection should report ENOBUFS");
    assert!(destination.pid == test_pid(), "the rejection should target the caller");
    assert!(destination.tid == tid, "the rejection should target the calling thread");
    assert_eq!(
        RequestIdentifier::read_from(&reply).raw(),
        request_id,
        "the rejection should correlate with the request"
    );
}

/// A counted console poll from an untrusted sender is rejected, yet its pull still completes.
#[tokio::test]
async fn rejected_counted_poll_completes_pull_with_empty_transfer() {
    let tid: ThreadIdentifier = ThreadIdentifier::from(21);
    let mut harness: IoHandlerHarness = IoHandlerHarness::spawn();

    harness
        .send(guest_request(PollInputRequest::build(tid, PULL_LEN), tid, 1))
        .await;
    harness.send(pull_header(tid, 1, 0x100)).await;

    assert_empty_pull_completion(harness.recv().await, tid, 1, 0x100);
    harness.shutdown().await;
}

/// A read rejected at capacity receives its error and then has its pull completed.
#[tokio::test]
async fn rejected_read_reports_error_and_completes_pull() {
    let tid: ThreadIdentifier = ThreadIdentifier::from(21);
    let mut harness: IoHandlerHarness = IoHandlerHarness::spawn();
    harness.fill_pending_bulk_requests().await;

    let read: Message = ReadRequest::build(
        tid,
        STDIN_FILENO,
        PULL_LEN,
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    );
    harness.send(guest_request(read, tid, 1)).await;
    assert_bulk_rejection(harness.recv().await, tid, 1);

    harness.send(pull_header(tid, 1, 0x200)).await;
    assert_empty_pull_completion(harness.recv().await, tid, 1, 0x200);
    harness.shutdown().await;
}

/// A write rejected at capacity receives its error, and its late data frame draws no reply.
#[tokio::test]
async fn rejected_write_drops_late_data_without_reply() {
    let tid: ThreadIdentifier = ThreadIdentifier::from(21);
    let mut harness: IoHandlerHarness = IoHandlerHarness::spawn();
    harness.fill_pending_bulk_requests().await;

    let write: Message = WriteRequest::build(
        tid,
        STDOUT_FILENO,
        4,
        [0u8; WriteRequest::BUFFER_SIZE],
        ProcessIdentifier::KERNEL,
        MessageType::Ikc,
    );
    harness.send(guest_request(write, tid, 1)).await;
    assert_bulk_rejection(harness.recv().await, tid, 1);

    // The next reply must complete a later pull, so the dropped data frame drew no reply and
    // the handler kept serving the guest.
    harness.send(push_data(tid, 1, b"late")).await;
    harness.send(pull_header(tid, 2, 0x300)).await;
    assert_empty_pull_completion(harness.recv().await, tid, 2, 0x300);
    harness.shutdown().await;
}
