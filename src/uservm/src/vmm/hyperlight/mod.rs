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
use ::arch::mem::PAGE_SIZE;
use ::core::convert::TryFrom;
use ::hyperlight_host::{
    GuestBinary,
    HyperlightError,
    MultiUseSandbox,
    UninitializedSandbox,
    mem::{
        memory_region::MemoryRegionFlags,
        mgr::SandboxMemoryManager,
        shared_mem::ExclusiveSharedMemory,
    },
    sandbox::{
        SandboxConfiguration,
        uninitialized::{
            GuestBlob,
            GuestEnvironment,
        },
    },
};
use ::std::{
    io::Write,
    path::Path,
    sync::Arc,
};
use ::sys::error::ErrorCode;
use ::syslog::{
    debug,
    error,
};
use ::tokio::{
    runtime::Handle,
    sync::{
        Mutex,
        mpsc::Sender,
    },
    task,
};

//==================================================================================================
// Types
//==================================================================================================

pub type StdinFn = dyn FnMut() -> Result<Vec<u8>, HyperlightError> + Send;

pub type StdoutFn = dyn FnMut(Vec<u8>) -> Result<i32, HyperlightError> + Send;

pub type StderrFn = dyn Write + Send;

//==================================================================================================
// Structure
//==================================================================================================

pub struct VirtualMemory {
    manager: SandboxMemoryManager<ExclusiveSharedMemory>,
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
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Vmm {
    pub fn new(args: MicroVmArgs) -> Result<Self> {
        let guest: Guest = Guest::default();

        // Required values for heap and stack sizes to be used by the kernel.
        let heap_size: usize = 4 * 1024 * 1024;
        let stack_size: usize = 4 * 1024;
        let memory_size: usize = args.memory_size;

        let guest_env: GuestEnvironment = if let Some(initrd_filename) = &args.initrd_filename {
            match std::fs::read(initrd_filename) {
                Ok(bytes) => {
                    let initrd_size: usize = bytes.len();
                    debug!("initrd: {} bytes", initrd_size);

                    let kernel_metadata: ::std::fs::Metadata =
                        std::fs::metadata(&args.kernel_filename).map_err(|e| {
                            let reason: String =
                                format!("failed to read kernel file metadata: {}", e);
                            error!("initrd(): {}", reason);
                            anyhow::anyhow!(reason)
                        })?;
                    let kernel_size: usize =
                        usize::try_from(kernel_metadata.len()).map_err(|_| {
                            let reason: String = format!(
                                "kernel file size {} exceeds supported range",
                                kernel_metadata.len()
                            );
                            error!("initrd(): {}", reason);
                            anyhow::anyhow!(reason)
                        })?;

                    let initrd_args_bytes: Vec<u8> =
                        Self::build_args_bytes(initrd_filename, &args.initrd_args)?;

                    // PEB, I/O buffers, host fxn defs, guard pages, etc.
                    let reserved_pages: usize = 11 * PAGE_SIZE;

                    let required_memory: usize = kernel_size
                        + initrd_size
                        + (heap_size + stack_size)
                        + reserved_pages
                        + ::config::hyperlight::INITRD_SIZE_BYTES
                        + initrd_args_bytes.len();

                    // Check if required memory exceeds memory size.
                    if memory_size <= required_memory {
                        let reason: &str = "not enough memory";
                        error!(
                            "new(): {reason} ({required_memory} bytes required, {memory_size} \
                             bytes total)"
                        );
                        return Err(anyhow::anyhow!(reason));
                    }

                    let padding_size: usize = memory_size - required_memory;

                    // Create a new vector with size header + original data + padding
                    let mut padded_bytes: Vec<u8> = Vec::with_capacity(
                        ::config::hyperlight::INITRD_SIZE_BYTES
                            + initrd_size
                            + initrd_args_bytes.len(),
                    );

                    // Write the actual size as first INITRD_SIZE_BYTES-bytes (little-endian)
                    padded_bytes.extend_from_slice(&(initrd_size as u64).to_le_bytes());

                    // Add the actual initrd data
                    padded_bytes.extend_from_slice(&bytes);

                    // Append length-prefixed initrd arguments so the guest can consume them.
                    padded_bytes.extend_from_slice(&initrd_args_bytes);

                    debug!(
                        "initrd blob: {} bytes total ({} byte header + {} bytes data + 1 byte \
                         args length + {} bytes args payload), extra memory: {} bytes",
                        padded_bytes.len(),
                        ::config::hyperlight::INITRD_SIZE_BYTES,
                        initrd_size,
                        initrd_args_bytes.len(),
                        padding_size
                    );

                    // Box the data to extend its lifetime
                    let boxed_data: Box<[u8]> = padded_bytes.into_boxed_slice();
                    let data_ref: &'static [u8] = Box::leak(boxed_data);

                    let extra_memory: u64 = padding_size.try_into().map_err(|_| {
                        let reason: String =
                            format!("padding size {} exceeds supported range", padding_size);
                        error!("initrd(): {}", reason);
                        anyhow::anyhow!(reason)
                    })?;

                    GuestEnvironment {
                        guest_binary: GuestBinary::FilePath(args.kernel_filename.to_string()),
                        init_data: Some(GuestBlob {
                            data: data_ref,
                            permissions: MemoryRegionFlags::READ
                                | MemoryRegionFlags::WRITE
                                | MemoryRegionFlags::EXECUTE,
                        }),
                        extra_memory: Some(extra_memory),
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

        let mut config: SandboxConfiguration = SandboxConfiguration::default();
        let heap_size_u64: u64 = u64::try_from(heap_size).map_err(|_| {
            let reason: String = format!("heap size {} exceeds supported range", heap_size);
            error!("hyperlight::new(): {}", reason);
            anyhow::anyhow!(reason)
        })?;
        config.set_heap_size(heap_size_u64);

        let stack_size_u64: u64 = u64::try_from(stack_size).map_err(|_| {
            let reason: String = format!("stack size {} exceeds supported range", stack_size);
            error!("hyperlight::new(): {}", reason);
            anyhow::anyhow!(reason)
        })?;
        config.set_stack_size(stack_size_u64);

        // Creates Hyperlight sandbox.
        let mut sandbox: UninitializedSandbox = UninitializedSandbox::new(guest_env, Some(config))?;
        let manager: SandboxMemoryManager<ExclusiveSharedMemory> = sandbox.mgr.unwrap_mgr().clone();
        let vmem: Arc<Mutex<VirtualMemory>> = Arc::new(Mutex::new(VirtualMemory {
            manager: manager.clone(),
        }));

        let guest: Arc<Mutex<Guest>> = Arc::new(Mutex::new(guest));

        // Create a closure that takes a String and writes it to stderr.
        // NOTE: underlying writer implements `Write` and requires mutable access.
        let mut stderr_writer: Box<StderrFn> = args.stderr;
        sandbox.register_print(move |s: String| -> i32 {
            if stderr_writer.write_all(s.as_bytes()).is_err() {
                return -1;
            }
            if stderr_writer.flush().is_err() {
                return -1;
            }
            0
        })?;

        // Create a closure for VmbusWrite that matches the expected signature
        // NOTE: output function is FnMut, so we must keep it mutable when captured.
        let mut output_fn: Box<StdoutFn> = args.output;
        sandbox.register("VmbusWrite", move |data: Vec<u8>| -> i32 {
            output_fn(data).unwrap_or(-1)
        })?;

        // Create a closure for VmbusRead that matches the expected signature
        // NOTE: input function is FnMut, so we must keep it mutable when captured.
        let mut input_fn: Box<StdinFn> = args.input;
        sandbox.register("VmbusRead", move || -> Vec<u8> { input_fn().unwrap_or_default() })?;

        Ok(Self {
            vmem,
            sandbox: Arc::new(Mutex::new(Some(sandbox))),
            guest,
            inner: Arc::new(Mutex::new(InnerVmm {
                control_tx: args.control_tx,
            })),
        })
    }

    pub fn spawn(mut self) -> tokio::task::JoinHandle<Result<u16>> {
        task::spawn_blocking(move || {
            let pthread_id: libc::pthread_t = unsafe { libc::pthread_self() };
            Handle::current().block_on(self.send_tid(pthread_id))?;
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

        // Run the sandbox.
        let result: Result<MultiUseSandbox, HyperlightError> = uninit.evolve();

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
        match result {
            Ok(_multiuse_sandbox) => {
                error!("run(): vmm exited");
                Ok(ErrorCode::ConnectionAborted.into())
            },
            Err(error) => {
                // note: this is a bit of a hack to check for the shutdown command.
                if !error
                    .to_string()
                    .contains(&::config::hyperlight::DEFAULT_VMM_SHUTDOWN_CMD.to_string())
                {
                    error!("run(): vmm aborted (error={error:?})");
                    Ok(ErrorCode::ConnectionReset.into())
                } else {
                    // FIXME (#1010): the vCPU thread already returns 0 always, but we will be able to remove
                    // this line once we can join the vCPU thread.
                    Ok(0)
                }
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
    async fn send_tid(&self, tid: libc::pthread_t) -> Result<()> {
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
        if let Some(program_args) = program_args {
            if !program_args.is_empty() {
                args_string.push(' ');
                args_string.push_str(program_args);
            }
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
