// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! TCP port allocation and management.
//!
//! This module provides RAII-based TCP port allocation for L2 deployment mode. It maintains
//! a pool of available TCP ports and ensures proper cleanup when ports are no longer needed.
//! The port allocator uses a mutex-protected pool to safely manage ports across concurrent
//! operations.

//==================================================================================================
// Imports
//==================================================================================================

use ::log::{
    error,
    warn,
};
use ::std::{
    collections::VecDeque,
    fmt,
    sync::{
        Arc,
        Mutex,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Raw TCP port number.
///
pub type RawTcpPortNum = u16;

///
/// # Description
///
/// A TCP port wrapper that ensures ports are properly released when no longer needed.
///
/// This structure implements RAII semantics so that TCP ports allocated for L2 gateway
/// connections are automatically returned to the pool when the port is dropped.
///
pub struct TcpPort {
    port: RawTcpPortNum,
    allocator: TcpPortAllocatorInner,
}

impl TcpPort {
    ///
    /// # Description
    ///
    /// Creates a new TCP port.
    ///
    /// # Parameters
    ///
    /// - `port`: TCP port number.
    /// - `allocator`: TCP port allocator.
    ///
    /// # Returns
    ///
    /// A new TCP port.
    ///
    fn new(port: RawTcpPortNum, allocator: TcpPortAllocatorInner) -> Self {
        Self { port, allocator }
    }
}

impl fmt::Debug for TcpPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.port)
    }
}

impl Drop for TcpPort {
    ///
    /// # Description
    ///
    /// Automatically releases the TCP port back to the allocator when the instance is dropped.
    ///
    /// NOTE: This uses synchronous operations since Drop cannot be async. If we spawn an
    /// async task to do this, we risk the task not being executed if the runtime is shut down
    /// before the task runs.
    ///
    fn drop(&mut self) {
        self.allocator.release(self.port);
    }
}

///
/// # Description
///
/// Pool of available TCP ports.
///
#[derive(Clone)]
struct TcpPortAllocatorInner {
    ports: Arc<Mutex<VecDeque<RawTcpPortNum>>>,
}

impl TcpPortAllocatorInner {
    ///
    /// # Description
    ///
    /// Allocates a free TCP port from the pool.
    ///
    /// # Returns
    ///
    /// A TCP port if there are any in the pool, or `None` otherwise.
    ///
    fn allocate(&mut self) -> Option<RawTcpPortNum> {
        match self.ports.lock() {
            Ok(mut guard) => guard.pop_front(),
            Err(error) => {
                error!("allocate(): failed to acquire lock on port pool: {error}");
                None
            },
        }
    }

    ///
    /// # Description
    ///
    /// Returns a TCP port to the pool.
    ///
    /// # Parameters
    ///
    /// - `port`: The TCP port to return.
    ///
    fn release(&self, port: RawTcpPortNum) {
        match self.ports.lock() {
            Ok(mut guard) => guard.push_back(port),
            Err(error) => {
                warn!("release(): failed to acquire lock on port pool: {error}");
            },
        }
    }
}

///
/// # Description
///
/// Wrapper around the TCP port pool and allocator.
///
#[derive(Clone)]
pub struct TcpPortAllocator {
    inner: TcpPortAllocatorInner,
}

impl TcpPortAllocator {
    ///
    /// # Description
    ///
    /// Initializes a new TCP port allocator with the specified port range.
    ///
    /// # Parameters
    ///
    /// - `begin`: The beginning of the port range (inclusive).
    /// - `end`: The end of the port range (inclusive).
    ///
    /// # Returns
    ///
    /// A new TCP port allocator.
    ///
    pub fn new(begin: RawTcpPortNum, end: RawTcpPortNum) -> Self {
        let mut ports: VecDeque<RawTcpPortNum> =
            VecDeque::with_capacity((end - begin + 1) as usize);
        for port in begin..=end {
            ports.push_back(port);
        }

        Self {
            inner: TcpPortAllocatorInner {
                ports: Arc::new(Mutex::new(ports)),
            },
        }
    }

    ///
    /// # Description
    ///
    /// Allocates a TCP port and returns a RAII guard that will automatically release the port
    /// when dropped.
    ///
    /// This is a synchronous operation that completes in microseconds (mutex lock + Vec pop +
    /// unlock). It does not need to be wrapped in `tokio::task::spawn_blocking()` when called
    /// from async contexts because the mutex critical section is trivial and won't block the
    /// async runtime scheduler.
    ///
    /// # Returns
    ///
    /// A RAII wrapper around a raw TCP port, or `None` if no ports are available.
    ///
    pub fn allocate(&mut self) -> Option<TcpPort> {
        if let Some(port) = self.inner.allocate() {
            Some(TcpPort::new(port, self.inner.clone()))
        } else {
            None
        }
    }
}
