// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Command-line argument parsing for the UserVM executable.

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
use ::syslog::DEFAULT_LOG_DIRECTORY;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

/// Command-line arguments for the UserVM executable.
pub struct Args {
    /// Unique identifier for this VM.
    user_vm_id: UserVmIdentifier,
    /// Kernel filename.
    kernel_filename: String,
    /// Initrd filename.
    initrd_filename: Option<String>,
    /// RAM filesystem filename.
    ramfs_filename: Option<String>,
    /// Arguments passed to the initrd.
    initrd_args: Option<String>,
    /// Arguments passed to the kernel.
    kernel_args: Option<String>,
    /// Standard error output file.
    vm_stderr: Option<String>,
    /// Whether logs are written to files.
    log_to_file: bool,
    /// Log directory.
    log_directory: String,
    /// Optional snapshot restore path.
    snapshot_path: Option<String>,
    /// Optional GDB server port.
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Args {
    /// Command-line option for printing the help message.
    pub const OPT_HELP: &'static str = "-help";
    /// Command-line option for the user VM identifier.
    pub const OPT_USER_VM_ID: &'static str = "-user-vm-id";
    /// Command-line option for the initrd file.
    pub const OPT_INITRD: &'static str = "-initrd";
    /// Command-line option for the kernel file.
    pub const OPT_KERNEL: &'static str = "-kernel";
    /// Command-line option for the standard error output file.
    pub const OPT_STDERR: &'static str = "-stderr";
    /// Command-line option for initrd arguments.
    pub const OPT_INITRD_ARGS: &'static str = "-initrd_args";
    /// Command-line option for kernel arguments.
    pub const OPT_KERNEL_ARGS: &'static str = "-kernel-args";
    /// Command-line option for the RAM filesystem file.
    pub const OPT_RAMFS: &'static str = "-ramfs";
    /// Command-line option for file logging.
    pub const OPT_LOGFILE: &'static str = "-log-to-file";
    /// Command-line option for the log directory.
    pub const OPT_LOGDIR: &'static str = "-log-dir";
    /// Command-line option for the snapshot restore path.
    pub const OPT_SNAPSHOT: &'static str = "-snapshot";
    /// Command-line option for the GDB server port.
    #[cfg(feature = "gdb")]
    pub const OPT_GDB_PORT: &'static str = "-gdb-port";

    const PROGRAM_NAME: &'static str = env!("CARGO_PKG_NAME");
    const TEST_LOG_FILENAME: &'static str = "test.log";

    ///
    /// # Description
    ///
    /// Parses UserVM command-line arguments.
    ///
    /// # Parameters
    ///
    /// - `args`: Raw process arguments.
    ///
    /// # Returns
    ///
    /// On success, returns parsed arguments. On failure, returns an error.
    ///
    pub fn parse(args: Vec<String>) -> Result<Self> {
        let mut user_vm_id: UserVmIdentifier = UserVmIdentifier::new(0);
        let mut kernel_filename: String = String::new();
        let mut initrd_filename: Option<String> = None;
        let mut ramfs_filename: Option<String> = None;
        let mut initrd_args: Option<String> = None;
        let mut kernel_args: Option<String> = None;
        let mut vm_stderr: Option<String> = None;
        let mut log_to_file: bool = false;
        let mut log_directory: Option<String> = None;
        let mut snapshot_path: Option<String> = None;
        #[cfg(feature = "gdb")]
        let mut gdb_port: Option<u16> = None;

        let mut index: usize = 1;
        while index < args.len() {
            match args[index].as_str() {
                Self::OPT_HELP => {
                    Self::usage();
                    process::exit(0);
                },
                Self::OPT_USER_VM_ID if index + 1 < args.len() => {
                    user_vm_id =
                        UserVmIdentifier::new(args[index + 1].parse::<u32>().map_err(|error| {
                            ::anyhow::anyhow!(
                                "invalid user VM identifier (arg={}, error={error})",
                                args[index + 1]
                            )
                        })?);
                    index += 1;
                },
                Self::OPT_KERNEL if index + 1 < args.len() => {
                    kernel_filename = args[index + 1].clone();
                    index += 1;
                },
                Self::OPT_INITRD if index + 1 < args.len() => {
                    initrd_filename = Some(args[index + 1].clone());
                    index += 1;
                },
                Self::OPT_INITRD_ARGS if index + 1 < args.len() => {
                    initrd_args = Some(args[index + 1].clone());
                    index += 1;
                },
                Self::OPT_KERNEL_ARGS if index + 1 < args.len() => {
                    kernel_args = Some(args[index + 1].clone());
                    index += 1;
                },
                Self::OPT_RAMFS if index + 1 < args.len() => {
                    ramfs_filename = Some(args[index + 1].clone());
                    index += 1;
                },
                Self::OPT_STDERR if index + 1 < args.len() => {
                    vm_stderr = Some(args[index + 1].clone());
                    index += 1;
                },
                Self::OPT_SNAPSHOT if index + 1 < args.len() => {
                    snapshot_path = Some(args[index + 1].clone());
                    index += 1;
                },
                #[cfg(feature = "gdb")]
                Self::OPT_GDB_PORT if index + 1 < args.len() => {
                    gdb_port = Some(args[index + 1].parse::<u16>().map_err(|error| {
                        ::anyhow::anyhow!(
                            "invalid GDB port (arg={}, error={error})",
                            args[index + 1]
                        )
                    })?);
                    index += 1;
                },
                Self::OPT_LOGFILE => {
                    log_to_file = true;
                },
                Self::OPT_LOGDIR if index + 1 < args.len() => {
                    log_directory = Some(args[index + 1].clone());
                    index += 1;
                },
                argument => {
                    Self::usage();
                    anyhow::bail!("invalid argument {argument}");
                },
            }
            index += 1;
        }

        if kernel_filename.is_empty() {
            Self::usage();
            anyhow::bail!("kernel file is missing");
        }

        let log_directory: String = match (log_to_file, log_directory) {
            (true, Some(path)) => path,
            (true, None) => {
                let mut path: PathBuf = env::current_dir().map_err(|error| {
                    ::anyhow::anyhow!("failed to get current directory: {error}")
                })?;
                path.push(DEFAULT_LOG_DIRECTORY);
                path.to_string_lossy().into_owned()
            },
            (false, _) => String::new(),
        };
        if log_to_file {
            Self::validate_log_directory(&log_directory)?;
        }

        Ok(Self {
            user_vm_id,
            kernel_filename,
            initrd_filename,
            ramfs_filename,
            initrd_args,
            kernel_args,
            vm_stderr,
            log_to_file,
            log_directory,
            snapshot_path,
            #[cfg(feature = "gdb")]
            gdb_port,
        })
    }

    fn validate_log_directory(log_directory: &str) -> Result<()> {
        let path: &Path = Path::new(log_directory);
        ::std::fs::create_dir_all(path).map_err(|error| {
            ::anyhow::anyhow!(
                "failed to create log directory (path={}, error={error})",
                path.display()
            )
        })?;
        let test_file_path: PathBuf = path.join(Self::TEST_LOG_FILENAME);
        ::std::fs::File::create(&test_file_path).map_err(|error| {
            ::anyhow::anyhow!(
                "failed to create log file (path={}, error={error})",
                test_file_path.display()
            )
        })?;
        let _ = ::std::fs::remove_file(test_file_path);
        Ok(())
    }

    /// Prints program usage.
    pub fn usage() {
        eprintln!(
            "Usage: {} [{} <id>] {} <kernel> [{} <file>] [{} <file>] [{} [{} <dir>]] [{} <args>] \
             [{} <args>] [{} <file>] [{} <path>]{}",
            Self::PROGRAM_NAME,
            Self::OPT_USER_VM_ID,
            Self::OPT_KERNEL,
            Self::OPT_INITRD,
            Self::OPT_STDERR,
            Self::OPT_LOGFILE,
            Self::OPT_LOGDIR,
            Self::OPT_INITRD_ARGS,
            Self::OPT_KERNEL_ARGS,
            Self::OPT_RAMFS,
            Self::OPT_SNAPSHOT,
            if cfg!(feature = "gdb") {
                " [-gdb-port <port>]"
            } else {
                ""
            },
        );
    }

    /// Returns the user VM identifier.
    pub fn user_vm_id(&self) -> UserVmIdentifier {
        self.user_vm_id
    }

    /// Takes the optional initrd filename.
    pub fn initrd_filename(&mut self) -> Option<String> {
        self.initrd_filename.take()
    }

    /// Takes the optional RAM filesystem filename.
    pub fn ramfs_filename(&mut self) -> Option<String> {
        self.ramfs_filename.take()
    }

    /// Takes the optional initrd arguments.
    pub fn initrd_args(&mut self) -> Option<String> {
        self.initrd_args.take()
    }

    /// Takes the optional kernel arguments.
    pub fn kernel_args(&mut self) -> Option<String> {
        self.kernel_args.take()
    }

    /// Returns the kernel filename.
    pub fn kernel_filename(&self) -> &str {
        &self.kernel_filename
    }

    /// Takes the optional standard error output filename.
    pub fn take_vm_stderr(&mut self) -> Option<String> {
        self.vm_stderr.take()
    }

    /// Returns whether file logging is enabled.
    pub fn log_to_file(&self) -> bool {
        self.log_to_file
    }

    /// Returns the log directory.
    pub fn log_directory(&self) -> String {
        self.log_directory.clone()
    }

    /// Takes the optional snapshot path.
    pub fn take_snapshot_path(&mut self) -> Option<String> {
        self.snapshot_path.take()
    }

    /// Returns the optional GDB server port.
    #[cfg(feature = "gdb")]
    pub fn gdb_port(&self) -> Option<u16> {
        self.gdb_port
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn base_args() -> Vec<String> {
        vec![
            "uservm".to_string(),
            Args::OPT_KERNEL.to_string(),
            "kernel.elf".to_string(),
        ]
    }

    #[test]
    fn parse_defaults_user_vm_identifier() {
        let args: Args = Args::parse(base_args()).expect("arguments should parse");
        assert_eq!(u32::from(args.user_vm_id()), 0);
        assert_eq!(args.kernel_filename(), "kernel.elf");
    }

    #[test]
    fn parse_accepts_user_vm_identifier_and_images() {
        let mut raw_args: Vec<String> = base_args();
        raw_args.extend([
            Args::OPT_USER_VM_ID.to_string(),
            "7".to_string(),
            Args::OPT_INITRD.to_string(),
            "initrd.img".to_string(),
            Args::OPT_RAMFS.to_string(),
            "ramfs.img".to_string(),
        ]);
        let mut args: Args = Args::parse(raw_args).expect("arguments should parse");
        assert_eq!(u32::from(args.user_vm_id()), 7);
        assert_eq!(args.initrd_filename().as_deref(), Some("initrd.img"));
        assert_eq!(args.ramfs_filename().as_deref(), Some("ramfs.img"));
    }

    #[test]
    fn parse_rejects_missing_kernel() {
        let result: Result<Args> = Args::parse(vec!["uservm".to_string()]);
        assert!(result.is_err());
    }

    #[cfg(feature = "gdb")]
    #[test]
    fn parse_accepts_gdb_port() {
        let mut raw_args: Vec<String> = base_args();
        raw_args.extend([Args::OPT_GDB_PORT.to_string(), "1234".to_string()]);
        let args: Args = Args::parse(raw_args).expect("GDB port should parse");
        assert_eq!(args.gdb_port(), Some(1234));
    }

    #[cfg(feature = "gdb")]
    #[test]
    fn parse_rejects_invalid_gdb_port() {
        let mut raw_args: Vec<String> = base_args();
        raw_args.extend([Args::OPT_GDB_PORT.to_string(), "invalid".to_string()]);
        assert!(Args::parse(raw_args).is_err());
    }
}
