// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Unix process management for nanvixd.

use tokio::process::Child;

/// Represents a running nanvixd process on Unix.
pub struct NanvixProcess {
    child: Option<Child>,
    pid_val: u32,
}

#[allow(dead_code)]
impl NanvixProcess {
    pub fn new(child: Child) -> Self {
        let pid = child.id().unwrap_or(0);
        Self {
            child: Some(child),
            pid_val: pid,
        }
    }

    /// Create a process handle from just a PID (child owned elsewhere).
    pub fn from_pid(pid: u32) -> Self {
        Self {
            child: None,
            pid_val: pid,
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid_val
    }

    pub async fn kill(&mut self, signal: u32) -> anyhow::Result<()> {
        let pid = self.pid_val;
        if pid == 0 {
            return Ok(());
        }

        let nix_pid = nix::unistd::Pid::from_raw(pid as i32);
        let nix_signal = nix::sys::signal::Signal::try_from(signal as i32)
            .unwrap_or(nix::sys::signal::Signal::SIGKILL);

        match nix::sys::signal::kill(nix_pid, nix_signal) {
            Ok(_) => Ok(()),
            Err(nix::errno::Errno::ESRCH) => Ok(()),
            Err(e) => {
                Err(anyhow::anyhow!("failed to send signal {} to pid {}: {}", signal, pid, e))
            },
        }
    }

    pub async fn wait(&mut self) -> u32 {
        if let Some(ref mut child) = self.child {
            match child.wait().await {
                Ok(status) => status.code().unwrap_or(1) as u32,
                Err(e) => {
                    log::error!("failed to wait for nanvixd: {}", e);
                    1
                },
            }
        } else {
            0
        }
    }
}
