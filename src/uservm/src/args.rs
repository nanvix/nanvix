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
    path::{
        Path,
        PathBuf,
    },
    process,
};
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Public Structures
//==================================================================================================

///
/// # Description
///
/// This structure packs the command-line arguments that were passed to the program.
///
pub struct Args {
    /// Unique identifier for this VM.
    user_vm_id: UserVmIdentifier,
    /// Kernel filename.
    kernel_filename: String,
    /// Initrd filename.
    initrd_filename: Option<String>,
    /// Ramfs filename.
    ramfs_filename: Option<String>,
    /// Arguments to be passed to the initrd.
    initrd_args: Option<String>,
    /// Memory size.
    memory_size: usize,
    /// Standard error.
    vm_stderr: Option<String>,
    /// System VM address.
    system_vm_addr: String,
    /// Control-plane address.
    control_plane_addr: String,
    /// Socket address exposed in the system VM for users to connect to the user VM's stdin/stdout.
    gateway_addr: String,
    /// Log to file?
    log_to_file: bool,
    /// Log directory.
    log_directory: String,
    /// Socket address type of the system VM socket.
    system_vm_socket_type: String,
    /// Socket address type of the control-plane socket.
    control_plane_socket_type: String,
    /// Socket address type of the gateway socket.
    gateway_socket_type: String,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    pub const OPT_HELP: &'static str = "-help";
    /// Command-line option for id.
    pub const OPT_USER_VM_ID: &'static str = "-user-vm-id";
    /// Command-line option for initrd file.
    pub const OPT_INITRD: &'static str = "-initrd";
    /// Command-line option for the kernel file.
    pub const OPT_KERNEL: &'static str = "-kernel";
    /// Command-line option for the memory size.
    pub const OPT_MEMORY_SIZE: &'static str = "-memory";
    /// Command-line option for the standard error.
    pub const OPT_STDERR: &'static str = "-stderr";
    /// Command-line option for system VM address.
    pub const OPT_SYSTEM_VM_SOCKADDR: &'static str = "-system-vm-addr";
    /// Command-line option for the system VM socket type.
    pub const OPT_SYSTEM_VM_SOCKET_TYPE: &'static str = "-system-vm-socket-type";
    /// Command-line option for control-plane address.
    pub const OPT_CONTROL_PLANE_SOCKADDR: &'static str = "-control-plane-addr";
    /// Command-line option for the control-plane socket type.
    pub const OPT_CONTROL_PLANE_SOCKET_TYPE: &'static str = "-control-plane-socket-type";
    /// Command-line option for setting socket address of the gateway.
    pub const OPT_GATEWAY_SOCKADDR: &'static str = "-gateway-addr";
    /// Command-line option for setting the socket address type of the gateway socket.
    pub const OPT_GATEWAY_SOCKET_TYPE: &'static str = "-gateway-bind-socket-type";
    /// Command-line option for specifying arguments to be passed to the initrd.
    pub const OPT_INITRD_ARGS: &'static str = "-initrd_args";
    /// Command-line option for the ramfs file.
    pub const OPT_RAMFS: &'static str = "-ramfs";
    /// Log to file.
    pub const OPT_LOGFILE: &'static str = "-log-to-file";
    /// Log directory
    pub const OPT_LOGDIR: &'static str = "-log-dir";

    /// Program name.
    const PROGRAM_NAME: &'static str = env!("CARGO_PKG_NAME");

    /// Default log directory.
    const DEFAULT_LOG_DIRECTORY: &'static str = "logs";

    /// Test log file name for validation.
    const TEST_LOG_FILENAME: &'static str = "test.log";

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
        let mut user_vm_id_raw: Option<u32> = None;
        let mut kernel_filename: String = String::new();
        let mut initrd_filename: Option<String> = None;
        let mut ramfs_filename: Option<String> = None;
        let mut initrd_args: Option<String> = None;
        let mut memory_size: usize = ::config::kernel::MEMORY_SIZE;
        let mut vm_stderr: Option<String> = None;
        let mut system_vm_addr: String = String::new();
        let mut control_plane_addr: String = String::new();
        let mut gateway_addr: String = String::new();
        let mut log_to_file: bool = false;
        let mut log_directory: Option<String> = None;
        let mut system_vm_socket_type: String = String::new();
        let mut control_plane_socket_type: String = String::new();
        let mut gateway_socket_type: String = String::new();

        // Parse command-line arguments.
        let mut i: usize = 1;
        while i < args.len() {
            match args[i].as_str() {
                // Print help message and exit.
                Self::OPT_HELP => {
                    Self::usage();
                    process::exit(0);
                },
                // Parse user VM ID.
                Self::OPT_USER_VM_ID if i + 1 < args.len() => {
                    let user_vm_id_arg: &String = &args[i + 1];

                    // Parse memory size.
                    user_vm_id_raw = match user_vm_id_arg.parse::<u32>() {
                        Ok(id) => Some(id),
                        Err(e) => {
                            anyhow::bail!("invalid user vm id (arg={user_vm_id_arg}, error={e:?})");
                        },
                    };
                    i += 1;
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
                // Set ramfs file.
                Self::OPT_RAMFS if i + 1 < args.len() => {
                    ramfs_filename = Some(args[i + 1].clone());
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
                        'K' | 'k' => {
                            memory_size = memory_size
                                .checked_mul(1024)
                                .ok_or_else(|| anyhow::anyhow!("memory size overflow"))?;
                        },
                        'M' | 'm' => {
                            memory_size = memory_size
                                .checked_mul(1024)
                                .ok_or_else(|| anyhow::anyhow!("memory size overflow"))?
                                .checked_mul(1024)
                                .ok_or_else(|| anyhow::anyhow!("memory size overflow"))?;
                        },
                        'G' | 'g' => {
                            memory_size = memory_size
                                .checked_mul(1024)
                                .ok_or_else(|| anyhow::anyhow!("memory size overflow"))?
                                .checked_mul(1024)
                                .ok_or_else(|| anyhow::anyhow!("memory size overflow"))?
                                .checked_mul(1024)
                                .ok_or_else(|| anyhow::anyhow!("memory size overflow"))?;
                        },
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
                    system_vm_addr = args[i + 1].clone();
                    i += 1;
                },
                // Set system VM socket type.
                Self::OPT_SYSTEM_VM_SOCKET_TYPE if i + 1 < args.len() => {
                    system_vm_socket_type = args[i + 1].clone();
                    i += 1;
                },
                // Set control-plane address.
                Self::OPT_CONTROL_PLANE_SOCKADDR if i + 1 < args.len() => {
                    control_plane_addr = args[i + 1].clone();
                    i += 1;
                },
                // Set control-plane socket type.
                Self::OPT_CONTROL_PLANE_SOCKET_TYPE if i + 1 < args.len() => {
                    control_plane_socket_type = args[i + 1].clone();
                    i += 1;
                },
                // Set gateway address.
                Self::OPT_GATEWAY_SOCKADDR if i + 1 < args.len() => {
                    gateway_addr = args[i + 1].clone();
                    i += 1;
                },
                // Set gateway socket type.
                Self::OPT_GATEWAY_SOCKET_TYPE if i + 1 < args.len() => {
                    gateway_socket_type = args[i + 1].clone();
                    i += 1;
                },
                // Set log to file flag.
                Self::OPT_LOGFILE => {
                    log_to_file = true;
                },
                // Set log directory
                Self::OPT_LOGDIR if i + 1 < args.len() => {
                    log_directory = Some(args[i + 1].clone());
                    i += 1;
                },
                // Invalid argument.
                arg => {
                    Self::usage();
                    anyhow::bail!("invalid argument {}", arg);
                },
            }

            i += 1;
        }

        // Parse user VM ID.
        let user_vm_id: UserVmIdentifier = match user_vm_id_raw {
            Some(id) => UserVmIdentifier::new(id),
            None => {
                Self::usage();
                anyhow::bail!("user vm id is missing");
            },
        };

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

        // Check if gateway address is missing.
        if gateway_addr.is_empty() {
            Self::usage();
            anyhow::bail!("gateway address is missing");
        }

        // Check if gateway socket type is missing.
        if gateway_socket_type.is_empty() {
            Self::usage();
            anyhow::bail!("gateway socket type is missing");
        }

        // Check if control-plane address is missing.
        if control_plane_addr.is_empty() {
            Self::usage();
            anyhow::bail!("control-plane address is missing");
        }

        // Check if control-plane socket type is missing.
        if control_plane_socket_type.is_empty() {
            Self::usage();
            anyhow::bail!("control-plane socket type is missing");
        }

        // Check if system VM address is missing.
        if system_vm_addr.is_empty() {
            Self::usage();
            anyhow::bail!("system VM address is missing");
        }

        // Check if system VM socket type is missing.
        if system_vm_socket_type.is_empty() {
            Self::usage();
            anyhow::bail!("system VM socket type is missing");
        }

        // Check if log file directory was set if logging to file is enabled. Set the default directory if not.
        let log_directory: String = match (log_to_file, log_directory) {
            (true, Some(path)) => path,
            // Default to log dir relative to the current working directory, make the directory path absolute.
            (true, None) => {
                let mut abs_path: PathBuf = std::env::current_dir().map_err(|e| {
                    anyhow::anyhow!("failed to get current directory (error={:?})", e)
                })?;
                abs_path.push(Self::DEFAULT_LOG_DIRECTORY);
                abs_path.to_str().map(|s| s.to_string()).ok_or_else(|| {
                    anyhow::anyhow!("failed to convert log directory path to string")
                })?
            },
            (false, _) => String::new(),
        };

        // Validate that the path to the log file exists if logging to file is enabled.
        if log_to_file {
            let path: &Path = Path::new(&log_directory);
            // Create the directory if it does not exist.
            if !path.exists() {
                std::fs::create_dir_all(path).map_err(|e| {
                    anyhow::anyhow!(
                        "failed to create log file directory (path={}, error={:?})",
                        path.display(),
                        e
                    )
                })?;
            }
            // Check if we can create and write a file in the directory.
            // TODO: Use a random string for the test file name to avoid collisions.
            let test_file_path: PathBuf = path.join(Self::TEST_LOG_FILENAME);
            match std::fs::File::create(&test_file_path) {
                Ok(_file) => {
                    // Clean up the test file.
                    std::fs::remove_file(&test_file_path).ok();
                },
                Err(error) => {
                    return Err(anyhow::anyhow!(
                        "failed to create log file (path={}, error={:?})",
                        test_file_path.display(),
                        error
                    ));
                },
            }
        }

        Ok(Self {
            user_vm_id,
            kernel_filename,
            initrd_filename,
            ramfs_filename,
            initrd_args,
            memory_size,
            vm_stderr,
            system_vm_addr,
            control_plane_addr,
            gateway_addr,
            log_to_file,
            log_directory,
            system_vm_socket_type,
            control_plane_socket_type,
            gateway_socket_type,
        })
    }

    ///
    /// # Description
    ///
    /// Prints program usage.
    ///
    pub fn usage() {
        eprintln!(
            "Usage: {} {} <id> {} <kernel> [{} <size>] [{} <file>] [{} <file>]  [{} \
             <system-vm-addr> {} <control-plane-addr> {} <gateway-addr>] [{} [{} <dir>]] [{} \
             <args>] [{} <file>]",
            Self::PROGRAM_NAME,
            Self::OPT_USER_VM_ID,
            Self::OPT_KERNEL,
            Self::OPT_MEMORY_SIZE,
            Self::OPT_INITRD,
            Self::OPT_STDERR,
            Self::OPT_SYSTEM_VM_SOCKADDR,
            Self::OPT_CONTROL_PLANE_SOCKADDR,
            Self::OPT_GATEWAY_SOCKADDR,
            Self::OPT_LOGFILE,
            Self::OPT_LOGDIR,
            Self::OPT_INITRD_ARGS,
            Self::OPT_RAMFS,
        );
    }

    ///
    /// # Description
    ///
    /// Returns the user VM ID that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The ID of the user VM.
    ///
    pub fn user_vm_id(&self) -> UserVmIdentifier {
        self.user_vm_id
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
    /// Returns the ramfs filename that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The ramfs filename that was passed as a command-line argument to the program. If no ramfs
    /// filename was passed, this method returns `None`.
    ///
    pub fn ramfs_filename(&mut self) -> Option<String> {
        self.ramfs_filename.take()
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
    pub fn system_vm_addr(&self) -> &str {
        &self.system_vm_addr
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
    pub fn control_plane_addr(&self) -> &str {
        &self.control_plane_addr
    }

    ///
    /// # Description
    ///
    /// Returns the address of the gateway that was passed as a command-line argument to the program.
    ///
    /// # Returns
    ///
    /// The gateway address that was passed as a command-line argument to the program.
    ///
    pub fn gateway_addr(&self) -> &str {
        &self.gateway_addr
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
    ///
    /// # Description
    ///
    /// Returns the log directory.
    ///
    /// # Returns
    ///
    /// The log directory.
    ///
    pub fn log_directory(&self) -> String {
        self.log_directory.clone()
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
    pub fn system_vm_socket_type(&self) -> &str {
        &self.system_vm_socket_type
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
    pub fn control_plane_socket_type(&self) -> &str {
        &self.control_plane_socket_type
    }

    ///
    /// # Description
    ///
    /// Returns the socket address type of the gateway socket that was passed as a command-line
    /// argument to the program.
    ///
    /// # Returns
    ///
    /// The socket address type of the gateway socket that was passed as a command-line argument to
    /// the program.
    ///
    pub fn gateway_socket_type(&self) -> &str {
        &self.gateway_socket_type
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::anyhow::Result as AnyResult;
    use ::std::{
        env,
        fs,
        path::PathBuf,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    fn build_base_args() -> Vec<String> {
        vec![
            String::from("uservm"),
            Args::OPT_USER_VM_ID.to_string(),
            String::from("7"),
            Args::OPT_KERNEL.to_string(),
            String::from("kernel.elf"),
            Args::OPT_SYSTEM_VM_SOCKADDR.to_string(),
            String::from("127.0.0.1:7000"),
            Args::OPT_SYSTEM_VM_SOCKET_TYPE.to_string(),
            String::from("tcp"),
            Args::OPT_CONTROL_PLANE_SOCKADDR.to_string(),
            String::from("127.0.0.1:8000"),
            Args::OPT_CONTROL_PLANE_SOCKET_TYPE.to_string(),
            String::from("tcp"),
            Args::OPT_GATEWAY_SOCKADDR.to_string(),
            String::from("127.0.0.1:9000"),
            Args::OPT_GATEWAY_SOCKET_TYPE.to_string(),
            String::from("tcp"),
        ]
    }

    fn unique_log_dir() -> AnyResult<(String, PathBuf)> {
        let base_dir: PathBuf = env::temp_dir();
        let timestamp_nanos: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| ::anyhow::anyhow!("failed to compute timestamp (error={:?})", error))?
            .as_nanos();
        let dir_name: String = format!("nanvix-uservm-args-test-{}", timestamp_nanos);
        let log_dir: PathBuf = base_dir.join(dir_name);
        if log_dir.exists() {
            fs::remove_dir_all(&log_dir).ok();
        }
        let log_dir_str: String = log_dir.to_string_lossy().into_owned();
        Ok((log_dir_str, log_dir))
    }

    #[test]
    fn parse_returns_expected_values() -> AnyResult<()> {
        let mut args_vec: Vec<String> = build_base_args();
        let (log_dir_str, log_dir_path) = unique_log_dir()?;
        args_vec.push(Args::OPT_MEMORY_SIZE.to_string());
        args_vec.push(String::from("64M"));
        args_vec.push(Args::OPT_INITRD.to_string());
        args_vec.push(String::from("initrd.img"));
        args_vec.push(Args::OPT_INITRD_ARGS.to_string());
        args_vec.push(String::from("--flag=value"));
        args_vec.push(Args::OPT_RAMFS.to_string());
        args_vec.push(String::from("ramfs.img"));
        args_vec.push(Args::OPT_STDERR.to_string());
        args_vec.push(String::from("stderr.log"));
        args_vec.push(Args::OPT_LOGFILE.to_string());
        args_vec.push(Args::OPT_LOGDIR.to_string());
        args_vec.push(log_dir_str.clone());

        let mut parsed_args: Args = Args::parse(args_vec)?;

        assert_eq!(format!("{}", parsed_args.user_vm_id()), "7");
        assert_eq!(parsed_args.kernel_filename(), "kernel.elf");
        assert_eq!(parsed_args.memory_size(), 64 * 1024 * 1024);
        let initrd: Option<String> = parsed_args.initrd_filename();
        assert!(matches!(initrd, Some(ref value) if value == "initrd.img"));
        let ramfs: Option<String> = parsed_args.ramfs_filename();
        assert!(matches!(ramfs, Some(ref value) if value == "ramfs.img"));
        let initrd_args: Option<String> = parsed_args.initrd_args();
        assert!(matches!(initrd_args, Some(ref value) if value == "--flag=value"));
        let stderr_path: Option<String> = parsed_args.take_vm_stderr();
        assert!(matches!(stderr_path, Some(ref value) if value == "stderr.log"));
        assert_eq!(parsed_args.system_vm_addr(), "127.0.0.1:7000");
        assert_eq!(parsed_args.system_vm_socket_type(), "tcp");
        assert_eq!(parsed_args.control_plane_addr(), "127.0.0.1:8000");
        assert_eq!(parsed_args.control_plane_socket_type(), "tcp");
        assert_eq!(parsed_args.gateway_addr(), "127.0.0.1:9000");
        assert_eq!(parsed_args.gateway_socket_type(), "tcp");
        assert!(parsed_args.log_to_file());
        assert_eq!(parsed_args.log_directory(), log_dir_str);

        fs::remove_dir_all(log_dir_path).ok();

        Ok(())
    }

    #[test]
    fn parse_detects_memory_overflow() {
        let mut args_vec: Vec<String> = build_base_args();
        let overflow_arg: String = format!("{}K", ::std::usize::MAX);
        args_vec.push(Args::OPT_MEMORY_SIZE.to_string());
        args_vec.push(overflow_arg);

        match Args::parse(args_vec) {
            Err(error) => {
                assert!(error.to_string().contains("memory size overflow"));
            },
            Ok(_) => {
                assert!(false, "expected memory size overflow to produce an error");
            },
        }
    }
}
