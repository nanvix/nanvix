// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::config;
use ::anyhow::Result;
use ::hwloc::HwLoc;
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
    pub fn spawn(program: &str, program_args: Option<&str>, addr: &str, stderr: Option<&str>, hwloc: Option<HwLoc>) -> Result<Self> {
        let mut user_vm_args: Vec<String> = vec![
            format!("{}/microvm.elf", config::BINARY_DIRECTORY),
            "-log-to-file".to_string(),
            "-kernel".to_string(),
            format!("{}/kernel.elf", config::BINARY_DIRECTORY),
            "-initrd".to_string(),
            program.to_string(),
            "-gateway".to_string(),
            addr.to_string(),
        ];

        if let Some(program_args) = program_args {
            user_vm_args.push("-initrd_args".to_string());
            user_vm_args.push(program_args.to_string());
        }

        if let Some(stderr_file) = stderr {
            user_vm_args.push("-stderr".to_string());
            user_vm_args.push(stderr_file.to_string());
        }

        if let Some(hwloc) = hwloc {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_nanovm_core_str(),
            ];
            user_vm_args.splice(0..0, taskset);
        }

        let child = Command::new(&user_vm_args[0])
            .args(&user_vm_args[1..])
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
                    let ret_code = unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) };

                    if ret_code < 0 {
                        error!("error sending SIGINT to user VM: {}", std::io::Error::last_os_error());
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
