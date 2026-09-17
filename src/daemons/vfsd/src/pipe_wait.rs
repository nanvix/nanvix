// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Suspended pipe-request queues (the vfsd side of MINIX-style `suspend`/`revive`).
//!
//! vfsd must never block its own event loop. When a `read` finds an empty pipe (with writers still
//! open) or a `write` finds a full pipe, the caller is *parked* here instead of being answered, and
//! the client stays blocked inside its `__kcall_pull`/`__kcall_recv`. The complementary operation —
//! a later `write` for a parked reader, or a `read` for a parked writer — *revives* the parked
//! caller from [`super::handler::pipe`].
//!
//! State is keyed by the pipe's stable identity (`pipe_id`), which never recycles, so a stale
//! wakeup can never target the wrong pipe. Only IPC identity lives here; the buffer bytes live in
//! the VFS library next to the FD table.

extern crate alloc;

use crate::error::ResponseContext;
use ::alloc::{
    collections::{
        BTreeMap,
        BTreeSet,
        VecDeque,
    },
    vec::Vec,
};
use ::sys::{
    error::ErrorCode,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Blocked Requests
//==================================================================================================

/// A `read` request suspended on an empty pipe.
#[derive(Clone, Copy)]
pub(crate) struct BlockedReader {
    /// Exact response routing and correlation metadata for the original request.
    pub response_context: ResponseContext,
    /// Thread that issued the read, used by the push rendezvous and response builder.
    pub source_tid: ThreadIdentifier,
    /// Process that issued the read (needed to `__kcall_push` the data back to the caller).
    pub source_pid: ProcessIdentifier,
    /// Read-end file descriptor in the caller's process.
    pub fd: i32,
    /// Number of bytes the caller requested.
    pub count: usize,
    /// Error to report once the caller registers its pull.
    pub error: Option<ErrorCode>,
}

/// A `write` request suspended on a full pipe.
pub(crate) struct BlockedWriter {
    /// Exact response routing and correlation metadata for the original request.
    pub response_context: ResponseContext,
    /// Thread that issued the write, used by the response builder.
    pub source_tid: ThreadIdentifier,
    /// Process that issued the write (needed to resolve the write-end FD on revive).
    pub source_pid: ProcessIdentifier,
    /// Write-end file descriptor in the caller's process.
    pub fd: i32,
    /// Bytes already pulled from the caller but not yet fully buffered.
    pub data: Vec<u8>,
    /// Bytes of `data` accepted into the pipe buffer so far (for writes larger than `PIPE_BUF`).
    pub written: usize,
    /// Original request length, reported in the final `WriteResponse`.
    pub total: usize,
}

//==================================================================================================
// Wait Table
//==================================================================================================

/// Per-pipe FIFO queues of suspended readers and writers, keyed by `pipe_id`.
pub(crate) struct PipeWaitTable {
    readers: BTreeMap<u64, VecDeque<BlockedReader>>,
    writers: BTreeMap<u64, VecDeque<BlockedWriter>>,
    read_retries: BTreeSet<u64>,
}

impl PipeWaitTable {
    /// Creates an empty wait table.
    pub fn new() -> Self {
        Self {
            readers: BTreeMap::new(),
            writers: BTreeMap::new(),
            read_retries: BTreeSet::new(),
        }
    }

    /// Suspends a reader at the back of `pipe_id`'s reader queue.
    pub fn park_reader(&mut self, pipe_id: u64, reader: BlockedReader) {
        self.readers.entry(pipe_id).or_default().push_back(reader);
    }

    /// Suspends a writer at the back of `pipe_id`'s writer queue.
    pub fn park_writer(&mut self, pipe_id: u64, writer: BlockedWriter) {
        self.writers.entry(pipe_id).or_default().push_back(writer);
    }

    /// Returns a snapshot of the reader at the front of `pipe_id`'s queue, if any.
    pub fn front_reader(&self, pipe_id: u64) -> Option<BlockedReader> {
        self.readers.get(&pipe_id)?.front().copied()
    }

    /// Removes the reader at the front of `pipe_id`'s queue.
    pub fn pop_reader(&mut self, pipe_id: u64) -> Option<BlockedReader> {
        let queue: &mut VecDeque<BlockedReader> = self.readers.get_mut(&pipe_id)?;
        let reader: Option<BlockedReader> = queue.pop_front();
        if queue.is_empty() {
            self.readers.remove(&pipe_id);
        }
        reader
    }

    /// Returns a shared reference to the writer at the front of `pipe_id`'s queue, if any.
    pub fn front_writer(&self, pipe_id: u64) -> Option<&BlockedWriter> {
        self.writers.get(&pipe_id)?.front()
    }

    /// Returns the routing identity `(source_pid, fd)` of the front writer of `pipe_id`, if any.
    ///
    /// The values are copied out so the caller can use them as a `while let` loop condition without
    /// holding a borrow on the table across the body.
    pub fn front_writer_meta(&self, pipe_id: u64) -> Option<(ProcessIdentifier, i32)> {
        let w: &BlockedWriter = self.writers.get(&pipe_id)?.front()?;
        Some((w.source_pid, w.fd))
    }

    /// Returns a mutable reference to the writer at the front of `pipe_id`'s queue, if any.
    pub fn front_writer_mut(&mut self, pipe_id: u64) -> Option<&mut BlockedWriter> {
        self.writers.get_mut(&pipe_id)?.front_mut()
    }

    /// Removes the writer at the front of `pipe_id`'s queue.
    pub fn pop_writer(&mut self, pipe_id: u64) -> Option<BlockedWriter> {
        let queue: &mut VecDeque<BlockedWriter> = self.writers.get_mut(&pipe_id)?;
        let writer: Option<BlockedWriter> = queue.pop_front();
        if queue.is_empty() {
            self.writers.remove(&pipe_id);
        }
        writer
    }

    /// Removes and returns all writers suspended on `pipe_id` (used for `EPIPE` wakeups).
    pub fn drain_writers(&mut self, pipe_id: u64) -> VecDeque<BlockedWriter> {
        self.writers.remove(&pipe_id).unwrap_or_default()
    }

    /// Marks a pipe-read retry as pending.
    ///
    /// Returns `true` when the caller should enqueue a retry event, or `false` if one is already
    /// pending for `pipe_id`.
    pub fn schedule_read_retry(&mut self, pipe_id: u64) -> bool {
        self.read_retries.insert(pipe_id)
    }

    /// Consumes the pending pipe-read retry marker for `pipe_id`.
    pub fn consume_read_retry(&mut self, pipe_id: u64) {
        self.read_retries.remove(&pipe_id);
    }

    /// Cancels one suspended request identified by its exact response context.
    ///
    /// Returns the number of bytes already accepted for a cancelled writer, or zero for a
    /// cancelled reader. If the thread has no suspended pipe request, returns `None`.
    pub fn cancel(
        &mut self,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        request_id: ::sys::ipc::RequestIdentifier,
    ) -> Option<usize> {
        let mut transferred: Option<usize> = None;

        self.readers.retain(|_, queue| {
            queue.retain(|reader| {
                let matches: bool = reader.source_pid == pid
                    && reader.source_tid == tid
                    && reader.response_context.request_id() == request_id;
                if matches {
                    transferred = Some(0);
                }
                !matches
            });
            !queue.is_empty()
        });

        self.writers.retain(|_, queue| {
            queue.retain(|writer| {
                let matches: bool = writer.source_pid == pid
                    && writer.source_tid == tid
                    && writer.response_context.request_id() == request_id;
                if matches {
                    transferred = Some(writer.written);
                }
                !matches
            });
            !queue.is_empty()
        });

        transferred
    }

    /// Drops every suspended request issued by `pid`.
    ///
    /// Called when a process exits or replaces its image: its parked readers and writers can no
    /// longer be answered, so they are discarded before EOF/`EPIPE` wakeups run for its peers.
    pub fn purge_pid(&mut self, pid: ProcessIdentifier) {
        Self::purge_readers(&mut self.readers, pid);
        Self::purge_writers(&mut self.writers, pid);
    }

    /// Removes all readers belonging to `pid`, dropping now-empty per-pipe queues.
    fn purge_readers(map: &mut BTreeMap<u64, VecDeque<BlockedReader>>, pid: ProcessIdentifier) {
        map.retain(|_, queue| {
            queue.retain(|r| r.source_pid != pid);
            !queue.is_empty()
        });
    }

    /// Removes all writers belonging to `pid`, dropping now-empty per-pipe queues.
    fn purge_writers(map: &mut BTreeMap<u64, VecDeque<BlockedWriter>>, pid: ProcessIdentifier) {
        map.retain(|_, queue| {
            queue.retain(|w| w.source_pid != pid);
            !queue.is_empty()
        });
    }
}
