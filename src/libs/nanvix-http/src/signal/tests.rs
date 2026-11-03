// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
