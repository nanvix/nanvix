// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! The `nanvixd-vmm` daemon: boots the Nanvix guest in **standalone** deployment
//! on top of the OpenVMM virtualization stack, reusing the production host-side
//! daemons (`hostfsd`, `networkd`).
//!
//! It is a drop-in for the production `nanvixd` standalone binary and supports
//! the same two operating modes:
//!
//! - **terminal** (interactive): a program is given after `--`; the guest's
//!   stdin/stdout are bridged to the daemon's stdin/stdout and the process exits
//!   with the guest's exit code.
//! - **http**: `-http-addr <host:port>` starts a control server exposing the
//!   `NEW`/`KILL` API plus a per-VM gateway Unix socket (see [`nanvixd_vmm::http`]).
//!
//! The two modes are mutually exclusive.

use ::nanvixd_vmm::{
    build_guest_image,
    http::{
        self,
        HttpConfig,
    },
    init_logging,
    io::{
        GuestIo,
        HostGuestIo,
    },
    open_console,
    stdin::HostStdin,
    vmm,
    DEFAULT_MEM_SIZE,
};
use ::std::{
    path::PathBuf,
    process::ExitCode,
};

/// Environment variable that overrides the host-side log filter.
const LOG_ENV_VAR: &str = "NANVIXD_VMM_LOG";

/// Largest POSIX exit code; out-of-range guest codes clamp to this.
const MAX_EXIT_CODE: i32 = 255;

/// Parsed command-line configuration.
struct Cli {
    /// Directory containing `kernel.elf` and guest binaries.
    bin_dir: PathBuf,
    /// Optional RAM filesystem image.
    ramfs: Option<PathBuf>,
    /// Optional kernel arguments.
    kernel_args: Option<String>,
    /// Optional guest console sink file (default: stderr).
    console_file: Option<PathBuf>,
    /// Optional host directory served to the guest via `hostfsd`.
    mount_directory: Option<PathBuf>,
    /// Whether host networking is served via `networkd`.
    networking: bool,
    /// HTTP socket address; when set, the daemon runs in HTTP mode.
    http_addr: Option<String>,
    /// Program (initrd) to boot in terminal mode.
    program: Option<String>,
    /// Arguments forwarded to the program in terminal mode.
    program_args: Vec<String>,
}

impl Cli {
    /// Parses arguments, accepting (and ignoring where appropriate) the flags
    /// that the production `nanvixd` accepts so this binary is a drop-in.
    fn parse(mut args: impl Iterator<Item = String>) -> ::anyhow::Result<Self> {
        let mut cli = Cli {
            bin_dir: PathBuf::from("./bin"),
            ramfs: None,
            kernel_args: None,
            console_file: None,
            mount_directory: None,
            networking: false,
            http_addr: None,
            program: None,
            program_args: Vec::new(),
        };

        // Skip the program name.
        let _ = args.next();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-bin-dir" => cli.bin_dir = PathBuf::from(require_value(&mut args, "-bin-dir")?),
                "-ramfs" => cli.ramfs = Some(PathBuf::from(require_value(&mut args, "-ramfs")?)),
                "-kernel-args" => cli.kernel_args = Some(require_value(&mut args, "-kernel-args")?),
                "-console-file" => {
                    cli.console_file =
                        Some(PathBuf::from(require_value(&mut args, "-console-file")?))
                },
                "-mount" => {
                    cli.mount_directory = Some(PathBuf::from(require_value(&mut args, "-mount")?))
                },
                "-allow-host-networking" => cli.networking = true,
                "-http-addr" => cli.http_addr = Some(require_value(&mut args, "-http-addr")?),
                // Flags accepted from the production `nanvixd` for drop-in
                // compatibility but not meaningful for a single-vCPU standalone
                // OpenVMM guest. They take a value, which we discard.
                "-clh-bin-path" | "-hwloc" | "-log-dir" | "-netns-pool-size" => {
                    let _ = require_value(&mut args, &arg)?;
                },
                // L2 deployment is not supported by this standalone VMM.
                "-l2" => anyhow::bail!("-l2 (L2 deployment) is not supported by nanvixd-vmm"),
                // Everything after `--` is the program and its arguments.
                "--" => {
                    if let Some(program) = args.next() {
                        cli.program = Some(program);
                        cli.program_args = args.by_ref().collect();
                    }
                    break;
                },
                other => anyhow::bail!("unknown argument: {other}"),
            }
        }

        if cli.http_addr.is_some() && cli.program.is_some() {
            anyhow::bail!(
                "http mode (-http-addr) and interactive mode (-- program) are mutually exclusive"
            );
        }
        if cli.http_addr.is_none() && cli.program.is_none() {
            anyhow::bail!(
                "no mode selected: pass -http-addr <host:port> for HTTP mode or `-- <program> \
                 [args...]` for terminal mode"
            );
        }

        Ok(cli)
    }
}

/// Returns the value following an option, or an error if it is missing.
fn require_value(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> ::anyhow::Result<String> {
    args.next()
        .ok_or_else(|| anyhow::anyhow!("missing value for {option}"))
}

fn main() -> ExitCode {
    init_logging(LOG_ENV_VAR);

    let cli: Cli = match Cli::parse(::std::env::args()) {
        Ok(cli) => cli,
        Err(e) => {
            ::log::error!("{e}");
            return ExitCode::FAILURE;
        },
    };

    let result: ::anyhow::Result<ExitCode> = if let Some(http_addr) = cli.http_addr.clone() {
        run_http(&cli, &http_addr)
    } else {
        run_terminal(cli)
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            ::log::error!("nanvixd-vmm failed: {e:?}");
            ExitCode::FAILURE
        },
    }
}

/// Runs the daemon in terminal (interactive) mode and returns the guest's code.
fn run_terminal(cli: Cli) -> ::anyhow::Result<ExitCode> {
    let program: String = cli.program.expect("terminal mode requires a program");
    let initrd_args: Option<String> = if cli.program_args.is_empty() {
        None
    } else {
        Some(cli.program_args.join(" "))
    };

    let image = build_guest_image(
        &cli.bin_dir,
        Some(PathBuf::from(program)),
        initrd_args,
        cli.kernel_args,
        cli.ramfs,
        DEFAULT_MEM_SIZE,
    );
    let console = open_console(cli.console_file.as_deref())?;
    let io: Box<dyn GuestIo> = Box::new(HostGuestIo::new(HostStdin::spawn()));
    let mount_directory: Option<PathBuf> = cli.mount_directory;
    let networking: bool = cli.networking;

    let exit_code: u16 = ::pal_async::DefaultPool::run_with(move |driver| async move {
        vmm::run(driver, image, io, console, mount_directory, networking).await
    })?;

    Ok(ExitCode::from(clamp_exit_code(i32::from(exit_code))))
}

/// Runs the daemon in HTTP mode until a shutdown signal.
fn run_http(cli: &Cli, http_addr: &str) -> ::anyhow::Result<ExitCode> {
    let config = HttpConfig {
        bin_dir: cli.bin_dir.clone(),
        ramfs: cli.ramfs.clone(),
        kernel_args: cli.kernel_args.clone(),
        console_file: cli.console_file.clone(),
        mount_directory: cli.mount_directory.clone(),
        networking: cli.networking,
        mem_size: DEFAULT_MEM_SIZE,
    };

    let runtime = ::tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(http::serve(http_addr, config))?;
    Ok(ExitCode::SUCCESS)
}

/// Clamps an exit code to the valid POSIX range `[0, 255]`.
fn clamp_exit_code(exit_code: i32) -> u8 {
    if (0..=MAX_EXIT_CODE).contains(&exit_code) {
        exit_code as u8
    } else {
        MAX_EXIT_CODE as u8
    }
}
