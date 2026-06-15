// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! The guest's standard-I/O endpoint for the IKC bridge.
//!
//! The IKC bridge ([`crate::ikc`]) routes the guest's `read(2)`/`write(2)` on
//! the standard file descriptors to a [`GuestIo`] endpoint. Two implementations
//! are provided:
//!
//! - [`HostGuestIo`] — the default: guest stdout/stderr go to the host's
//!   stdout/stderr, and guest stdin is fed by the streaming [`HostStdin`]
//!   reader. This is the terminal role of standalone `nanvixd`.
//! - [`ChannelGuestIo`] — an in-process endpoint that connects guest stdio to a
//!   pair of channels, letting a host-side harness exchange raw payloads with
//!   the guest with no operating-system pipe in between. This is what gives the
//!   warm-start benchmark white-box parity with the Nanvix `warm-start-vmm`
//!   micro-benchmark (which measures the round-trip latency in and out of the
//!   VMM over its in-process IKC channels).

use crate::stdin::HostStdin;
use ::std::{
    collections::VecDeque,
    io::Write as _,
    sync::mpsc::{
        Receiver,
        Sender,
    },
};

/// Standard output / standard error file descriptors.
const STDOUT_FILENO: i32 = 1;
const STDERR_FILENO: i32 = 2;

/// The guest's standard-I/O endpoint.
///
/// Implementations are driven on the single vCPU thread, so methods take
/// `&mut self` and may block (a blocking `read_stdin` parks the vCPU until input
/// is available, matching the guest's blocking-read semantics).
pub trait GuestIo: Send {
    /// Handles `data` the guest wrote to `fd`; returns the count written, or
    /// `-1` for an unsupported descriptor.
    fn write_stdout(&mut self, fd: i32, data: &[u8]) -> i32;

    /// Returns up to `max` bytes for a guest stdin read, blocking until data is
    /// available or end-of-file. An empty result signals EOF.
    fn read_stdin(&mut self, max: usize) -> Vec<u8>;
}

/// Default endpoint: bridges guest stdio to the host terminal.
pub struct HostGuestIo {
    stdin: HostStdin,
}

impl HostGuestIo {
    /// Creates a host endpoint fed by the given streaming stdin reader.
    pub fn new(stdin: HostStdin) -> Self {
        Self { stdin }
    }
}

impl GuestIo for HostGuestIo {
    fn write_stdout(&mut self, fd: i32, data: &[u8]) -> i32 {
        match fd {
            STDOUT_FILENO => {
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(data);
                let _ = out.flush();
                data.len() as i32
            },
            STDERR_FILENO => {
                let mut err = std::io::stderr().lock();
                let _ = err.write_all(data);
                let _ = err.flush();
                data.len() as i32
            },
            other => {
                log::warn!("rejecting write to unsupported fd {other}");
                -1
            },
        }
    }

    fn read_stdin(&mut self, max: usize) -> Vec<u8> {
        self.stdin.read_up_to(max)
    }
}

/// In-process endpoint that connects guest stdio to a [`GuestIoHandle`].
///
/// Guest writes are forwarded to the handle; guest reads block until the handle
/// supplies bytes (or is dropped, which signals EOF).
pub struct ChannelGuestIo {
    from_handle: Receiver<Vec<u8>>,
    to_handle: Sender<Vec<u8>>,
    pending: VecDeque<u8>,
}

/// The host-side counterpart of a [`ChannelGuestIo`].
pub struct GuestIoHandle {
    to_guest: Option<Sender<Vec<u8>>>,
    from_guest: Receiver<Vec<u8>>,
}

impl ChannelGuestIo {
    /// Creates a VM-side endpoint and its paired host-side handle.
    pub fn pair() -> (ChannelGuestIo, GuestIoHandle) {
        let (to_guest, from_handle) = ::std::sync::mpsc::channel();
        let (to_handle, from_guest) = ::std::sync::mpsc::channel();
        (
            ChannelGuestIo {
                from_handle,
                to_handle,
                pending: VecDeque::new(),
            },
            GuestIoHandle {
                to_guest: Some(to_guest),
                from_guest,
            },
        )
    }
}

impl GuestIo for ChannelGuestIo {
    fn write_stdout(&mut self, _fd: i32, data: &[u8]) -> i32 {
        let _ = self.to_handle.send(data.to_vec());
        data.len() as i32
    }

    fn read_stdin(&mut self, max: usize) -> Vec<u8> {
        if self.pending.is_empty() {
            match self.from_handle.recv() {
                Ok(data) => self.pending.extend(data),
                Err(_) => return Vec::new(), // handle dropped => EOF
            }
        }
        let n = self.pending.len().min(max);
        self.pending.drain(..n).collect()
    }
}

impl GuestIoHandle {
    /// Feeds `data` to the guest's subsequent stdin read(s).
    pub fn send(&self, data: &[u8]) {
        if let Some(tx) = &self.to_guest {
            let _ = tx.send(data.to_vec());
        }
    }

    /// Blocks until the guest has written `n` bytes in total. Returns `false` if
    /// the guest closed its output (EOF) before `n` bytes arrived.
    pub fn read_exact(&self, n: usize) -> bool {
        let mut got = 0usize;
        while got < n {
            match self.from_guest.recv() {
                Ok(chunk) => got += chunk.len(),
                Err(_) => return false,
            }
        }
        true
    }

    /// Closes the guest's input, causing its next blocking read to see EOF.
    pub fn close_input(&mut self) {
        self.to_guest = None;
    }

    /// Splits the handle into independent sender/receiver halves.
    ///
    /// This is what the HTTP-mode gateway bridge uses: the sender half is moved
    /// into the task that forwards the gateway connection to the guest's stdin,
    /// while the receiver half is drained (on a blocking task) to forward the
    /// guest's stdout back to the connection. Splitting is required because the
    /// two directions run concurrently and the underlying channels are not
    /// `Sync`.
    pub fn split(self) -> (GuestStdinSender, GuestStdoutReceiver) {
        (
            GuestStdinSender {
                to_guest: self.to_guest,
            },
            GuestStdoutReceiver {
                from_guest: self.from_guest,
            },
        )
    }
}

/// The stdin-feeding half of a split [`GuestIoHandle`].
pub struct GuestStdinSender {
    to_guest: Option<Sender<Vec<u8>>>,
}

impl GuestStdinSender {
    /// Feeds `data` to the guest's subsequent stdin read(s).
    pub fn send(&self, data: &[u8]) {
        if let Some(tx) = &self.to_guest {
            let _ = tx.send(data.to_vec());
        }
    }

    /// Closes the guest's input, causing its next blocking read to see EOF.
    pub fn close(self) {
        drop(self.to_guest);
    }
}

/// The stdout-draining half of a split [`GuestIoHandle`].
pub struct GuestStdoutReceiver {
    from_guest: Receiver<Vec<u8>>,
}

impl GuestStdoutReceiver {
    /// Blocks until the guest writes the next chunk of output. Returns `None`
    /// when the guest has closed its output (EOF).
    pub fn recv(&self) -> Option<Vec<u8>> {
        self.from_guest.recv().ok()
    }
}
