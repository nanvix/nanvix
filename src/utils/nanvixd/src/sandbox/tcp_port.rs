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

use ::std::{
    fmt,
    sync::Arc,
};
use ::tokio::sync::Mutex;

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
    fn drop(&mut self) {
        let allocator: TcpPortAllocatorInner = self.allocator.clone();
        let port: RawTcpPortNum = self.port;
        ::tokio::task::spawn(async move {
            allocator.release(port).await;
        });
    }
}

///
/// # Description
///
/// Pool of available TCP ports.
///
#[derive(Clone)]
struct TcpPortAllocatorInner {
    ports: Arc<Mutex<Vec<RawTcpPortNum>>>,
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
    async fn allocate(&mut self) -> Option<RawTcpPortNum> {
        self.ports.lock().await.pop()
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
    async fn release(&self, port: RawTcpPortNum) {
        self.ports.lock().await.push(port)
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
        let mut ports: Vec<RawTcpPortNum> = Vec::with_capacity((end - begin + 1) as usize);
        for port in begin..=end {
            ports.push(port);
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
    /// # Returns
    ///
    /// A RAII wrapper around a raw TCP port, or `None` if no ports are available.
    ///
    pub async fn allocate(&mut self) -> Option<TcpPort> {
        if let Some(port) = self.inner.allocate().await {
            Some(TcpPort::new(port, self.inner.clone()))
        } else {
            None
        }
    }
}
