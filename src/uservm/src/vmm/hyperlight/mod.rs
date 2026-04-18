// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

pub mod guest;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    orchestrator::VcpuControlResponse,
    vmm::{
        MicroVmArgs,
        guest::Guest,
    },
};
use ::anyhow::Result;
use ::core::convert::TryFrom;
use ::hyperlight_host::{
    GuestBinary,
    GuestCounter,
    HyperlightError,
    MultiUseSandbox,
    UninitializedSandbox,
    mem::memory_region::MemoryRegionFlags,
    sandbox::{
        SandboxConfiguration,
        uninitialized::{
            GuestBlob,
            GuestEnvironment,
        },
    },
};
#[cfg(target_os = "windows")]
use ::log::warn;
use ::log::{
    debug,
    error,
};
#[cfg(target_os = "linux")]
use ::std::{
    fs::File,
    os::{
        raw::c_int,
        unix::io::AsRawFd,
    },
};
use ::std::{
    io::Write,
    path::Path,
    sync::Arc,
    time::Duration,
};
use ::sys::error::ErrorCode;
use ::tokio::{
    runtime::Handle,
    sync::{
        Mutex,
        mpsc::Sender,
    },
    task,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Signal used to interrupt the vCPU thread.
#[cfg(target_os = "linux")]
pub const INTERRUPT_SIGNAL: c_int = libc::SIGUSR1;

/// Signal used to kill the vCPU thread.
#[cfg(target_os = "linux")]
pub const KILL_SIGNAL: c_int = libc::SIGKILL;

/// Grace period before sending SIGKILL to the vCPU thread during shutdown.
/// This allows the kernel's `abort_with_code()` to complete before the thread is killed.
/// See issue #1010 for more context on this workaround.
pub const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_millis(100);

/// Label used for the RAMFS file mapping in the PEB.
const RAMFS_LABEL: &str = "ramfs";

//==================================================================================================
// Types
//==================================================================================================

pub type StdinFn = dyn FnMut() -> Result<Vec<u8>, HyperlightError> + Send;

pub type StdoutFn = dyn FnMut(Vec<u8>) -> Result<i32, HyperlightError> + Send;

/// Output function for data chunk transfers (VmbusBulkWrite host function). The kernel sends only the
/// DataChunkHeader, and this function reads the actual data from guest shared memory at the GPA
/// stored in the header.
pub type BulkStdoutFn = dyn FnMut(Vec<u8>) -> Result<i32, HyperlightError> + Send;

/// Input function for chunked bulk reads (VmbusBulkRead host function). Each call returns
/// the next chunk of pending bulk data (up to MAX_CHUNK bytes), or an empty Vec when done.
pub type BulkStdinFn = dyn FnMut() -> Result<Vec<u8>, HyperlightError> + Send;

pub type StderrFn = dyn Write + Send;

//==================================================================================================
// StderrRedirect
//==================================================================================================

/// RAII guard that redirects process stderr to a file and restores the original fd on drop.
#[cfg(target_os = "linux")]
struct StderrRedirect {
    saved_fd: c_int,
}

#[cfg(target_os = "linux")]
impl StderrRedirect {
    /// Redirects process stderr to `path`, returning a guard that restores it on drop.
    fn new(path: &str) -> Result<Self> {
        // SAFETY: STDERR_FILENO is always valid. `dup` returns a new fd or -1 on failure.
        let saved_fd: c_int = unsafe { libc::dup(libc::STDERR_FILENO) };
        if saved_fd == -1 {
            return Err(anyhow::anyhow!(
                "failed to save stderr fd: {}",
                std::io::Error::last_os_error()
            ));
        }
        let file: File = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .map_err(|e| {
                // SAFETY: saved_fd was just obtained from dup() and is valid.
                unsafe { libc::close(saved_fd) };
                anyhow::anyhow!("failed to open stderr file {path:?}: {e}")
            })?;
        // SAFETY: both fds are valid — file was just opened and STDERR_FILENO is always
        // present. On success dup2 returns the new fd; on failure it returns -1.
        if unsafe { libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO) } == -1 {
            let err: std::io::Error = std::io::Error::last_os_error();
            // SAFETY: saved_fd was obtained from dup() and is valid.
            unsafe { libc::close(saved_fd) };
            return Err(anyhow::anyhow!("failed to redirect stderr to {path:?}: {err}"));
        }
        // After dup2, STDERR_FILENO holds a copy of the fd. The original File is dropped here.
        Ok(Self { saved_fd })
    }
}

#[cfg(target_os = "linux")]
impl Drop for StderrRedirect {
    fn drop(&mut self) {
        // SAFETY: saved_fd was obtained from dup() in new() and has not been closed.
        unsafe {
            libc::dup2(self.saved_fd, libc::STDERR_FILENO);
            libc::close(self.saved_fd);
        }
    }
}

/// No-op stderr redirect on Windows — hyperlight DebugPrint output goes to default stderr.
#[cfg(target_os = "windows")]
struct StderrRedirect;

#[cfg(target_os = "windows")]
impl StderrRedirect {
    fn new(path: &str) -> Result<Self> {
        if !path.is_empty() {
            warn!(
                "stderr redirection to '{}' is not supported on Windows; output goes to default \
                 stderr",
                path
            );
        }
        Ok(Self)
    }
}

//==================================================================================================
// Structure
//==================================================================================================

pub struct VirtualMemory {
    counter: GuestCounter,
}

//==================================================================================================
// VirtualMemory Implementations
//==================================================================================================

impl VirtualMemory {
    /// Writes a sequence of bytes into guest memory at the given address.
    ///
    /// TODO (#1731): implement using upstream hyperlight host shared-memory API.
    pub fn write_bytes(&mut self, _addr: u64, _data: &[u8]) -> ::anyhow::Result<()> {
        error!("write_bytes(): not implemented for hyperlight VMM");
        Err(anyhow::anyhow!("write_bytes not implemented for hyperlight VMM"))
    }

    /// Reads a sequence of bytes from guest memory at the given address.
    ///
    /// TODO (#1731): implement using upstream hyperlight host shared-memory API.
    pub fn read_bytes(&mut self, _addr: u64, _data: &mut [u8]) -> ::anyhow::Result<()> {
        error!("read_bytes(): not implemented for hyperlight VMM");
        Err(anyhow::anyhow!("read_bytes not implemented for hyperlight VMM"))
    }
}

#[derive(Clone)]
pub struct Vmm {
    guest: Arc<Mutex<Guest>>,
    inner: Arc<Mutex<InnerVmm>>,
    // Wrapped in Option so we can move the UninitializedSandbox out (evolve consumes self).
    sandbox: Arc<Mutex<Option<UninitializedSandbox>>>,
    vmem: Arc<Mutex<VirtualMemory>>,
}

struct InnerVmm {
    control_tx: Sender<VcpuControlResponse>,
    /// RAII guard that restores the original stderr fd when dropped.
    _stderr_redirect: Option<StderrRedirect>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Vmm {
    pub fn new(args: MicroVmArgs) -> Result<Self> {
        let guest: Guest = Guest;

        let memory_size: usize = ::config::kernel::MEMORY_SIZE;

        let guest_env: GuestEnvironment = if let Some(initrd_filename) = &args.initrd_filename {
            match std::fs::read(initrd_filename) {
                Ok(bytes) => {
                    let initrd_size: usize = bytes.len();
                    debug!("initrd: {} bytes", initrd_size);

                    // Detect whether this is a multibinary NVMB image or a single ELF.
                    let is_multibinary: bool = initrd_size >= ::multibin::MAGIC.len()
                        && bytes[..::multibin::MAGIC.len()] == ::multibin::MAGIC;

                    // Build the init_data blob based on the initrd format.
                    let init_data_bytes: Vec<u8> = if is_multibinary {
                        // Multibinary: pass raw NVMB image, no wrapping needed.
                        debug!("initrd: multibinary format detected, passing raw image");
                        bytes
                    } else {
                        // Single ELF: prepend size header and append args (old format).
                        let initrd_args_bytes: Vec<u8> =
                            Self::build_args_bytes(initrd_filename, &args.initrd_args)?;

                        let mut padded: Vec<u8> = Vec::with_capacity(
                            ::config::hyperlight::INITRD_SIZE_BYTES
                                + initrd_size
                                + initrd_args_bytes.len(),
                        );

                        // Write the actual size as first INITRD_SIZE_BYTES (little-endian).
                        padded.extend_from_slice(&(initrd_size as u64).to_le_bytes());
                        padded.extend_from_slice(&bytes);
                        padded.extend_from_slice(&initrd_args_bytes);

                        debug!(
                            "initrd blob: {} bytes total ({} byte header + {} bytes data + {} \
                             bytes args)",
                            padded.len(),
                            ::config::hyperlight::INITRD_SIZE_BYTES,
                            initrd_size,
                            initrd_args_bytes.len(),
                        );

                        padded
                    };

                    // Box the data to extend its lifetime.
                    let boxed_data: Box<[u8]> = init_data_bytes.into_boxed_slice();
                    let data_ref: &'static [u8] = Box::leak(boxed_data);

                    GuestEnvironment {
                        guest_binary: GuestBinary::FilePath(args.kernel_filename.to_string()),
                        init_data: Some(GuestBlob {
                            data: data_ref,
                            permissions: MemoryRegionFlags::READ
                                | MemoryRegionFlags::WRITE
                                | MemoryRegionFlags::EXECUTE,
                        }),
                    }
                },
                Err(err) => {
                    let reason: String = format!("failed to read initrd file {err:?}");
                    error!("initrd(): {reason} (args={args:?})");
                    return Err(anyhow::anyhow!("{reason}"));
                },
            }
        } else {
            GuestEnvironment::new(GuestBinary::FilePath(args.kernel_filename.to_string()), None)
        };

        // The hyperlight heap covers everything from after the I/O buffers to the end of the
        // usable guest physical memory. With `executable_heap` enabled, it is RWX so the kernel
        // can execute code from the heap (used for process loading).
        //
        // The heap must be large enough to cover:
        //   - Padding from structures_end to KPOOL_BASE
        //   - KPOOL_SIZE (the kernel pool)
        //   - Any remaining memory up to memory_size
        //
        // We compute heap_size as: memory_size - HYPERLIGHT_BASE_ADDRESS - overhead
        // where overhead accounts for the kernel code, PEB, and I/O buffers that precede the heap.
        // Since we don't know the exact kernel code size here (hyperlight computes it internally),
        // we use memory_size as an upper bound. Hyperlight will allocate only what's needed.
        let mut config: SandboxConfiguration = SandboxConfiguration::default();

        // Set heap large enough to cover from after kernel structures through end of memory.
        // The actual heap start depends on the kernel loaded size, PEB, and I/O buffers.
        // We request enough heap so that the heap region extends well beyond KPOOL_BASE + KPOOL_SIZE.
        let heap_size: usize = memory_size;
        config.set_heap_size(heap_size as u64);

        // Creates Hyperlight sandbox.
        let mut sandbox: UninitializedSandbox = UninitializedSandbox::new(guest_env, Some(config))
            .map_err(|e| {
                error!("failed to create UninitializedSandbox: {e:?}");
                anyhow::anyhow!("{e:?}")
            })?;

        // Map RAMFS file into sandbox memory if provided.
        // The file is mapped copy-on-write at the first GPA after the sandbox's
        // shared memory slot. With nanvix-unstable, BASE_ADDRESS is 0x0 so the
        // shared memory occupies GPA [0, shared_mem_size). The file mapping
        // metadata is automatically written to the PEB during evolve(), so the
        // guest kernel can discover and identity-map the RAMFS region during boot.
        if let Some(ramfs_filename) = &args.ramfs_filename {
            let ramfs_path: &Path = Path::new(ramfs_filename);
            let ramfs_gpa: u64 = sandbox.shared_mem_size() as u64;
            let ramfs_size: u64 = sandbox
                .map_file_cow(ramfs_path, ramfs_gpa, Some(RAMFS_LABEL))
                .map_err(|e| {
                    error!("failed to map ramfs file: {e:?}");
                    anyhow::anyhow!("failed to map ramfs file: {e:?}")
                })?;
            debug!(
                "ramfs: mapped {:?} ({} bytes) at GPA {:#010x}",
                ramfs_path, ramfs_size, ramfs_gpa
            );
        }

        // Create a guest counter backed by a fixed offset in scratch memory.
        // The counter holds its own Arc clones of the mapping handle and RwLock,
        // so it remains valid across evolve() and for the sandbox's entire lifetime.
        let counter: GuestCounter = sandbox.guest_counter().map_err(|e| {
            error!("failed to get guest counter: {e:?}");
            anyhow::anyhow!("{e:?}")
        })?;

        let vmem: Arc<Mutex<VirtualMemory>> = Arc::new(Mutex::new(VirtualMemory { counter }));

        let guest: Arc<Mutex<Guest>> = Arc::new(Mutex::new(guest));

        // Redirect process stderr to the custom file when configured, so that DebugPrint VM-exit
        // output (sent via `eprint!` in the hyperlight SDK) reaches the intended destination.
        // The guard restores the original stderr when the Vmm is dropped.
        let stderr_redirect: Option<StderrRedirect> = args
            .stderr_path
            .as_deref()
            .map(StderrRedirect::new)
            .transpose()?;

        // Create a closure for VmbusWrite that matches the expected signature
        // NOTE: output function is FnMut, so we must keep it mutable when captured.
        let mut output_fn: Box<StdoutFn> = args.output;
        sandbox.register("VmbusWrite", move |data: Vec<u8>| -> i32 {
            output_fn(data).unwrap_or(-1)
        })?;

        // Create a closure for VmbusBulkWrite that handles data chunk transfers.
        // NOTE: bulk output function is FnMut, so we must keep it mutable when captured.
        let mut bulk_output_fn: Box<BulkStdoutFn> = args.bulk_output;
        sandbox.register("VmbusBulkWrite", move |data: Vec<u8>| -> i32 {
            bulk_output_fn(data).unwrap_or(-1)
        })?;

        // Create a closure for VmbusBulkRead that returns chunks of pending bulk data.
        let mut bulk_input_fn: Box<BulkStdinFn> = args.bulk_input;
        sandbox.register("VmbusBulkRead", move || -> Vec<u8> {
            match bulk_input_fn() {
                Ok(data) => data,
                Err(e) => {
                    error!("VmbusBulkRead: {e:?}");
                    Vec::new()
                },
            }
        })?;

        // Create a closure for VmbusRead that matches the expected signature
        // NOTE: input function is FnMut, so we must keep it mutable when captured.
        let mut input_fn: Box<StdinFn> = args.input;
        sandbox.register("VmbusRead", move || -> Vec<u8> {
            match input_fn() {
                Ok(data) => data,
                Err(e) => {
                    error!("VmbusRead: {e:?}");
                    Vec::new()
                },
            }
        })?;

        Ok(Self {
            vmem,
            sandbox: Arc::new(Mutex::new(Some(sandbox))),
            guest,
            inner: Arc::new(Mutex::new(InnerVmm {
                control_tx: args.control_tx,
                _stderr_redirect: stderr_redirect,
            })),
        })
    }

    pub fn spawn(mut self) -> tokio::task::JoinHandle<Result<u16>> {
        task::spawn_blocking(move || {
            #[cfg(target_os = "linux")]
            let thread_id: u64 = unsafe { libc::pthread_self() } as u64;
            #[cfg(target_os = "windows")]
            let thread_id: u64 =
                unsafe { windows::Win32::System::Threading::GetCurrentThreadId() as u64 };
            Handle::current().block_on(self.send_tid(thread_id))?;
            self.run()
        })
    }

    ///
    /// # Description
    ///
    /// This function instantiates and runs the virtual machine monitor (VMM) with the given arguments.
    ///
    /// # Parameters
    ///
    /// - `memory_size`: The memory size for the virtual machine in bytes.
    /// - `kernel_filename`: The path to the kernel file to be loaded into the virtual machine.
    /// - `initrd_filename`: An optional path to the initial RAM disk (initrd) file.
    /// - `initrd_args`: Optional arguments to be passed to the initrd.
    /// - `stderr`: An optional path to a file where the virtual machine's standard error output will be written.
    /// - `system_vm_stream`: An optional connection to the system VM for communication with the virtual machine.
    /// - `control_plane_stream`: An optional connection to the nanvixd control-plane.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the exit status of the virtual machine.
    /// Otherwise, it returns an error.
    ///
    pub fn run(&mut self) -> Result<u16> {
        let uninit: UninitializedSandbox = {
            let mut guard: ::tokio::sync::MutexGuard<'_, Option<UninitializedSandbox>> =
                self.sandbox.blocking_lock();
            guard
                .take()
                .ok_or_else(|| anyhow::anyhow!("sandbox already evolved"))?
        };

        // Phase 1: Evolve — boot the kernel through early init. The kernel halts from
        // hyperlight_pre_kmain() during the pre-kmain evolve phase via VmAction::Halt with the
        // _nanvix_dispatch address in EAX. This causes evolve() to return Ok(MultiUseSandbox).
        let mut sandbox: MultiUseSandbox = match uninit.evolve() {
            Ok(s) => s,
            Err(error) => {
                // Drop the stderr redirect guard to restore the original stderr fd.
                self.inner.blocking_lock()._stderr_redirect.take();

                // Communicate shutdown to orchestrator.
                if let Err(e) = self
                    .inner
                    .blocking_lock()
                    .control_tx
                    .blocking_send(VcpuControlResponse::Shutdown)
                {
                    error!("run(): failed to notify vmm thread (error={e:?})");
                }

                // During evolve(), GuestAborted is wrapped in HyperlightVmError(Initialize(...)) so
                // extract it from the error chain.
                return match Self::extract_guest_abort(&error) {
                    Some((code, message)) => {
                        if message.is_empty() {
                            debug!("run(): guest exited during evolve (code={code})");
                        } else {
                            debug!(
                                "run(): guest exited during evolve (code={code}, \
                                 message={message})"
                            );
                        }
                        Ok(code as u16)
                    },
                    None => {
                        error!("run(): vmm aborted during evolve (debug={error:?})");
                        Ok(ErrorCode::ConnectionReset.into())
                    },
                };
            },
        };

        debug!("run(): evolve completed, calling sandbox.call(\"kmain\")");

        // Phase 2: Call — re-enter the guest at _nanvix_dispatch to actually run the system.
        let call_result: Result<(), HyperlightError> = sandbox.call("kmain", ());

        // Drop the stderr redirect guard to restore the original stderr fd.
        self.inner.blocking_lock()._stderr_redirect.take();

        // Communicate shutdown to orchestrator.
        if let Err(error) = self
            .inner
            .blocking_lock()
            .control_tx
            .blocking_send(VcpuControlResponse::Shutdown)
        {
            error!("run(): failed to notify vmm thread (error={error:?})");
            // Don't bail as we are shutting down anyway.
        }

        // Parse result.
        // The dispatch handler halts via VmAction::Halt, which produces VmExit::Halt on the host.
        // Hyperlight then tries to read a guest function return value from the PEB output buffer,
        // which may fail with "Stack pointer is out of bounds" because the dispatch handler doesn't
        // use Hyperlight's guest function return convention. We treat this specific error as
        // success.
        match call_result {
            Ok(()) => {
                debug!("run(): guest call completed normally");
                Ok(0)
            },
            Err(ref error) if Self::is_halt_exit(error) => {
                debug!("run(): guest halted cleanly (Halt path, no return value in PEB)");
                Ok(0)
            },
            Err(error) => match Self::extract_guest_abort(&error) {
                Some((code, message)) => {
                    if message.is_empty() {
                        debug!("run(): guest exited (code={code})");
                    } else {
                        debug!("run(): guest exited (code={code}, message={message})");
                    }
                    Ok(code as u16)
                },
                None => {
                    error!("run(): vmm aborted (debug={error:?}, display={error})");
                    Ok(ErrorCode::ConnectionReset.into())
                },
            },
        }
    }

    ///
    /// # Description
    ///
    /// Checks whether the error from `sandbox.call()` represents a clean VM halt.
    ///
    /// When the kernel exits via `VmAction::Halt` during a guest call, Hyperlight's dispatch loop
    /// returns `Ok(())` but then tries to read a guest function return value from the PEB output
    /// buffer. Since the kernel doesn't use Hyperlight's guest function return convention, this
    /// read fails with "Stack pointer is out of bounds" (an `AnyhowError` from `shared_mem.rs`).
    ///
    /// We detect this by first narrowing to the expected `AnyhowError` variant and then checking
    /// the inner error's `Display` representation. This avoids treating unrelated
    /// `HyperlightError` variants as clean halts just because their top-level formatted message
    /// contains the same substring.
    ///
    fn is_halt_exit(error: &HyperlightError) -> bool {
        match error {
            HyperlightError::AnyhowError(inner) => {
                inner.to_string().contains("Stack pointer is out of bounds")
            },
            _ => false,
        }
    }

    /// Extracts the guest exit code from a nested `GuestAborted` error.
    ///
    /// The kernel always exits via `abort_with_code()`, which during `evolve()` is wrapped as:
    /// `HyperlightVmError(Initialize(Run(HandleIo(Outb(GuestAborted { code, message })))))`.
    /// Since the nested error types are `pub(crate)` in hyperlight, we match the top-level
    /// `GuestAborted` variant directly and fall back to parsing the `Debug` representation.
    fn extract_guest_abort(error: &HyperlightError) -> Option<(u8, String)> {
        // Direct match for top-level GuestAborted (used by DispatchGuestCall path).
        if let HyperlightError::GuestAborted(code, message) = error {
            return Some((*code, message.clone()));
        }

        // For Initialize path, parse the Debug representation.
        // Format: ...GuestAborted { code: N, message: "..." }...
        let debug_str = format!("{error:?}");
        if let Some(result) = Self::parse_guest_aborted_from_debug(&debug_str) {
            return Some(result);
        }

        // Fallback: parse the Display representation.
        // Format: ...Guest aborted: error code N, message: ...
        let display_str = format!("{error}");
        Self::parse_guest_aborted_from_display(&display_str)
    }

    /// Parses a `GuestAborted { code: N, message: "..." }` fragment from a `Debug` string.
    fn parse_guest_aborted_from_debug(debug_str: &str) -> Option<(u8, String)> {
        const CODE_PREFIX: &str = "code: ";
        const MESSAGE_PREFIX: &str = "message: \"";

        let rest = &debug_str[debug_str.find("GuestAborted")?..];
        let code_str = &rest[rest.find(CODE_PREFIX)? + CODE_PREFIX.len()..];
        let code_end = code_str.find([',', ' ', '}'])?;
        let code = code_str[..code_end].parse::<u8>().ok()?;

        let message = rest
            .find(MESSAGE_PREFIX)
            .map(|pos| {
                let msg_str = &rest[pos + MESSAGE_PREFIX.len()..];
                msg_str
                    .find('"')
                    .map(|end| msg_str[..end].to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        Some((code, message))
    }

    /// Parses a `Guest aborted: error code N, message: M` fragment from a `Display` string.
    fn parse_guest_aborted_from_display(display_str: &str) -> Option<(u8, String)> {
        const PREFIX: &str = "Guest aborted: error code ";

        let rest = &display_str[display_str.find(PREFIX)? + PREFIX.len()..];
        let code_end = rest.find(',')?;
        let code = rest[..code_end].trim().parse::<u8>().ok()?;

        let message = rest
            .find("message: ")
            .map(|pos| rest[pos + "message: ".len()..].trim().to_string())
            .unwrap_or_default();

        Some((code, message))
    }

    ///
    /// # Description
    ///
    /// Sends the vCPU thread's tid to the main thread.
    ///
    /// # Parameters
    ///
    /// - `tid`: The vCPU thread's tid.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    async fn send_tid(&self, tid: u64) -> Result<()> {
        Ok(self
            .inner
            .lock()
            .await
            .control_tx
            .send(VcpuControlResponse::Tid(tid))
            .await?)
    }

    pub fn guest(&self) -> Arc<Mutex<Guest>> {
        self.guest.clone()
    }

    pub fn vmem(&self) -> Arc<Mutex<VirtualMemory>> {
        self.vmem.clone()
    }

    pub async fn load_snapshot(&self, filepath: String) -> Result<()> {
        let reason: String = format!("load_snapshot(): not implemented for filepath={}", filepath);
        error!("{}", reason);
        Err(anyhow::anyhow!(reason))
    }

    pub async fn create_snapshot(&self, filepath: String) -> Result<()> {
        let reason: String =
            format!("create_snapshot(): not implemented for filepath={}", filepath);
        error!("{}", reason);
        Err(anyhow::anyhow!(reason))
    }

    /// No-op on the Hyperlight backend. Shutdown is handled via cooperative guest exit.
    pub fn request_shutdown(&self) {}

    ///
    /// # Description
    ///
    /// Encodes the program name and arguments into a byte vector suitable for passing to the guest.
    /// The first byte of the vector indicates the length of the arguments, followed by the program
    /// name and arguments as a null-terminated string.
    ///
    /// # Parameters
    ///
    /// - `program_name`: The name of the program to be executed.
    /// - `program_args`: An optional string containing the arguments to be passed to the program.
    ///
    /// # Returns
    ///
    /// On success, this function returns a vector of bytes representing the encoded arguments.
    /// On failure, it returns an error.
    ///
    fn build_args_bytes(program_name: &String, program_args: &Option<String>) -> Result<Vec<u8>> {
        // Extract filename.
        let mut args_string: String = Path::new(program_name)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| program_name.to_string());

        // Push arguments.
        if let Some(program_args) = program_args
            && !program_args.is_empty()
        {
            args_string.push(' ');
            args_string.push_str(program_args);
        }

        // Encode length-prefixed arguments.
        let args_bytes: Vec<u8> = args_string.into_bytes();
        let args_len: u8 = match u8::try_from(args_bytes.len()) {
            Ok(value) => value,
            Err(_) => {
                let reason: String =
                    format!("initrd arguments too long (len={})", args_bytes.len());
                error!("build_args_bytes(): {}", reason);
                return Err(anyhow::anyhow!(reason));
            },
        };

        Ok([&[args_len], &args_bytes[..]].concat())
    }
}

#[cfg(test)]
mod tests {
    use super::Vmm;

    #[test]
    fn parse_guest_aborted_typical() {
        let input = r#"HyperlightVmError(Initialize(Run(HandleIo(Outb(GuestAborted { code: 42, message: "kernel panic" })))))"#;
        let result = Vmm::parse_guest_aborted_from_debug(input);
        assert_eq!(result, Some((42, "kernel panic".to_string())));
    }

    #[test]
    fn parse_guest_aborted_empty_message() {
        let input = r#"GuestAborted { code: 1, message: "" }"#;
        let result = Vmm::parse_guest_aborted_from_debug(input);
        assert_eq!(result, Some((1, String::new())));
    }

    #[test]
    fn parse_guest_aborted_no_message_field() {
        let input = "GuestAborted { code: 7 }";
        let result = Vmm::parse_guest_aborted_from_debug(input);
        assert_eq!(result, Some((7, String::new())));
    }

    #[test]
    fn parse_guest_aborted_missing() {
        let input = "SomeOtherError { reason: \"something\" }";
        let result = Vmm::parse_guest_aborted_from_debug(input);
        assert_eq!(result, None);
    }

    #[test]
    fn parse_guest_aborted_invalid_code() {
        let input = r#"GuestAborted { code: 999, message: "overflow" }"#;
        let result = Vmm::parse_guest_aborted_from_debug(input);
        assert_eq!(result, None); // 999 doesn't fit u8
    }

    #[test]
    fn parse_guest_aborted_from_display_typical() {
        let input = "Guest aborted: error code 13, message: kernel exited";
        let result = Vmm::parse_guest_aborted_from_display(input);
        assert_eq!(result, Some((13, "kernel exited".to_string())));
    }

    #[test]
    fn parse_guest_aborted_from_display_empty_message() {
        let input = "Guest aborted: error code 1, message: ";
        let result = Vmm::parse_guest_aborted_from_display(input);
        assert_eq!(result, Some((1, String::new())));
    }

    #[test]
    fn parse_guest_aborted_from_display_nested() {
        let input = "initialize sandbox: Guest aborted: error code 42, message: test panic";
        let result = Vmm::parse_guest_aborted_from_display(input);
        assert_eq!(result, Some((42, "test panic".to_string())));
    }

    #[test]
    fn parse_guest_aborted_from_display_missing() {
        let input = "some unrelated error occurred";
        let result = Vmm::parse_guest_aborted_from_display(input);
        assert_eq!(result, None);
    }

    #[test]
    fn parse_guest_aborted_from_display_invalid_code() {
        let input = "Guest aborted: error code 999, message: overflow";
        let result = Vmm::parse_guest_aborted_from_display(input);
        assert_eq!(result, None); // 999 doesn't fit u8
    }
}
