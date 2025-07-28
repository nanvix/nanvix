// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;
use ::nix::{
    sys::signal::{
        Signal,
        kill,
    },
    unistd::Pid,
};
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
    pub fn spawn(program: &str, program_args: Option<&str>, addr: &str, stderr: Option<&str>) -> Result<Self> {
        let mut cmd = Command::new(format!("{}/microvm.elf", config::BINARY_DIRECTORY));

        cmd
            .arg("-log-to-file")
            .arg("-kernel")
            .arg(format!("{}/kernel.elf", config::BINARY_DIRECTORY))
            .arg("-initrd")
            .arg(program)
            .arg("-gateway")
            .arg(addr);

        if let Some(program_args) = program_args {
            cmd.arg("-initrd_args").arg(program_args);
        }

        if let Some(stderr_file) = stderr {
            cmd.arg("-stderr").arg(stderr_file);
        }

        let child = cmd
            .stdout(Stdio::piped())
            .spawn()?;

        debug!(
            "spawning microvm child.pid={:?} program={:?} args={:?} addr={:?} stderr={:?}",
            child.id(),
            program,
            program_args,
            addr,
            stderr
        );

        Ok(Self(Some(child)))
    }
}

impl Drop for Microvm {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            match child.id() {
                Some(pid) => {
                    let pid: ::sys::pm::ProcessIdentifier = match pid.try_into() {
                        Ok(pid) => pid,
                        Err(e) => return error!("error converting micro VMs PID (error={e:?})"),
                    };
                    if let Err(e) = kill(Pid::from_raw(pid.into()), Signal::SIGINT) {
                        error!("error sending SIGINT to user VM (error={e:?})");
                    }
                },
                None => error!("user VM process has no PID"),
            }

            // Wait for the child to finish in a separate thread to be able to `await` on it.
            tokio::spawn(async move {
                if let Err(e) = child.wait().await {
                    error!("failed to wait for user VM to shut down (error={e:?})");
                }
            });
        }
    }
}
