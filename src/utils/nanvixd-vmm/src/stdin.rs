// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Streaming host-stdin channel for the guest's `read(2)` bridge.
//!
//! The guest reads its standard input through the IKC bridge. To support
//! interactive and streaming workloads (e.g. the VFS benchmark, which sends one
//! request at a time and waits for the reply before sending the next), host
//! stdin must be delivered to the guest *incrementally* rather than buffered up
//! front. A background thread reads the host's standard input and appends it to
//! a shared queue; the guest's blocking `read` waits on a condition variable
//! until data is available or stdin reaches end-of-file.
//!
//! This mirrors the Nanvix `uservm` standalone handler, whose read path waits on
//! its input channel when the buffer is empty and the channel is still open, and
//! reports EOF once it closes.

use ::std::{
    collections::VecDeque,
    sync::{
        Arc,
        Condvar,
        Mutex,
    },
};

/// Shared, growable buffer of host stdin bytes plus an end-of-file flag.
struct State {
    data: VecDeque<u8>,
    closed: bool,
}

struct Inner {
    state: Mutex<State>,
    cond: Condvar,
}

/// A handle to the host's standard input, fed by a background reader thread.
#[derive(Clone)]
pub struct HostStdin {
    inner: Arc<Inner>,
}

impl HostStdin {
    /// Spawns the background reader thread and returns a handle.
    ///
    /// The thread reads the process's standard input in chunks until EOF (or an
    /// error), making each chunk immediately available to [`Self::read_up_to`].
    pub fn spawn() -> Self {
        let inner = Arc::new(Inner {
            state: Mutex::new(State {
                data: VecDeque::new(),
                closed: false,
            }),
            cond: Condvar::new(),
        });

        let reader = inner.clone();
        std::thread::spawn(move || {
            use std::io::Read as _;
            let mut stdin = std::io::stdin();
            let mut buf = [0u8; 4096];
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut state = reader.state.lock().expect("host stdin lock");
                        state.data.extend(&buf[..n]);
                        reader.cond.notify_all();
                    },
                    Err(_) => break,
                }
            }
            let mut state = reader.state.lock().expect("host stdin lock");
            state.closed = true;
            reader.cond.notify_all();
        });

        Self { inner }
    }

    /// Returns up to `max` bytes of host stdin.
    ///
    /// Blocks until at least one byte is available, returning it immediately
    /// (without waiting to fill `max`). Returns an empty vector only once stdin
    /// has reached end-of-file, which the caller surfaces to the guest as EOF.
    pub fn read_up_to(&self, max: usize) -> Vec<u8> {
        let mut state = self.inner.state.lock().expect("host stdin lock");
        loop {
            if !state.data.is_empty() {
                let n = state.data.len().min(max);
                return state.data.drain(..n).collect();
            }
            if state.closed {
                return Vec::new();
            }
            state = self.inner.cond.wait(state).expect("host stdin wait");
        }
    }
}
