// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Shutdown signals of the HTTP server.
//!
//! This module listens for the signals that ask the Nanvix Daemon to shut down. When one of them
//! arrives, the HTTP server stops serving requests and tears down the running VM, which releases
//! host resources that would otherwise outlive the daemon, such as the socket file of the gateway
//! endpoint. Signals that are not handled here keep their default disposition, which terminates
//! the process without any cleanup.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::log::error;
#[cfg(unix)]
use ::std::{
    future::poll_fn,
    task::Poll,
};
#[cfg(unix)]
use ::tokio::signal::unix::{
    signal,
    Signal,
    SignalKind,
};
#[cfg(windows)]
use ::tokio::signal::windows::{
    ctrl_c,
    CtrlC,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Signals that trigger a graceful shutdown on Unix, along with their names.
///
/// Besides `SIGINT`, which is sent on Ctrl-C, these are `SIGTERM`, which is the default signal of
/// `kill(1)` and the one that service managers and container runtimes send to stop a process,
/// `SIGHUP`, which is sent when the controlling terminal goes away, and `SIGQUIT`, which is sent
/// on Ctrl-\.
#[cfg(unix)]
const SHUTDOWN_SIGNALS: [(SignalKind, &str); 4] = [
    (SignalKind::interrupt(), "SIGINT"),
    (SignalKind::terminate(), "SIGTERM"),
    (SignalKind::hangup(), "SIGHUP"),
    (SignalKind::quit(), "SIGQUIT"),
];

/// Name of the console event that triggers a graceful shutdown on Windows.
#[cfg(windows)]
const CTRL_C_EVENT_NAME: &str = "Ctrl-C";

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Listener for the signals that trigger a graceful shutdown of the HTTP server.
///
/// Handlers for all shutdown signals are installed when the listener is created, and they stay
/// installed for the remaining lifetime of the process. A signal that arrives while the listener
/// is not being polled is not lost: it is reported by the next call to [`ShutdownSignal::recv`].
///
pub(crate) struct ShutdownSignal {
    /// Streams of the shutdown signals, along with their names.
    #[cfg(unix)]
    streams: Vec<(Signal, &'static str)>,
    /// Stream of Ctrl-C events.
    #[cfg(windows)]
    ctrl_c: CtrlC,
}

//==================================================================================================
// Implementations
//==================================================================================================

#[cfg(unix)]
impl ShutdownSignal {
    ///
    /// # Description
    ///
    /// Installs handlers for `SIGINT`, `SIGTERM`, `SIGHUP`, and `SIGQUIT`.
    ///
    /// # Returns
    ///
    /// On success, returns a listener for the shutdown signals. On failure, returns an error
    /// describing which handler could not be installed.
    ///
    pub(crate) fn new() -> Result<Self> {
        let mut streams: Vec<(Signal, &'static str)> = Vec::with_capacity(SHUTDOWN_SIGNALS.len());
        for (kind, name) in SHUTDOWN_SIGNALS {
            match signal(kind) {
                Ok(stream) => streams.push((stream, name)),
                Err(error) => {
                    let reason: String = format!("failed to install {name} handler: {error}");
                    error!("new(): {reason}");
                    anyhow::bail!(reason);
                },
            }
        }
        Ok(Self { streams })
    }

    ///
    /// # Description
    ///
    /// Waits for the next shutdown signal.
    ///
    /// This method is cancel safe: if it is dropped before completing, no signal is lost.
    ///
    /// # Returns
    ///
    /// The name of the signal that was received.
    ///
    pub(crate) async fn recv(&mut self) -> &'static str {
        poll_fn(|cx| {
            for (stream, name) in self.streams.iter_mut() {
                // A stream also completes once the signal driver shuts down, after which no more
                // signals can be received, so shutting down is the only sensible option left.
                if stream.poll_recv(cx).is_ready() {
                    return Poll::Ready(*name);
                }
            }
            Poll::Pending
        })
        .await
    }
}

#[cfg(windows)]
impl ShutdownSignal {
    ///
    /// # Description
    ///
    /// Installs a handler for Ctrl-C events.
    ///
    /// # Returns
    ///
    /// On success, returns a listener for the shutdown signals. On failure, returns an error
    /// describing why the handler could not be installed.
    ///
    pub(crate) fn new() -> Result<Self> {
        match ctrl_c() {
            Ok(ctrl_c) => Ok(Self { ctrl_c }),
            Err(error) => {
                let reason: String =
                    format!("failed to install {CTRL_C_EVENT_NAME} handler: {error}");
                error!("new(): {reason}");
                anyhow::bail!(reason);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Waits for the next Ctrl-C event.
    ///
    /// This method is cancel safe: if it is dropped before completing, no event is lost.
    ///
    /// # Returns
    ///
    /// The name of the event that was received.
    ///
    pub(crate) async fn recv(&mut self) -> &'static str {
        // The stream also completes once the signal driver shuts down, after which no more events
        // can be received, so shutting down is the only sensible option left.
        let _ = self.ctrl_c.recv().await;
        CTRL_C_EVENT_NAME
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use ::std::time::Duration;
    use ::tokio::time::timeout;

    /// Maximum time to wait for a raised signal to be reported.
    const SIGNAL_TIMEOUT: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn recv_reports_each_shutdown_signal() {
        let signals: [(::libc::c_int, &str); 4] = [
            (::libc::SIGINT, "SIGINT"),
            (::libc::SIGTERM, "SIGTERM"),
            (::libc::SIGHUP, "SIGHUP"),
            (::libc::SIGQUIT, "SIGQUIT"),
        ];
        for (signum, name) in signals {
            let mut shutdown_signal: ShutdownSignal =
                ShutdownSignal::new().expect("failed to install shutdown signal handlers");
            // SAFETY: raise() has no memory-safety preconditions. The signal is caught by the
            // handler installed above, so it does not terminate the test process.
            let ret: ::libc::c_int = unsafe { ::libc::raise(signum) };
            assert_eq!(ret, 0, "failed to raise {name}");
            let received: &str = timeout(SIGNAL_TIMEOUT, shutdown_signal.recv())
                .await
                .unwrap_or_else(|_| panic!("{name} was not reported within {SIGNAL_TIMEOUT:?}"));
            assert_eq!(received, name);
        }
    }
}
