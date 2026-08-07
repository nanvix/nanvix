// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Suspended console readers awaiting cooked terminal input.

extern crate alloc;

use crate::error::ResponseContext;
use ::alloc::collections::VecDeque;
use ::sys::{
    error::ErrorCode,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

/// A blocking console read parked by VFSD.
#[derive(Clone, Copy)]
pub(crate) struct BlockedConsoleReader {
    /// Exact response routing and correlation metadata for the original request.
    pub response_context: ResponseContext,
    /// Process that issued the read.
    pub source_pid: ProcessIdentifier,
    /// Thread that issued the read.
    pub source_tid: ThreadIdentifier,
    /// Console descriptor in the caller's table.
    pub fd: i32,
    /// Maximum number of cooked bytes requested.
    pub count: usize,
    /// Error to report without consulting the terminal state.
    pub error: Option<ErrorCode>,
}

/// FIFO of blocking reads on the shared console input device.
pub(crate) struct ConsoleWaitTable {
    readers: VecDeque<BlockedConsoleReader>,
    input_available: bool,
    read_retry_pending: bool,
}

impl ConsoleWaitTable {
    /// Creates an empty wait table.
    pub fn new() -> Self {
        Self {
            readers: VecDeque::new(),
            input_available: false,
            read_retry_pending: false,
        }
    }

    /// Parks a reader at the back of the console queue.
    pub fn park(&mut self, reader: BlockedConsoleReader) {
        self.readers.push_back(reader);
    }

    /// Returns a snapshot of the front reader.
    pub fn front(&self) -> Option<BlockedConsoleReader> {
        self.readers.front().copied()
    }

    /// Removes the front reader.
    pub fn pop(&mut self) -> Option<BlockedConsoleReader> {
        self.readers.pop_front()
    }

    /// Cancels one parked read identified by its exact response context.
    pub fn cancel(
        &mut self,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        request_id: ::sys::ipc::RequestIdentifier,
    ) -> bool {
        let reader: Option<usize> = self.readers.iter().position(|reader| {
            reader.source_pid == pid
                && reader.source_tid == tid
                && reader.response_context.request_id() == request_id
        });
        if let Some(index) = reader {
            self.readers.remove(index);
            true
        } else {
            false
        }
    }

    /// Removes all requests belonging to a terminated process.
    pub fn purge_pid(&mut self, pid: ProcessIdentifier) {
        self.readers.retain(|reader| reader.source_pid != pid);
    }

    /// Records that UserVM has console input or EOF available for one snapshot.
    pub fn mark_input_available(&mut self) {
        self.input_available = true;
    }

    /// Consumes a pending input-availability token.
    pub fn take_input_available(&mut self) -> bool {
        core::mem::replace(&mut self.input_available, false)
    }

    /// Returns whether the front reader needs terminal input rather than a queued error.
    pub fn front_needs_input(&self) -> bool {
        matches!(self.readers.front(), Some(reader) if reader.error.is_none())
    }

    /// Marks a read-delivery retry pending and reports whether one should be enqueued.
    pub fn schedule_read_retry(&mut self) -> bool {
        if self.read_retry_pending {
            false
        } else {
            self.read_retry_pending = true;
            true
        }
    }

    /// Clears the pending read-delivery retry marker.
    pub fn consume_read_retry(&mut self) {
        self.read_retry_pending = false;
    }
}
