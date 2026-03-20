// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Windows process management for nanvixd.

use tokio::process::Child;

pub struct NanvixProcess {
    child: Option<Child>,
    pid_val: u32,
}

impl NanvixProcess {
    pub fn new(child: Child) -> Self {
        let pid = child.id().unwrap_or(0);
        Self {
            child: Some(child),
            pid_val: pid,
        }
    }

    pub fn from_pid(pid: u32) -> Self {
        Self {
            child: None,
            pid_val: pid,
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid_val
    }

    pub async fn kill(&mut self, _signal: u32) -> anyhow::Result<()> {
        if let Some(ref mut child) = self.child {
            child.kill().await?;
        }
        Ok(())
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
