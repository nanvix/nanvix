// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! # Platform abstraction layer for Windows
//!
//! This module provides platform-specific functionalities for Windows-based systems.
//!

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::std::{
    collections::VecDeque,
    fs::{
        self,
        File,
    },
    os::windows::io::AsRawHandle,
    path::Path,
    sync::{
        Arc,
        Condvar,
        Mutex,
        MutexGuard,
    },
    thread::{
        self,
        JoinHandle,
    },
    time::{
        Duration,
        Instant,
    },
};
use ::sys::ipc::{
    DataChunkHeader,
    IkcFrame,
    Message,
};
use ::tokio::sync::mpsc::Sender;
use ::windows::Win32::{
    Foundation::{
        CloseHandle,
        HANDLE,
    },
    System::Memory::{
        CreateFileMappingW,
        FILE_MAP_READ,
        MEMORY_MAPPED_VIEW_ADDRESS,
        MapViewOfFile,
        PAGE_READONLY,
        UnmapViewOfFile,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

/// A memory-mapped file.
pub struct FileMapping {
    /// Section handle returned by `CreateFileMappingW`.
    section_handle: HANDLE,
    /// File view mapped into host address space by `MapViewOfFile`.
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    /// Size of the mapped region (in bytes).
    size: usize,
}

// SAFETY: `FileMapping` owns OS handles (section handle, mapped view) that have no thread
// affinity. All mutation requires `&mut self`, and resources are released exactly once in `Drop`.
unsafe impl Send for FileMapping {}
unsafe impl Sync for FileMapping {}

//==================================================================================================
// Implementations
//==================================================================================================

impl FileMapping {
    ///
    /// # Description
    ///
    /// Maps a file into memory (read-only).
    ///
    /// # Parameters
    ///
    /// * `filename` - Name of the file to be loaded.
    ///
    /// # Returns
    ///
    /// On success, this function returns an object representing the memory-mapped file. On failure,
    /// an error object that describes the error is returned instead.
    ///
    pub fn open(filename: &str) -> Result<Self> {
        trace!("open(): filename={filename}");

        let path: &Path = Path::new(filename);

        let file: File = fs::File::open(path).map_err(|e| {
            let reason: String = format!("failed to open file (error={e})");
            error!("open(): {reason} (filename={filename})");
            anyhow::anyhow!(reason)
        })?;

        let size: usize = usize::try_from(
            file.metadata()
                .map_err(|e| {
                    let reason: String = format!("failed to get file metadata (error={e})");
                    error!("open(): {reason} (filename={filename})");
                    anyhow::anyhow!(reason)
                })?
                .len(),
        )
        .map_err(|_| {
            let reason: &str = "file size exceeds addressable range";
            error!("open(): {reason} (filename={filename})");
            anyhow::anyhow!(reason)
        })?;

        if size == 0 {
            let reason: &str = "cannot map zero-sized file";
            error!("open(): {reason} (filename={filename})");
            anyhow::bail!(reason);
        }

        let file_handle: HANDLE = HANDLE(file.as_raw_handle());

        // NOTE: `file` is dropped at the end of this function, but the section handle created below
        // keeps an internal kernel reference to the underlying file object.  The OS file handle can
        // therefore be closed safely; the mapping stays valid until `section_handle` itself is
        // closed in `Drop`.
        //
        // SAFETY: `file_handle` is a valid OS handle obtained from the `File` opened above.
        // Passing `None` for security attributes and name is permitted. Size parameters of
        // (0, 0) tell the OS to use the file's actual size.
        let section_handle: HANDLE = unsafe {
            CreateFileMappingW(file_handle, None, PAGE_READONLY, 0, 0, None).map_err(|e| {
                let reason: String = format!("failed to create file mapping (error={e:?})");
                error!("open(): {reason} (filename={filename})");
                anyhow::anyhow!(reason)
            })?
        };

        // SAFETY: `section_handle` is a valid section handle from the successful
        // `CreateFileMappingW` call above. `size` equals the file size obtained from metadata.
        let view: MEMORY_MAPPED_VIEW_ADDRESS =
            unsafe { MapViewOfFile(section_handle, FILE_MAP_READ, 0, 0, size) };

        if view.Value.is_null() {
            // SAFETY: `section_handle` is a valid handle from `CreateFileMappingW`; it must be
            // closed before returning, since the view creation failed.
            unsafe {
                if CloseHandle(section_handle).is_err() {
                    warn!("open(): CloseHandle() failed while cleaning up section handle");
                }
            }
            let reason: &str = "MapViewOfFile returned null";
            error!("open(): {reason} (filename={filename})");
            anyhow::bail!(reason);
        }

        Ok(Self {
            section_handle,
            view,
            size,
        })
    }

    ///
    /// # Description
    ///
    /// Returns a pointer to the mapped file data.
    ///
    /// # Returns
    ///
    /// A pointer to the file data.
    ///
    pub fn ptr(&self) -> *const u8 {
        self.view.Value as *const u8
    }

    ///
    /// # Description
    ///
    /// Returns the size of the mapped file (in bytes).
    ///
    /// # Returns
    ///
    /// The size of the file (in bytes).
    ///
    pub fn size(&self) -> usize {
        self.size
    }

    ///
    /// # Description
    ///
    /// Returns the mapped file contents as an immutable byte slice.
    ///
    /// # Returns
    ///
    /// An immutable byte slice covering the entire mapped file.
    ///
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: The mapping is valid for `self.size` bytes for the lifetime of `self`.
        unsafe { ::std::slice::from_raw_parts(self.view.Value as *const u8, self.size) }
    }
}

impl ::std::fmt::Debug for FileMapping {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.debug_struct("FileMapping")
            .field("view", &self.view.Value)
            .field("size", &self.size)
            .finish()
    }
}

impl Drop for FileMapping {
    fn drop(&mut self) {
        trace!("drop(): FileMapping (size={})", self.size);
        // SAFETY: `self.view` and `self.section_handle` are valid OS resources created in
        // `open()`. They are released exactly once here and not used after this point.
        unsafe {
            if let Err(e) = UnmapViewOfFile(self.view) {
                error!("drop(): UnmapViewOfFile failed (error={e:?})");
            }
            if let Err(e) = CloseHandle(self.section_handle) {
                error!("drop(): CloseHandle failed on section handle (error={e:?})");
            }
        }
    }
}

//==================================================================================================
// IKC Stdout Sink
//==================================================================================================

/// Upper bound on how long [`IkcStdoutSink`]'s drop handler waits for the drain thread to flush and
/// stop before detaching it. A wedged drain thread (blocked in a host send against a consumer that
/// never drains) must not block VM teardown.
const DRAIN_JOIN_GRACE: Duration = Duration::from_secs(2);

/// Maximum host memory budget used by [`IkcStdoutSink`] for IKC frames that have been accepted by
/// the producer but not yet handed to the downstream bounded channel.
const IKC_SINK_BUDGET_BYTES: usize = 64 * 1024 * 1024;

///
/// # Description
///
/// State shared between the producer (vCPU thread) and the drain thread.
///
struct IkcSinkState {
    /// IKC frames awaiting hand-off to the bounded channel.
    data: VecDeque<IkcFrame>,
    /// Total accounted bytes accepted by the producer but not yet handed to the bounded channel.
    buffered_bytes: usize,
    /// Set when the producer is shutting down; the drain thread flushes then exits.
    closing: bool,
    /// Set once the drain thread has stopped (clean exit or the downstream channel closed).
    stopped: bool,
}

///
/// # Description
///
/// Synchronization primitives shared with the drain thread.
///
struct IkcSinkChannel {
    /// Shared mutable state.
    state: Mutex<IkcSinkState>,
    /// Signaled when a frame is enqueued or `closing` is set.
    nonempty: Condvar,
    /// Signaled when the drain thread stops.
    stopped: Condvar,
}

///
/// # Description
///
/// Decouples the guest's vmbus stdout (application/IKC) output path from the WHP partition lock.
///
/// On Windows/WHP the vCPU run loop holds the partition lock across
/// `Emulator::handle_pmio_access()`. For a guest `write()` (vmbus stdout port, Dword width) that
/// call invokes the closure built by `output_fn()`, which forwards the IKC frame to the
/// asynchronous I/O consumer over a bounded channel. Sending directly with
/// [`Sender::blocking_send()`] blocks the vCPU thread while it holds the partition lock whenever
/// the consumer is momentarily starved (for example a tokio task starved under heavy host load on
/// Windows), freezing the guest — see issue #2603. This is the same coupling that issue #2579 /
/// PR #2583 fixed for the per-byte console path (port `0xE9`).
///
/// [`IkcStdoutSink`] breaks this coupling for the application path: [`IkcStdoutSink::send()`] only
/// appends the frame to a budgeted in-process queue and returns immediately (it never blocks),
/// while a dedicated drain thread drains the queue to the bounded channel off the partition lock.
/// Back-pressure is therefore applied to the drain thread, never to the vCPU thread, so the guest
/// is never gated on the host's ability to drain the channel. If the consumer remains starved long
/// enough to exhaust the in-process budget, `send()` returns an error instead of growing host
/// memory without bound.
///
pub struct IkcStdoutSink {
    /// Shared channel between the producer and the drain thread.
    channel: Arc<IkcSinkChannel>,
    /// Handle to the drain thread, joined on drop.
    handle: Option<JoinHandle<()>>,
}

impl IkcSinkChannel {
    ///
    /// # Description
    ///
    /// Locks the shared state, recovering the guard if the mutex was poisoned.
    ///
    /// # Returns
    ///
    /// A guard for the shared state.
    ///
    fn lock(&self) -> MutexGuard<'_, IkcSinkState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl IkcStdoutSink {
    ///
    /// # Description
    ///
    /// Wraps `queue` so that IKC frames are decoupled from the caller's lock. A dedicated thread
    /// owns `queue` and performs the (potentially blocking) [`Sender::blocking_send()`].
    ///
    /// # Parameters
    ///
    /// - `queue`: Bounded channel to which buffered frames are forwarded.
    ///
    /// # Returns
    ///
    /// A new [`IkcStdoutSink`].
    ///
    pub fn new(queue: Sender<IkcFrame>) -> Self {
        let channel: Arc<IkcSinkChannel> = Arc::new(IkcSinkChannel {
            state: Mutex::new(IkcSinkState {
                data: VecDeque::new(),
                buffered_bytes: 0,
                closing: false,
                stopped: false,
            }),
            nonempty: Condvar::new(),
            stopped: Condvar::new(),
        });

        let drain_channel: Arc<IkcSinkChannel> = channel.clone();
        let handle: Option<JoinHandle<()>> = match thread::Builder::new()
            .name("nanvix-ikc".to_string())
            .spawn(move || drain_loop(&drain_channel, queue))
        {
            Ok(handle) => Some(handle),
            // If the drain thread could not be spawned, log the underlying I/O error and mark the
            // sink stopped so that `send()` surfaces the failure instead of buffering frames
            // forever behind a thread that does not exist.
            Err(e) => {
                error!("IkcStdoutSink::new(): failed to spawn IKC drain thread: {e}");
                channel.lock().stopped = true;
                None
            },
        };

        Self { channel, handle }
    }

    ///
    /// # Description
    ///
    /// Appends a frame to the in-process queue for the drain thread. This never blocks, so the vCPU
    /// thread is never stalled on host I/O while it holds the partition lock.
    ///
    /// # Parameters
    ///
    /// - `frame`: IKC frame emitted by the guest.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the frame has been queued, or an error if the drain thread has stopped because
    /// the downstream consumer went away or if the in-process queue budget is exhausted.
    ///
    pub fn send(&self, frame: IkcFrame) -> Result<()> {
        let mut state: MutexGuard<'_, IkcSinkState> = self.channel.lock();

        // If the drain thread is gone, the downstream consumer has closed; surface the failure so
        // the emulator unwinds, matching the prior `blocking_send()` error behavior.
        if state.stopped {
            anyhow::bail!("IKC stdout sink drain thread has stopped (downstream closed)");
        }

        let frame_size: usize = frame_size_bytes(&frame);
        if frame_size > IKC_SINK_BUDGET_BYTES
            || state.buffered_bytes > IKC_SINK_BUDGET_BYTES - frame_size
        {
            anyhow::bail!(
                "IKC stdout sink queue budget exceeded (buffered_bytes={}, frame_bytes={}, \
                 budget_bytes={})",
                state.buffered_bytes,
                frame_size,
                IKC_SINK_BUDGET_BYTES
            );
        }

        let was_empty: bool = state.data.is_empty();
        state.buffered_bytes += frame_size;
        state.data.push_back(frame);
        drop(state);

        // Wake the drain thread only on an empty -> non-empty transition: while the queue is
        // non-empty the drain thread is guaranteed to revisit it without an explicit wakeup.
        if was_empty {
            self.channel.nonempty.notify_one();
        }

        Ok(())
    }
}

impl Drop for IkcStdoutSink {
    fn drop(&mut self) {
        // Signal shutdown and wake the drain thread so it flushes buffered frames.
        self.channel.lock().closing = true;
        self.channel.nonempty.notify_one();

        // Wait briefly for the drain thread to finish flushing. If it is wedged in a blocking send
        // against a consumer that never drains, do NOT block teardown on it: leave it detached so
        // that process exit reclaims it. Joining unconditionally here would reintroduce the very
        // hang this sink exists to prevent.
        let stopped: bool = {
            let mut state: MutexGuard<'_, IkcSinkState> = self.channel.lock();
            let deadline: Instant = Instant::now() + DRAIN_JOIN_GRACE;
            while !state.stopped {
                let now: Instant = Instant::now();
                if now >= deadline {
                    break;
                }
                let (guard, _timeout) = self
                    .channel
                    .stopped
                    .wait_timeout(state, deadline - now)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state = guard;
            }
            state.stopped
        };

        if stopped {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        } else {
            warn!(
                "IkcStdoutSink: IKC drain thread still busy at shutdown; detaching to avoid \
                 blocking VM teardown"
            );
        }
    }
}

///
/// # Description
///
/// Drains buffered frames to `downstream` until the sink is closing and the queue is empty.
///
/// # Parameters
///
/// - `channel`: Shared channel between the producer and the drain thread.
/// - `downstream`: Bounded channel owned by the drain thread.
///
fn drain_loop(channel: &Arc<IkcSinkChannel>, downstream: Sender<IkcFrame>) {
    let mut scratch: Vec<IkcFrame> = Vec::new();
    loop {
        let mut state: MutexGuard<'_, IkcSinkState> = channel.lock();
        while state.data.is_empty() && !state.closing {
            state = channel
                .nonempty
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        if state.data.is_empty() && state.closing {
            state.stopped = true;
            channel.stopped.notify_all();
            return;
        }
        scratch.clear();
        scratch.extend(state.data.drain(..));
        drop(state);

        // Forward the batch off the lock. Each `blocking_send()` may block while the bounded
        // channel is full, applying back-pressure here instead of on the vCPU thread.
        for frame in scratch.drain(..) {
            let frame_size: usize = frame_size_bytes(&frame);
            if downstream.blocking_send(frame).is_err() {
                // Downstream consumer is gone; nothing more can be delivered.
                let mut state: MutexGuard<'_, IkcSinkState> = channel.lock();
                state.stopped = true;
                channel.stopped.notify_all();
                drop(state);
                warn!("IkcStdoutSink: downstream channel closed; remaining IKC frames discarded");
                return;
            }

            let mut state: MutexGuard<'_, IkcSinkState> = channel.lock();
            state.buffered_bytes = state.buffered_bytes.saturating_sub(frame_size);
        }
    }
}

///
/// # Description
///
/// Returns the number of host bytes charged against [`IkcStdoutSink`]'s in-process budget for
/// `frame`.
///
/// # Parameters
///
/// - `frame`: IKC frame to account.
///
/// # Returns
///
/// The fixed IKC message size or the data chunk header plus payload size.
///
fn frame_size_bytes(frame: &IkcFrame) -> usize {
    match frame {
        IkcFrame::Message(_) => ::std::mem::size_of::<Message>(),
        IkcFrame::Bulk(chunk) => DataChunkHeader::SIZE + chunk.data().len(),
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

//==================================================================================================
// IKC Stdout Sink Tests
//==================================================================================================

/// Unit tests for the asynchronous IKC stdout sink.
///
/// These reproduce the issue #2603 hang mechanism deterministically and verify that the sink
/// decouples the producer from back-pressure without dropping or reordering frames.
#[cfg(test)]
#[allow(clippy::expect_used)]
mod ikc_tests;
