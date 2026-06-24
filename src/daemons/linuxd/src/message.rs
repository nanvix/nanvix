// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::WorkerThreadError,
    syscalls::SyscallTable,
};
use ::alloc::collections::BTreeMap;
use ::sys::{
    error::Error,
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::syscall::message::{
    MessagePartitioner,
    SystemCallLongMessage,
    SystemCallMessagePart,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default)]
pub struct RequestAssembler {
    inflight: BTreeMap<ThreadIdentifier, RequestAssemblerType>,
}

impl RequestAssembler {
    /// Assembles a message part and, if all parts have arrived, extracts the
    /// deserialized request without executing it. This is intended to be called
    /// while the assembler lock is held so that the lock can be released before
    /// the potentially-blocking syscall runs.
    pub fn assemble_and_take<S, T: RequestAssemblerTrait<S>>(
        &mut self,
        source: ThreadIdentifier,
        part: SystemCallMessagePart,
    ) -> Result<Option<T>, WorkerThreadError> {
        match self.assemble_and_take_internal::<S, T>(source, part) {
            Ok(request) => Ok(request),
            Err(WorkerThreadError::Interrupted) => Err(WorkerThreadError::Interrupted),
            Err(WorkerThreadError::Error(e)) => {
                self.inflight.remove(&source);
                Err(WorkerThreadError::Error(e))
            },
        }
    }

    fn assemble_and_take_internal<S, T: RequestAssemblerTrait<S>>(
        &mut self,
        source: ThreadIdentifier,
        part: SystemCallMessagePart,
    ) -> Result<Option<T>, WorkerThreadError> {
        let message_complete: bool = {
            match self.assemble_parts::<S, T>(source, part) {
                Ok(message_complete) => message_complete,
                Err(e) => {
                    return Err(e);
                },
            }
        };

        if !message_complete {
            return Ok(None);
        }

        match self.take_request::<S, T>(source) {
            Ok(request) => Ok(Some(request)),
            Err(e) => Err(e),
        }
    }

    fn assemble_parts<S, T: RequestAssemblerTrait<S>>(
        &mut self,
        source: ThreadIdentifier,
        part: SystemCallMessagePart,
    ) -> Result<bool, WorkerThreadError> {
        let assembler: &mut RequestAssemblerType = self
            .inflight
            .entry(source)
            .or_insert_with(|| T::new_assembler());
        T::add_part(assembler, part)?;
        Ok(T::is_complete(assembler)?)
    }

    fn take_request<S, T: RequestAssemblerTrait<S>>(
        &mut self,
        source: ThreadIdentifier,
    ) -> Result<T, WorkerThreadError> {
        let assembler: RequestAssemblerType = self
            .inflight
            .remove(&source)
            .expect("inflight request does exist");

        let parts: Vec<SystemCallMessagePart> = T::take_parts(assembler);
        Ok(T::from_parts(&parts)?)
    }
}

#[allow(clippy::enum_variant_names)]
pub enum RequestAssemblerType {
    FileStatAtRequest(SystemCallLongMessage),
    SymbolicLinkAtRequest(SystemCallLongMessage),
    LinkAtRequest(SystemCallLongMessage),
    ReadLinkAtRequest(SystemCallLongMessage),
    MakeDirectoryAtRequest(SystemCallLongMessage),
    UpdateFileAccessTimeAtRequest(SystemCallLongMessage),
    FileChownAtRequest(SystemCallLongMessage),
    FileChmodAtRequest(SystemCallLongMessage),
    OpenAtRequest(SystemCallLongMessage),
    RenameAtRequest(SystemCallLongMessage),
    UnlinkAtRequest(SystemCallLongMessage),
    ChangeDirectoryRequest(SystemCallLongMessage),
    FileAccessAtRequest(SystemCallLongMessage),
    PollRequest(SystemCallLongMessage),
    SelectRequest(SystemCallLongMessage),
}

pub trait RequestAssemblerTrait<T>
where
    Self: Sized,
    Self: MessagePartitioner,
{
    fn new_assembler() -> RequestAssemblerType;

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError>;

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error>;

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart>;

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError>;
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::std::{
        sync::Arc,
        time::{
            Duration,
            Instant,
        },
    };
    use ::sys::ipc::MessageType;
    use ::syscall::{
        fcntl::message::OpenAtRequest,
        message::MessagePartitioner,
        SystemCallMessage,
    };
    use ::tokio::sync::Mutex;

    /// Extracts `SystemCallMessagePart` payloads from the IPC `Message` objects
    /// produced by `MessagePartitioner::into_parts()`.
    #[allow(clippy::expect_used)]
    fn extract_parts(messages: Vec<Message>) -> Vec<SystemCallMessagePart> {
        messages
            .into_iter()
            .map(|msg| {
                let daemon_msg: SystemCallMessage = SystemCallMessage::try_from_bytes(msg.payload)
                    .expect("valid SystemCallMessage");
                SystemCallMessagePart::from_bytes(daemon_msg.payload)
            })
            .collect()
    }

    /// Verifies that `assemble_and_take` returns the deserialized request
    /// without executing `process_request`. This is the core property
    /// introduced by the fix for issue #1493 — assembly is separated from
    /// execution so the caller can release the lock before running the
    /// potentially-blocking syscall.
    #[test]
    #[allow(clippy::expect_used)]
    fn assemble_and_take_returns_request_without_executing() {
        let source: ThreadIdentifier = ThreadIdentifier::from(42_i32);
        let original: OpenAtRequest =
            OpenAtRequest::new(-100, "/tmp/test.txt", 0x41, 0o644).expect("valid OpenAtRequest");

        let parts: Vec<SystemCallMessagePart> = extract_parts(
            original
                .into_parts(source, ::syscall::LINUXD, MessageType::Ikc)
                .expect("valid parts"),
        );

        let mut assembler: RequestAssembler = RequestAssembler::default();

        let mut result: Option<OpenAtRequest> = None;
        for part in parts {
            let r: Option<OpenAtRequest> = assembler
                .assemble_and_take::<(), OpenAtRequest>(source, part)
                .expect("assembly should succeed");
            if r.is_some() {
                result = r;
            }
        }

        let request: OpenAtRequest = result.expect("request should be fully assembled");
        assert_eq!(request.dirfd, -100, "dirfd mismatch");
        assert_eq!(request.flags, 0x41, "flags mismatch");
        assert_eq!(request.mode, 0o644, "mode mismatch");
        assert_eq!(request.pathname, "/tmp/test.txt", "pathname mismatch");
    }

    /// Regression test for issue #1493: deadlock when the assembler lock was
    /// held across blocking syscall execution.
    ///
    /// This test replicates the `handle_long_request` two-phase pattern
    /// introduced by the fix.  Thread A acquires the assembler lock, calls
    /// `assemble_and_take`, **drops the lock**, and then simulates a slow
    /// `process_request` (500 ms sleep).  Thread B attempts to acquire the
    /// same lock shortly after Thread A.
    ///
    /// With the fix (lock released before processing), Thread B acquires the
    /// lock almost immediately.  If someone were to revert to the old pattern
    /// (lock held during `process_request`), Thread B would be blocked for the
    /// full 500 ms, causing the assertion to fail.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[allow(clippy::expect_used)]
    async fn assembler_lock_released_before_process_request() {
        let assembler: Arc<Mutex<RequestAssembler>> =
            Arc::new(Mutex::new(RequestAssembler::default()));

        let source_a: ThreadIdentifier = ThreadIdentifier::from(1_i32);
        let request_a: OpenAtRequest =
            OpenAtRequest::new(-100, "/tmp/a.txt", 0x41, 0o644).expect("valid OpenAtRequest");
        let parts_a: Vec<SystemCallMessagePart> = extract_parts(
            request_a
                .into_parts(source_a, ::syscall::LINUXD, MessageType::Ikc)
                .expect("valid parts"),
        );

        let source_b: ThreadIdentifier = ThreadIdentifier::from(2_i32);
        let request_b: OpenAtRequest =
            OpenAtRequest::new(-100, "/tmp/b.txt", 0x41, 0o644).expect("valid OpenAtRequest");
        let parts_b: Vec<SystemCallMessagePart> = extract_parts(
            request_b
                .into_parts(source_b, ::syscall::LINUXD, MessageType::Ikc)
                .expect("valid parts"),
        );

        // Barrier: signal when Thread A has released the lock.
        let lock_released: Arc<tokio::sync::Notify> = Arc::new(tokio::sync::Notify::new());

        // Thread A: assemble under lock → drop lock → simulate slow process_request.
        let asm_a: Arc<Mutex<RequestAssembler>> = assembler.clone();
        let notify: Arc<tokio::sync::Notify> = lock_released.clone();
        let task_a: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            // Phase 1: assemble under lock (mirrors the fixed handle_long_request).
            {
                let mut guard: tokio::sync::MutexGuard<'_, RequestAssembler> = asm_a.lock().await;
                for part in parts_a {
                    let _: Option<OpenAtRequest> = guard
                        .assemble_and_take::<(), OpenAtRequest>(source_a, part)
                        .expect("assembly should succeed");
                }
            } // Lock dropped here — the core of the fix.

            notify.notify_one();

            // Phase 2: simulate blocking process_request (e.g., libc::openat).
            tokio::time::sleep(Duration::from_millis(500)).await;
        });

        // Wait until Thread A has released the lock.
        lock_released.notified().await;

        // Thread B: should acquire the lock almost immediately.
        let start: Instant = Instant::now();
        let asm_b: Arc<Mutex<RequestAssembler>> = assembler.clone();
        let task_b: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            let mut guard: tokio::sync::MutexGuard<'_, RequestAssembler> = asm_b.lock().await;
            for part in parts_b {
                let _: Option<OpenAtRequest> = guard
                    .assemble_and_take::<(), OpenAtRequest>(source_b, part)
                    .expect("assembly should succeed");
            }
        });

        task_b.await.expect("task_b should complete");
        let elapsed: Duration = start.elapsed();

        // If the lock were held during process_request (the old, buggy
        // pattern), this would take ~500 ms.  With the fix, it takes < 50 ms.
        assert!(
            elapsed < Duration::from_millis(100),
            "Thread B waited {}ms for the assembler lock — expected < 100ms. The lock is likely \
             still held during process_request (issue #1493).",
            elapsed.as_millis()
        );

        task_a.await.expect("task_a should complete");
    }

    /// Validates that a `SelectRequest` survives the linuxd receive path: split into
    /// `SelectRequestPart`s on the client side, then reassembled by the worker's
    /// `RequestAssembler` back into the original request (without executing `do_select`).
    #[test]
    #[allow(clippy::expect_used)]
    fn select_request_assembles_into_request() {
        use ::sysapi::sys_select::{
            fd_set,
            timeval,
        };
        use ::syscall::sys::select::message::SelectRequest;

        let source: ThreadIdentifier = ThreadIdentifier::from(7_i32);

        let mut readfds: fd_set = fd_set::default();
        readfds.set_bit(0).expect("fd in range");
        readfds.set_bit(5).expect("fd in range");
        let mut writefds: fd_set = fd_set::default();
        writefds.set_bit(2).expect("fd in range");
        let timeout: timeval = timeval {
            tv_sec: 1,
            tv_usec: 2,
        };

        let original: SelectRequest =
            SelectRequest::new(6, &Some(&mut readfds), &Some(&mut writefds), &None, &Some(timeout))
                .expect("valid SelectRequest");

        let parts: Vec<SystemCallMessagePart> = extract_parts(
            original
                .into_parts(source, ::syscall::LINUXD, MessageType::Ikc)
                .expect("valid parts"),
        );

        // The request is transported as a `SelectRequestPart` stream; the part count is derived
        // from the wire size and the per-part payload, so it stays correct across ABI/message-size
        // changes (including the single-part case). Feed the parts through the assembler in order.
        let expected_parts: usize =
            SelectRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        assert_eq!(parts.len(), expected_parts, "unexpected number of parts");

        let mut assembler: RequestAssembler = RequestAssembler::default();
        let mut result: Option<SelectRequest> = None;
        for part in parts {
            let r: Option<SelectRequest> = assembler
                .assemble_and_take::<(), SelectRequest>(source, part)
                .expect("assembly should succeed");
            if r.is_some() {
                result = r;
            }
        }

        let request: SelectRequest = result.expect("request should be fully assembled");
        assert_eq!(request.nfds, 6, "nfds mismatch");
        assert_eq!(
            request.readfds.map(|s| s.to_bytes()),
            Some(readfds.to_bytes()),
            "readfds mismatch"
        );
        assert_eq!(
            request.writefds.map(|s| s.to_bytes()),
            Some(writefds.to_bytes()),
            "writefds mismatch"
        );
        assert!(request.errorfds.is_none(), "errorfds should be absent");
        assert_eq!(request.timeout, Some(timeout.to_bytes()), "timeout mismatch");
    }
}
