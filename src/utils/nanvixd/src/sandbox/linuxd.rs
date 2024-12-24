// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;
use ::std::process::Stdio;
use ::tokio::process::{
    Child,
    Command,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct LinuxDaemon(Option<Child>);

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemon {
    pub fn spawn(linuxd_sockaddr: &str, sandbox_sockaddr: &str) -> Result<Self> {
        debug!("spawning linux daemon {linuxd_sockaddr} {sandbox_sockaddr}");
        let child = Command::new(format!("{}/linuxd.elf", config::BINARY_DIRECTORY))
            .arg("-log-to-file")
            .arg("-bind-addr")
            .arg(linuxd_sockaddr)
            .arg("-gateway-addr")
            .arg(sandbox_sockaddr)
            .stdout(Stdio::piped())
            .spawn()?;
        Ok(Self(Some(child)))
    }
}

impl Drop for LinuxDaemon {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            tokio::spawn(async move {
                debug!("killing linux daemon");
                if let Err(e) = child.kill().await {
                    error!("failed to kill linux daemon ({:?})", e);
                }

                if let Err(e) = child.wait().await {
                    error!("failed to wait for linux daemon ({:?})", e);
                }
            });
        }
    }
}
