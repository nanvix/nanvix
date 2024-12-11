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

pub struct Microvm(Option<Child>);

//==================================================================================================
// Implementations
//==================================================================================================

impl Microvm {
    pub fn spawn(program: &str, addr: &str, stderr: &str) -> Result<Self> {
        debug!("spawning microvm{program} {addr} {stderr}");
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
        Ok(Self(Some(child)))
    }
}

impl Drop for Microvm {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            tokio::spawn(async move {
                debug!("killing microvm");
                if let Err(e) = child.kill().await {
                    error!("failed to kill microvm({:?})", e);
                }

                if let Err(e) = child.wait().await {
                    error!("failed to wait for microvm({:?})", e);
                }
            });
        }
    }
}
