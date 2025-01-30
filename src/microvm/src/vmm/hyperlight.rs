// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
extern crate kvm_bindings;
#[cfg(target_os = "linux")]
extern crate kvm_ioctls;

use crate::{
    io::IoThread,
    Gateway,
};
use ::anyhow::Result;
use ::hyperlight_host::{
    sandbox::SandboxConfiguration,
    sandbox_state::{
        sandbox::EvolvableSandbox,
        transition::Noop,
    },
    GuestBinary,
    MultiUseSandbox,
    UninitializedSandbox,
};
use ::std::{
    fs::File,
    io::Write,
    sync::{
        mpsc,
        mpsc::{
            Sender,
            TryRecvError,
        },
        Arc,
        Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};
use ::sys::ipc::{
    Message,
    MessageType,
};
use hyperlight_host::{
    func::{
        HostFunction0,
        HostFunction1,
    },
    sandbox::uninitialized::GuestInitrd,
    HyperlightError,
};

//==================================================================================================
// Structure
//==================================================================================================

pub struct Vmm {
    _gateway_tx: Sender<Message>,
    _io_thread: Option<JoinHandle<Result<()>>>,
    sandbox: Option<UninitializedSandbox>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Vmm {
    pub fn new(
        memory_size: usize,
        kernel_filename: &str,
        initrd_filename: Option<String>,
        stderr: Option<String>,
        gateway_conn: Option<Gateway>,
    ) -> Result<Self> {
        crate::timer!("vmm_creation");

        let (vm_tx, gateway_rx) = mpsc::channel::<Message>();
        let (gateway_tx, vm_rx) = mpsc::channel::<Message>();

        // Spawn I/O thread.
        let _io_thread: Option<JoinHandle<Result<()>>> =
            gateway_conn.map(|conn| IoThread::spawn(conn, gateway_rx, gateway_tx.clone()));

        let mut config: SandboxConfiguration = SandboxConfiguration::default();
        config.set_heap_size(4 * 1024 * 1024);
        config.set_stack_size(4 * 1024);
        config.set_max_execution_time(Duration::from_secs(10));
        config.set_max_execution_cancel_wait_time(Duration::from_secs(10));
        config.set_guest_memory_size(memory_size);

        let file_writer = Self::get_stderr_writer(stderr.clone())?;

        let writer_fn = move |s: String| -> Result<i32, HyperlightError> {
            let mut file_writer = file_writer.lock().unwrap();
            file_writer.write_all(s.as_bytes())?;
            Ok(s.len() as i32)
        };

        // Creates Hyperlight sandbox.
        let mut sandbox = UninitializedSandbox::new(
            GuestBinary::FilePath(kernel_filename.to_string()),
            initrd_filename.map(GuestInitrd::FilePath),
            Some(config),
            None, // Use default run options.
            Some(&Arc::new(Mutex::new(writer_fn))),
        )?;

        let vmbus_write = move |data: Vec<u8>| -> Result<i32, HyperlightError> {
            let bytes = data.as_slice();
            let message: Message = match Message::try_from_bytes(
                bytes.try_into().expect("slice with incorrect length"),
            ) {
                Ok(message) => message,
                Err(err) => {
                    let reason: String = format!("failed to parse message: {:?}", err);
                    error!("output(): {}", reason);
                    return Err(HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
                },
            };

            if let Err(e) = vm_tx.send(message) {
                let reason: String = format!("failed to send message: {:?}", e);
                error!("output(): {}", reason);
                return Err(HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
            }

            Ok(data.len() as i32)
        };

        let vmbus_read = move || -> Result<Vec<u8>, HyperlightError> {
            match vm_rx.try_recv() {
                Ok(mut msg) => {
                    msg.message_type = MessageType::Ikc;
                    Ok(msg.to_bytes().to_vec())
                },
                // No message available.
                Err(TryRecvError::Empty) => {
                    let empty_message = Message::default();
                    Ok(empty_message.to_bytes().to_vec())
                },
                // Channel has disconnected.
                Err(TryRecvError::Disconnected) => {
                    let reason: String = "channel has been disconnected".to_string();
                    error!("input(): {}", reason);
                    Err(HyperlightError::AnyhowError(anyhow::Error::msg(reason)))
                },
            }
        };

        let vmbus_write_host_fn = Arc::new(Mutex::new(vmbus_write));
        let vmbus_read_host_fn = Arc::new(Mutex::new(vmbus_read));

        // Register host functions.
        vmbus_read_host_fn.register(&mut sandbox, "VmbusRead")?;
        vmbus_write_host_fn.register(&mut sandbox, "VmbusWrite")?;

        Ok(Self {
            _gateway_tx: gateway_tx,
            _io_thread,
            sandbox: Some(sandbox),
        })
    }

    ///
    /// # Description
    ///
    /// This function runs the virtual machine monitor (VMM) with the given arguments.
    ///
    /// # Parameters
    ///
    /// * `args` - Arguments for the virtual machine monitor.
    pub fn run(&mut self) -> Result<()> {
        crate::timer!("vmm_run");
        if let Some(sandbox) = self.sandbox.take() {
            let _ = sandbox.evolve(Noop::<UninitializedSandbox, MultiUseSandbox>::default())?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Obtains a buffered writer for the virtual machine's standard error device. If the standard
    /// error device is set to a file, the function attempts to open the file and create a buffered
    /// writer. If the standard error device is not set to a file, the function falls back to stderr.
    ///
    /// # Parameters
    ///
    /// * `vm_stderr` - The path to the file where the standard error device is set.
    ///
    /// # Returns
    ///
    /// On success, the function returns a buffered writer for the virtual machine's standard error
    ///
    fn get_stderr_writer(vm_stderr: Option<String>) -> Result<Arc<Mutex<File>>> {
        // Obtain a buffered writer for the virtual machine's standard error device.
        let file_writer: Arc<Mutex<File>> = if let Some(vm_stderr) = vm_stderr {
            // Standard error was set to a file. Attempt to open file and create a writer.
            let file = File::options()
                .read(false)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&vm_stderr)?;
            log::debug!("stderr: {:?}", file);
            Arc::new(Mutex::new(file))
        } else {
            // Standard error was not set to a file. Fallback to stderr.
            Arc::new(Mutex::new(File::create("/dev/stderr")?))
        };
        Ok(file_writer)
    }
}
