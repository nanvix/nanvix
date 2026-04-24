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
/// This allows the kernel's `VmAction::Halt` to propagate before the thread is killed.
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

        // Get RAMFS file size (if present) for memory budget calculation.
        let ramfs_file_size: usize = args
            .ramfs_filename
            .as_deref()
            .map(Self::get_ramfs_size)
            .transpose()?
            .unwrap_or(0);

        // Get kernel size for memory layout calculation.
        let kernel_size: usize = Self::get_kernel_size(&args.kernel_filename).map_err(|e| {
            error!("new(): failed to determine kernel size: {e:?}");
            anyhow::anyhow!("failed to determine kernel size: {e:?}")
        })?;

        // Build the guest environment.
        let (guest_env, init_data_size): (GuestEnvironment, usize) =
            Self::build_guest_env(&args.kernel_filename, &args.initrd_filename, &args.initrd_args)?;

        // Compute memory layout parameters.
        let kernel_end: usize = ::config::memory_layout::KERNEL_BASE_RAW + kernel_size;
        let kpool_start: usize = ::config::kernel::KPOOL_BASE_RAW;
        let kpool_end: usize = kpool_start + ::config::kernel::KPOOL_SIZE;

        let guest_heap_size: usize = Self::calculate_guest_heap_size(kernel_end, kpool_start)?;

        // Snapshot budget equation:  MEMORY_SIZE = snapshot_budget_size + ramfs + scratch
        // With nanvix-unstable, Hyperlight skips guest page-table generation in
        // Snapshot::from_env(), so snapshot_budget_size equals Hyperlight's get_memory_size()
        // exactly — there is no PT overhead.
        let snapshot_budget_size: usize = {
            let unaligned: usize = kpool_end.checked_add(init_data_size).ok_or_else(|| {
                error!("new(): kpool_end + init_data_size overflow");
                anyhow::anyhow!("kpool_end + init_data_size overflow")
            })?;
            ::sys::mm::align_up(unaligned, ::sys::mm::Alignment::Align4096).ok_or_else(|| {
                error!("new(): snapshot budget alignment overflow");
                anyhow::anyhow!("snapshot budget alignment overflow")
            })?
        };

        // Scratch region: whatever remains after snapshot and ramfs.  This includes the input and
        // output buffers for VmbusRead/VmbusWrite, the guest counter page, and the last page
        // that Hyperlight reserves for bookkeeping (exception stack, allocator state).
        let scratch_size: usize = memory_size
            .checked_sub(
                snapshot_budget_size
                    .checked_add(ramfs_file_size)
                    .ok_or_else(|| {
                        error!("new(): snapshot_budget + ramfs_file_size overflow");
                        anyhow::anyhow!("snapshot_budget + ramfs_file_size overflow")
                    })?,
            )
            .ok_or_else(|| {
                error!(
                    "new(): memory_size ({memory_size:#x}) too small for snapshot_budget \
                     ({snapshot_budget_size:#x}) + ramfs ({ramfs_file_size:#x})"
                );
                anyhow::anyhow!("memory_size too small for snapshot + ramfs")
            })?;

        debug!(
            "memory budget: memory_size={:#x}, snapshot_budget={:#x}, ramfs={:#x}, scratch={:#x}, \
             guest_heap={:#x}",
            memory_size, snapshot_budget_size, ramfs_file_size, scratch_size, guest_heap_size
        );

        let mut config: SandboxConfiguration = SandboxConfiguration::default();
        config.set_heap_size(guest_heap_size as u64);
        config.set_scratch_size(scratch_size);

        // Creates Hyperlight sandbox.
        let mut sandbox: UninitializedSandbox = UninitializedSandbox::new(guest_env, Some(config))
            .map_err(|e| {
                error!("failed to create UninitializedSandbox: {e:?}");
                anyhow::anyhow!("{e:?}")
            })?;

        // With nanvix-unstable, Hyperlight does not append page tables to the snapshot (the
        // #[cfg(not(feature = "nanvix-unstable"))] block in Snapshot::from_env() is compiled out),
        // so shared_mem_size() == snapshot_budget_size and pt_overhead is 0.  The field is kept in
        // GetMemoryLayout for forward-compatibility.
        let actual_snapshot: usize = sandbox.shared_mem_size();
        let pt_overhead: usize = actual_snapshot.saturating_sub(snapshot_budget_size);
        debug!(
            "actual layout: snapshot={:#x} (budget={:#x}, pt_overhead={:#x}), ramfs={:#x}, \
             scratch={:#x}",
            actual_snapshot, snapshot_budget_size, pt_overhead, ramfs_file_size, scratch_size
        );

        // Map RAMFS file into sandbox memory if provided.
        // The file is mapped copy-on-write at the first GPA after the sandbox's
        // shared memory slot. With nanvix-unstable, BASE_ADDRESS is 0x0 so the
        // shared memory occupies GPA [0, shared_mem_size).
        let mut layout_ramfs_base: u32 = 0;
        let mut layout_ramfs_size: u32 = 0;
        if let Some(ramfs_filename) = &args.ramfs_filename {
            let ramfs_path: &Path = Path::new(ramfs_filename);
            let ramfs_gpa: u64 = sandbox.shared_mem_size() as u64;
            let mapped_size: u64 = sandbox
                .map_file_cow(ramfs_path, ramfs_gpa, Some(RAMFS_LABEL))
                .map_err(|e| {
                    error!("failed to map ramfs file: {e:?}");
                    anyhow::anyhow!("failed to map ramfs file: {e:?}")
                })?;
            layout_ramfs_base = u32::try_from(ramfs_gpa).map_err(|_| {
                error!("new(): ramfs GPA {ramfs_gpa:#x} exceeds u32::MAX");
                anyhow::anyhow!("ramfs GPA {ramfs_gpa:#x} exceeds u32::MAX")
            })?;
            layout_ramfs_size = u32::try_from(mapped_size).map_err(|_| {
                error!("new(): ramfs mapped size {mapped_size:#x} exceeds u32::MAX");
                anyhow::anyhow!("ramfs mapped size {mapped_size:#x} exceeds u32::MAX")
            })?;
            if layout_ramfs_size as usize != ramfs_file_size {
                error!(
                    "new(): ramfs mapped size ({mapped_size:#x}) differs from expected \
                     ramfs_file_size ({ramfs_file_size:#x})"
                );
                return Err(anyhow::anyhow!(
                    "ramfs mapped size ({mapped_size:#x}) differs from expected ramfs_file_size \
                     ({ramfs_file_size:#x})"
                ));
            }
            debug!(
                "ramfs: mapped {:?} ({} bytes) at GPA {:#010x}",
                ramfs_path, mapped_size, ramfs_gpa
            );
        }

        // Build the memory layout descriptor and register the GetMemoryLayout host function.  The
        // guest kernel calls this during init() to discover the authoritative snapshot, RAMFS, and
        // scratch region boundaries instead of inferring them from fragile address calculations.
        //
        // pt_overhead is always 0 with nanvix-unstable (no guest page tables in the snapshot).  The
        // field is preserved for forward-compatibility if upstream Hyperlight re-enables page-table
        // generation for this feature.
        let layout_snapshot_budget: u32 = u32::try_from(snapshot_budget_size).map_err(|_| {
            anyhow::anyhow!("snapshot_budget {snapshot_budget_size:#x} exceeds u32::MAX")
        })?;
        let layout_pt_overhead: u32 = u32::try_from(pt_overhead)
            .map_err(|_| anyhow::anyhow!("pt_overhead {pt_overhead:#x} exceeds u32::MAX"))?;
        let layout_scratch_size: u32 = u32::try_from(scratch_size)
            .map_err(|_| anyhow::anyhow!("scratch_size {scratch_size:#x} exceeds u32::MAX"))?;
        let layout_bytes: Vec<u8> = [
            layout_snapshot_budget.to_le_bytes(),
            layout_pt_overhead.to_le_bytes(),
            layout_ramfs_base.to_le_bytes(),
            layout_ramfs_size.to_le_bytes(),
            layout_scratch_size.to_le_bytes(),
        ]
        .concat();
        sandbox.register("GetMemoryLayout", move || -> Vec<u8> { layout_bytes.clone() })?;
        debug!(
            "GetMemoryLayout: snapshot_budget={:#x}, pt_overhead={:#x}, ramfs_base={:#x}, \
             ramfs_size={:#x}, scratch={:#x}",
            layout_snapshot_budget,
            layout_pt_overhead,
            layout_ramfs_base,
            layout_ramfs_size,
            layout_scratch_size
        );

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

                error!("run(): vmm failed during evolve (error={error:?})");
                return Ok(ErrorCode::ConnectionAborted.into());
            },
        };

        debug!("run(): evolve completed, calling sandbox.call(\"kmain\")");

        // Phase 2: Call — re-enter the guest at _nanvix_dispatch to actually run the system.
        // The kernel writes a FunctionCallResult with the exit code to the PEB output buffer
        // before halting, so sandbox.call() returns Ok(exit_code) through Hyperlight's normal
        // guest function return convention (see issue #2088).
        let call_result: Result<i32, HyperlightError> = sandbox.call("kmain", ());

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
        match call_result {
            Ok(code) => {
                debug!("run(): guest exited (code={code})");
                Ok((code & 0xFF) as u16)
            },
            Err(error) => {
                error!("run(): vmm failed during call (error={error:?})");
                Ok(ErrorCode::ConnectionReset.into())
            },
        }
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

    /// Enable the guest profiler (no-op on Hyperlight backend).
    ///
    /// Hyperlight does not support guest stack sampling because it does not
    /// expose guest registers or memory in the same way as WHP/KVM.
    /// Returns a profiler with an empty sample buffer.
    pub fn enable_guest_profiler(&mut self) -> crate::guest_profiler::GuestProfiler {
        crate::guest_profiler::GuestProfiler::new(0) // No sampling on Hyperlight.
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

    ///
    /// # Description
    ///
    /// Returns the page-aligned size of the kernel after it is loaded in memory.
    ///
    /// This is computed as the end address of the highest PT_LOAD segment in the ELF32 binary,
    /// rounded up to page alignment. The result corresponds to the first GPA after the kernel's
    /// in-memory image, where Hyperlight places the PEB.
    ///
    /// # Parameters
    ///
    /// - `path`: The file path to the kernel ELF32 binary.
    ///
    /// # Returns
    ///
    /// On success, returns the page-aligned in-memory size of the kernel. On failure, returns an
    /// error.
    ///
    fn get_kernel_size(path: &str) -> Result<usize> {
        #[cfg(target_os = "linux")]
        let mapping: crate::pal::FileMapping = crate::pal::FileMapping::mmap(path)?;
        #[cfg(target_os = "windows")]
        let mapping: crate::pal::FileMapping = crate::pal::FileMapping::open(path)?;
        let footprint: crate::elf::MemoryFootprint =
            crate::elf::memory_footprint(mapping.as_slice())?;
        ::sys::mm::align_up(footprint.end(), ::sys::mm::Alignment::Align4096).ok_or_else(|| {
            error!("get_kernel_size(): ELF end address alignment overflow");
            anyhow::anyhow!("ELF end address alignment overflow")
        })
    }

    ///
    /// # Description
    ///
    /// Computes the Hyperlight Guest Heap size needed to bridge the gap between the PEB and the
    /// end of the Kernel Pool.
    ///
    /// Hyperlight lays out the snapshot region (all page-aligned) as:
    ///   [Kernel code + Data] [PEB Struct] [Guest Heap] [Init Data] [Guest Page Tables]
    ///
    /// The Nanvix kernel expects the following physical memory layout:
    ///   [Kernel Code + Data] [Kernel Pool] [Init RAM Disk]
    /// Where the base address of the kernel pool (kpool_base) must be aligned to a page-table
    /// boundary (4 MB).
    ///
    /// Because kpool_base is 4 MB-aligned while kernel_end + PEB_SIZE may not be, the Guest Heap
    /// is split into two parts:
    ///
    ///   guest_heap_padding = kpool_start - (kernel_end + PEB_SIZE)
    ///   KPOOL_SIZE         = size of the kernel page pool
    ///   guest_heap_size    = guest_heap_padding + KPOOL_SIZE
    ///
    /// ```text
    ///   |<-- Kernel -->|<-PEB->|<- padding ->|<---- kpool ---->|
    ///                          ^             ^                 ^
    ///                   guest_heap_start  kpool_start      kpool_end
    ///                          |<- padding ->|<-- KPOOL_SIZE -->|
    ///                          |<-------- guest_heap_size ------>|
    /// ```
    ///
    /// # Parameters
    ///
    /// - `kernel_end`: The first GPA after the kernel's in-memory image (page-aligned).
    /// - `kpool_start`: The base address of the kernel pool (page-table-aligned, 4 MB).
    ///
    /// # Returns
    ///
    /// On success, returns the guest heap size in bytes. On failure (if the PEB overlaps the
    /// kernel pool), returns an error.
    ///
    fn calculate_guest_heap_size(kernel_end: usize, kpool_start: usize) -> Result<usize> {
        let guest_heap_start: usize = kernel_end + ::config::hyperlight::PEB_SIZE;

        if guest_heap_start > kpool_start {
            let reason: String = format!(
                "kernel_end + PEB ({guest_heap_start:#x}) overlaps kpool_start ({kpool_start:#x})"
            );
            error!("calculate_guest_heap_size(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        let guest_heap_padding: usize = kpool_start - guest_heap_start;
        Ok(guest_heap_padding + ::config::kernel::KPOOL_SIZE)
    }

    ///
    /// # Description
    ///
    /// Builds the guest environment from the kernel and optional initrd files.
    ///
    /// If an initrd is provided, it is read into memory and packaged as init_data for the
    /// sandbox. Multibinary images are passed through as-is; single ELF images are wrapped
    /// with a size header and argument trailer.
    ///
    /// # Parameters
    ///
    /// - `kernel_filename`: Path to the kernel ELF binary.
    /// - `initrd_filename`: Optional path to the initial RAM disk file.
    /// - `initrd_args`: Optional arguments to pass to the initrd program.
    ///
    /// # Returns
    ///
    /// On success, returns a tuple of the guest environment and the page-aligned init_data size
    /// in bytes (0 when no initrd is provided). On failure, returns an error.
    ///
    fn build_guest_env(
        kernel_filename: &str,
        initrd_filename: &Option<String>,
        initrd_args: &Option<String>,
    ) -> Result<(GuestEnvironment<'static, 'static>, usize)> {
        let Some(initrd_filename) = initrd_filename else {
            return Ok((
                GuestEnvironment::new(GuestBinary::FilePath(kernel_filename.to_string()), None),
                0,
            ));
        };

        let bytes: Vec<u8> = std::fs::read(initrd_filename).map_err(|err| {
            let reason: String = format!("failed to read initrd file {err:?}");
            error!("build_guest_env(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        let initrd_size: usize = bytes.len();
        debug!("initrd: {} bytes", initrd_size);

        // Detect whether this is a multibinary image or a single ELF.
        let is_multibinary: bool = initrd_size >= ::multibin::MAGIC.len()
            && bytes[..::multibin::MAGIC.len()] == ::multibin::MAGIC;

        // Build the init_data blob based on the initrd format.
        let init_data_bytes: Vec<u8> = if is_multibinary {
            // Multibinary: pass raw image, no wrapping needed.
            debug!("initrd: multibinary format detected, passing raw image");
            bytes
        } else {
            // Single ELF: prepend size header and append args (old format).
            let initrd_args_bytes: Vec<u8> = Self::build_args_bytes(initrd_filename, initrd_args)?;

            let mut padded: Vec<u8> = Vec::with_capacity(
                ::config::hyperlight::INITRD_SIZE_BYTES + initrd_size + initrd_args_bytes.len(),
            );

            // Write the actual size as first INITRD_SIZE_BYTES (little-endian).
            padded.extend_from_slice(&(initrd_size as u64).to_le_bytes());
            padded.extend_from_slice(&bytes);
            padded.extend_from_slice(&initrd_args_bytes);

            debug!(
                "initrd blob: {} bytes total ({} byte header + {} bytes data + {} bytes args)",
                padded.len(),
                ::config::hyperlight::INITRD_SIZE_BYTES,
                initrd_size,
                initrd_args_bytes.len(),
            );

            padded
        };

        let init_data_blob_size: usize = init_data_bytes.len();

        // Page-align the init_data size (Hyperlight places it at page boundaries).
        let init_data_size: usize =
            ::sys::mm::align_up(init_data_blob_size, ::sys::mm::Alignment::Align4096).ok_or_else(
                || {
                    error!("build_guest_env(): init_data alignment overflow");
                    anyhow::anyhow!("init_data alignment overflow")
                },
            )?;

        // Intentionally leaked to obtain a `'static` reference required by GuestBlob.
        // The sandbox owns this memory for the lifetime of the process.
        let boxed_data: Box<[u8]> = init_data_bytes.into_boxed_slice();
        let data_ref: &'static [u8] = Box::leak(boxed_data);

        let guest_env: GuestEnvironment = GuestEnvironment {
            guest_binary: GuestBinary::FilePath(kernel_filename.to_string()),
            init_data: Some(GuestBlob {
                data: data_ref,
                permissions: MemoryRegionFlags::READ
                    | MemoryRegionFlags::WRITE
                    | MemoryRegionFlags::EXECUTE,
            }),
        };

        Ok((guest_env, init_data_size))
    }

    ///
    /// # Description
    ///
    /// Returns the page-aligned size of a RAMFS file.
    ///
    /// # Parameters
    ///
    /// - `path`: The file path to the RAMFS image.
    ///
    /// # Returns
    ///
    /// On success, returns the page-aligned file size. On failure, returns an error.
    ///
    fn get_ramfs_size(path: &str) -> Result<usize> {
        let metadata: std::fs::Metadata = std::fs::metadata(path).map_err(|err| {
            let reason: String = format!("failed to read ramfs metadata for '{path}': {err}");
            error!("get_ramfs_size(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        let size: usize = usize::try_from(metadata.len()).map_err(|_| {
            let reason: &str = "ramfs file size exceeds usize";
            error!("get_ramfs_size(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Page-align the RAMFS size (hyperlight maps at page granularity).
        ::sys::mm::align_up(size, ::sys::mm::Alignment::Align4096).ok_or_else(|| {
            error!("get_ramfs_size(): ramfs size alignment overflow");
            anyhow::anyhow!("ramfs size alignment overflow")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Vmm;

    #[test]
    fn calculate_guest_heap_size_typical() {
        // kernel_end well below kpool_start: padding + KPOOL_SIZE.
        let kernel_end: usize = 0x0020_0000;
        let kpool_start: usize = 0x0040_0000;
        let result: usize = Vmm::calculate_guest_heap_size(kernel_end, kpool_start)
            .expect("should succeed for valid layout");
        let expected_padding: usize = kpool_start - kernel_end - ::config::hyperlight::PEB_SIZE;
        assert_eq!(result, expected_padding + ::config::kernel::KPOOL_SIZE);
    }

    #[test]
    fn calculate_guest_heap_size_overlap() {
        // kernel_end + PEB exceeds kpool_start: must fail.
        let kpool_start: usize = 0x0010_0000;
        let kernel_end: usize = kpool_start; // PEB would push past kpool_start.
        let result = Vmm::calculate_guest_heap_size(kernel_end, kpool_start);
        assert!(result.is_err(), "should fail when PEB overlaps kpool");
    }

    #[test]
    fn get_ramfs_size_missing_file() {
        let result = Vmm::get_ramfs_size("/nonexistent/path/to/ramfs.img");
        assert!(result.is_err(), "should fail for missing file");
    }
}
