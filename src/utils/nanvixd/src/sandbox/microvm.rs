// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;
use ::std::process::{
    ExitStatus,
    Stdio,
};
use ::tokio::process::{
    Child,
    Command,
};

//==================================================================================================
// Structures
//==================================================================================================

pub struct Microvm(Option<Child>);

//==================================================================================================
// Implementations
//==================================================================================================

impl Microvm {
    pub fn spawn(program: &str, addr: &str, stderr: &str) -> Result<Self> {
        let child = Command::new(format!("{}/microvm.elf", config::BINARY_DIRECTORY))
            .arg("-log-to-file")
            .arg("-kernel")
            .arg(format!("{}/kernel.elf", config::BINARY_DIRECTORY))
            .arg("-initrd")
            .arg(program)
            .arg("-stderr")
            .arg(stderr)
            .arg("-gateway")
            .arg(addr)
            .stdout(Stdio::piped())
            .spawn()?;
        debug!(
            "spawning microvm child.pid={:?} program={:?} addr={:?} stderr={:?}",
            child.id(),
            program,
            addr,
            stderr
        );
        Ok(Self(Some(child)))
    }

    /// Peek Microvm to check if it is still running.
    pub fn peek(&mut self) -> Result<Option<ExitStatus>> {
        if let Some(child) = &mut self.0 {
            match child.try_wait() {
                Ok(Some(status)) => Ok(Some(status)),
                Ok(None) => Ok(None),
                Err(e) => {
                    let reason: String = format!("failed to wait for microvm (error={:?})", e);
                    error!("peek(): {:?}", reason);
                    Err(anyhow::anyhow!(reason))
                },
            }
        } else {
            Ok(None)
        }
    }
}

impl Drop for Microvm {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            tokio::spawn(async move {
                debug!("killing microvm (child={:?})", child.id());
                if let Err(e) = child.kill().await {
                    error!("failed to kill microvm (child={:?}, Arror={:?})", child.id(), e);
                }

                if let Err(e) = child.wait().await {
                    error!("failed to wait for microvm (child{:?}, error={:?})", child.id(), e);
                }
            });
        }
    }
}
