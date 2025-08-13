// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Arguments
//!
//! This module provides utilities for parsing command-line arguments that were supplied to the
//! program.
//!

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::{
    env,
    process,
    str::FromStr,
};
use ::syscomm::SocketType;

//==================================================================================================
// Public Structures
//==================================================================================================

///
/// # Description
///
/// This structure packs the command-line arguments that were passed to the program.
///
pub struct Args {
    /// Kernel filename.
    kernel_filename: String,
    /// Initrd filename.
    initrd_filename: Option<String>,
    /// Arguments to be passed to the initrd.
    initrd_args: Option<String>,
    /// Memory size.
    memory_size: usize,
    /// Standard error.
    vm_stderr: Option<String>,
    /// System VM address.
    system_vm_addr: Option<String>,
    /// System VM socket type.
    system_vm_socket_type: Option<String>,
    /// Control-plane address.
    control_plane_addr: Option<String>,
    /// Control-plane socket type.
    control_plane_socket_type: Option<SocketType>,
    /// Log to file?
    log_to_file: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    const OPT_HELP: &'static str = "-help";
    /// Command-line option for initrd file.
    const OPT_INITRD: &'static str = "-initrd";
    /// Command-line option for the kernel file.
    const OPT_KERNEL: &'static str = "-kernel";
    /// Command-line option for the memory size.
    const OPT_MEMORY_SIZE: &'static str = "-memory";
    /// Command-line option for the standard error.
    const OPT_STDERR: &'static str = "-stderr";
    /// Command-line option for system VM address.
    const OPT_SYSTEM_VM_SOCKADDR: &'static str = "-system-vm-addr";
    /// Command-line option for the system VM socket type.
    const OPT_SYSTEM_VM_SOCKET_TYPE: &'static str = "-system-vm-socket-type";
    /// Command-line option for control-plane address.
    const OPT_CONTROL_PLANE_SOCKADDR: &'static str = "-control-plane-addr";
    /// Command-line option for the control-plane socket type.
    const OPT_CONTROL_PLANE_SOCKET_TYPE: &'static str = "-control-plane-socket-type";
    /// Command-line option for specifying arguments to be passed to the initrd.
    const OPT_INITRD_ARGS: &'static str = "-initrd_args";
    /// Log to file.
    const OPT_LOGFILE: &'static str = "-log-to-file";

    ///
    /// # Description
    ///
    /// Parses the command-line arguments that were passed to the program.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the command-line arguments that were passed
    /// to the program. Otherwise, it returns an error.
    ///
    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut kernel_filename: String = String::new();
        let mut initrd_filename: Option<String> = None;
        let mut initrd_args: Option<String> = None;
        let mut memory_size: usize = ::config::kernel::MEMORY_SIZE;
        let mut vm_stderr: Option<String> = None;
        let mut system_vm_addr: Option<String> = None;
        let mut system_vm_socket_type: Option<String> = None;
        let mut control_plane_addr: Option<String> = None;
        let mut control_plane_socket_type: Option<SocketType> = None;
        let mut log_to_file: bool = false;

        // Parse command-line arguments.
        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                // Print help message and exit.
                Self::OPT_HELP => {
                    Self::usage();
                    process::exit(0);
                },
                // Set initrd file.
                Self::OPT_INITRD if i + 1 < args.len() => {
                    initrd_filename = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set initrd arguments.
                Self::OPT_INITRD_ARGS if i + 1 < args.len() => {
                    initrd_args = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set kernel file.
                Self::OPT_KERNEL if i + 1 < args.len() => {
                    kernel_filename = args[i + 1].clone();
                    i += 1;
                },
                // Set memory size.
                Self::OPT_MEMORY_SIZE if i + 1 < args.len() => {
                    let mem_arg: &String = &args[i + 1];

                    // Parse memory size.
                    memory_size = match mem_arg[..mem_arg.len() - 1].parse::<usize>() {
                        Ok(size) => size,
                        Err(e) => {
                            anyhow::bail!("invalid memory size (error={})", e);
                        },
                    };

                    // Parse memory size suffix.
                    let endptr: char = match mem_arg.chars().last() {
                        Some(c) => c,
                        None => {
                            anyhow::bail!("invalid memory size '{}'", mem_arg);
                        },
                    };
                    match endptr {
                        'K' | 'k' => memory_size *= 1024,
                        'M' | 'm' => memory_size *= 1024 * 1024,
                        'G' | 'g' => memory_size *= 1024 * 1024 * 1024,
                        ch => {
                            anyhow::bail!("invalid memory size suffix '{}'", ch);
                        },
                    }
                    i += 1;
                },
                // Set error file.
                Self::OPT_STDERR if i + 1 < args.len() => {
                    vm_stderr = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set system VM address.
                Self::OPT_SYSTEM_VM_SOCKADDR if i + 1 < args.len() => {
                    system_vm_addr = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set system VM socket type.
                Self::OPT_SYSTEM_VM_SOCKET_TYPE if i + 1 < args.len() => {
                    system_vm_socket_type = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set control-plane address.
                Self::OPT_CONTROL_PLANE_SOCKADDR if i + 1 < args.len() => {
                    control_plane_addr = Some(args[i + 1].clone());
                    i += 1;
                },
                // Set control-plane socket type.
                Self::OPT_CONTROL_PLANE_SOCKET_TYPE if i + 1 < args.len() => {
                    match SocketType::from_str(&args[i + 1]) {
                        Ok(typ) => control_plane_socket_type = Some(typ),
                        Err(_) => {
                            let reason: String =
                                format!("unrecognised socket type: {}", args[i + 1].clone());
                            error!("{reason}");
                            return Err(anyhow::anyhow!("{reason}"));
                        },
                    }
                    i += 1;
                },
                // Set log to file flag.
                Self::OPT_LOGFILE => {
                    log_to_file = true;
                },
                // Invalid argument.
                _ => {
                    Self::usage();
                    anyhow::bail!("invalid argument {}", args[i]);
                },
            }

            i += 1;
        }

        // Check if kernel file is missing.
        if kernel_filename.is_empty() {
            Self::usage();
            anyhow::bail!("kernel file is missing");
        }

        // Check if memory size is invalid.
        if memory_size == 0 {
            Self::usage();
            anyhow::bail!("invalid memory size");
        }

        Ok(Self {
            kernel_filename,
            initrd_filename,
            initrd_args,
            memory_size,
            vm_stderr,
            system_vm_addr,
            system_vm_socket_type,
            control_plane_addr,
            control_plane_socket_type,
            log_to_file,
        })
    }

    ///
    /// # Description
    ///
    /// Prints program usage.
    ///
    pub fn usage() {
        eprintln!(
            "Usage: {} {} <kernel> [{} <size>] [{} <file>] [{} <file>]  [{} <socket-address>] \
             [{}] [{} <args>]",
            env::args().next().unwrap_or("microvm".to_string()),
            Self::OPT_KERNEL,
            Self::OPT_MEMORY_SIZE,
            Self::OPT_INITRD,
            Self::OPT_STDERR,
            Self::OPT_SYSTEM_VM_SOCKADDR,
            Self::OPT_LOGFILE,
            Self::OPT_INITRD_ARGS,
        );
    }

    ///
    /// # Description
    ///
    /// Returns the initrd filename that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The initrd filename that was passed as a command-line argument to the program. If no initrd
    /// filename was passed, this method returns `None`.
    ///
    pub fn initrd_filename(&mut self) -> Option<String> {
        self.initrd_filename.take()
    }

    ///
    /// # Description
    ///
    /// Returns the initrd arguments that were passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The initrd arguments that were passed as a command-line argument to the program. If no
    /// initrd arguments were passed, this method returns `None`.
    ///
    pub fn initrd_args(&mut self) -> Option<String> {
        self.initrd_args.take()
    }

    ///
    /// # Description
    ///
    /// Returns the kernel filename that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The kernel filename that was passed as a command-line argument to the program.
    ///
    pub fn kernel_filename(&self) -> &str {
        &self.kernel_filename
    }

    ///
    /// # Description
    ///
    /// Returns the memory size that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The memory size that was passed as a command-line argument to the program.
    ///
    pub fn memory_size(&self) -> usize {
        self.memory_size
    }

    ///
    /// # Description
    ///
    /// Returns the name of the standard error file that was passed as a command-line argument to the
    /// program.
    ///
    /// # Returns
    ///
    /// The name of standard error file that was passed as a command-line argument to the program. If
    /// no standard error file was passed, this method returns `None`.
    ///
    pub fn take_vm_stderr(&mut self) -> Option<String> {
        self.vm_stderr.take()
    }

    ///
    /// # Description
    ///
    /// Returns the address of the system VM that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The system VM address that was passed as a command-line argument to the program.
    ///
    pub fn system_vm_addr(&mut self) -> Option<String> {
        self.system_vm_addr.take()
    }

    ///
    /// # Description
    ///
    /// Returns the socket address type of the system VM socket that was passed as a command-line
    /// argument to the program.
    ///
    /// # Returns
    ///
    /// The socket address type of the system VM socket that was passed as a command-line argument to
    /// the program.
    ///
    pub fn system_vm_socket_type(&mut self) -> Option<String> {
        self.system_vm_socket_type.take()
    }

    ///
    /// # Description
    ///
    /// Returns the address of the control-plane that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The control-plane address that was passed as a command-line argument to the program.
    ///
    pub fn control_plane_addr(&mut self) -> Option<String> {
        self.control_plane_addr.take()
    }

    ///
    /// # Description
    ///
    /// Returns the socket address type of the control-plane socket that was passed as a command-line
    /// argument to the program.
    ///
    /// # Returns
    ///
    /// The socket address type of the control-plane socket that was passed as a command-line argument to
    /// the program.
    ///
    pub fn control_plane_socket_type(&mut self) -> Option<SocketType> {
        self.control_plane_socket_type.take()
    }

    ///
    /// # Description
    ///
    /// Returns whether the program should log to a file or to the standard output.
    ///
    /// # Returns
    ///
    /// Whether the program should log to a file or to the standard output.
    ///
    pub fn log_to_file(&self) -> bool {
        self.log_to_file
    }
}
