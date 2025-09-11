// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
/// A TCP port wrapper that ensures that we do not leak ports after a user VM dies.
///
pub struct TcpPort {
    port: RawTcpPortNum,
    allocator: TcpPortAllocatorInner,
}

impl TcpPort {
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
/// Pool of TCP ports that are ready to be used.
///
#[derive(Clone)]
struct TcpPortAllocatorInner {
    ports: Arc<Mutex<Vec<RawTcpPortNum>>>,
}

impl TcpPortAllocatorInner {
    ///
    /// # Description
    ///
    /// Allocate a free TCP port from the pool.
    ///
    /// # Returns
    ///
    /// A TCP port if there are any in the pool, None otherwise.
    ///
    async fn allocate(&mut self) -> Option<RawTcpPortNum> {
        self.ports.lock().await.pop()
    }

    ///
    /// # Description
    ///
    /// Return a TCP port to the pool.
    ///
    /// # Arguments
    ///
    /// The TCP port to return.
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
    /// Initialize a new TCP port pool and allocator.
    ///
    /// # Arguments
    ///
    /// - begin: the beginning of the port range we can use.
    /// - end: the ending of the port range we can use.
    ///
    /// # Returns
    ///
    /// A wrapper around the TCP port pool allocator.
    ///
    fn new(begin: RawTcpPortNum, end: RawTcpPortNum) -> Self {
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
    /// Allocate a TCP port and return a RAII guard.
    ///
    /// # Returns
    ///
    /// A RAII wrapper around a raw TCP port.
    ///
    pub async fn allocate(&mut self) -> Option<TcpPort> {
        if let Some(port) = self.inner.allocate().await {
            Some(TcpPort::new(port, self.inner.clone()))
        } else {
            None
        }
    }
}

///
/// # Description
///
/// Global static TCP port pool and allocator.
///
pub static TCP_PORT_ALLOCATOR: ::std::sync::OnceLock<Arc<Mutex<TcpPortAllocator>>> =
    ::std::sync::OnceLock::new();

///
/// # Description
///
/// Initializes the global TCP port allocator if it hasn't been initialized yet.
///
/// # Returns
///
/// A reference to the global TCP port allocator.
///
pub fn get_tcp_port_allocator() -> &'static Arc<Mutex<TcpPortAllocator>> {
    TCP_PORT_ALLOCATOR.get_or_init(|| {
        Arc::new(Mutex::new(TcpPortAllocator::new(
            ::config::linuxd::GATEWAY_PORT_RANGE_BEGIN,
            ::config::linuxd::GATEWAY_PORT_RANGE_END,
        )))
    })
}
