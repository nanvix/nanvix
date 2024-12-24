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
    kvm::vmem::VirtualMemory,
    microvm::{
        self,
        MicroVm,
    },
};
use ::anyhow::Result;
use ::std::{
    cell::RefCell,
    fs::File,
    io::Write,
    mem,
    os::unix::net::UnixStream,
    rc::Rc,
    sync::{
        mpsc,
        mpsc::{
            Receiver,
            Sender,
            TryRecvError,
        },
    },
    thread::JoinHandle,
    time::Duration,
};
use ::sys::ipc::{
    Message,
    MessageType,
};

//==================================================================================================
// Structure
//==================================================================================================

pub struct Vmm {
    _gateway_tx: Sender<Message>,
    _io_thread: Option<JoinHandle<Result<()>>>,
    microvm: MicroVm,
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
        gateway_addr: Option<String>,
    ) -> Result<Self> {
        crate::timer!("vmm_creation");

        let (vm_tx, gateway_rx) = mpsc::channel::<Message>();
        let (gateway_tx, vm_rx) = mpsc::channel::<Message>();
        let gateway_conn: Option<UnixStream> = match gateway_addr {
            Some(addr) => match UnixStream::connect(addr.clone()) {
                Ok(conn) => {
                    conn.set_read_timeout(Some(Duration::from_millis(1)))?;
                    Some(conn)
                },
                Err(e) => {
                    let reason: String = format!(
                        "failed to connect to gateway (gateway_addr={:?}, error={:?})",
                        addr, e
                    );
                    error!("io_thread(): {}", reason);
                    anyhow::bail!(reason)
                },
            },
            None => None,
        };

        // Spawn I/O thread.
        let _io_thread: Option<JoinHandle<Result<()>>> =
            gateway_conn.map(|conn| IoThread::spawn(conn, gateway_rx, gateway_tx.clone()));

        // Input function used for emulating I/O port reads.
        let input: Box<microvm::InputFn> = Self::build_input_fn(vm_rx);

        // Output function used for emulating I/O port writes.
        let output: Box<microvm::OutputFn> =
            Self::build_output_fn(Self::get_stderr_writer(stderr.clone())?, vm_tx);

        let mut microvm: MicroVm = MicroVm::new(memory_size, input, output)?;

        let rip: u64 = microvm.load_kernel(kernel_filename)?;
        if let Some(ref initrd_filename) = initrd_filename {
            microvm.load_initrd(initrd_filename)?;
        }

        microvm.reset(rip)?;

        Ok(Self {
            _gateway_tx: gateway_tx,
            _io_thread,
            microvm,
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
        self.microvm.run()?;

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
    fn get_stderr_writer(vm_stderr: Option<String>) -> Result<Box<dyn Write>> {
        // Obtain a buffered writer for the virtual machine's standard error device.
        let file_writer: Box<dyn Write> = if let Some(vm_stderr) = vm_stderr {
            // Standard error was set to a file. Attempt to open file and create a writer.
            let file = File::options()
                .read(false)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&vm_stderr)?;
            Box::new(file)
        } else {
            // Standard error was not set to a file. Fallback to stderr.
            Box::new(std::io::stderr())
        };
        Ok(file_writer)
    }

    fn build_input_fn(input_queue: Receiver<Message>) -> Box<microvm::InputFn> {
        // Input function used for emulating I/O port reads.
        let input = move |vm: &Rc<RefCell<VirtualMemory>>, data, size| -> Result<()> {
            // Check for invalid operand size.
            if size != 4 {
                let reason: String = format!("invalid operand size (size={:?})", size);
                error!("input(): {}", reason);
                anyhow::bail!(reason);
            }

            match input_queue.try_recv() {
                Ok(mut msg) => {
                    msg.message_type = MessageType::Ikc;
                    vm.borrow_mut().write_bytes(data as u64, &msg.to_bytes())?;
                },
                // No message available.
                Err(TryRecvError::Empty) => {
                    let empty_message = Message::default();
                    vm.borrow_mut()
                        .write_bytes(data as u64, &empty_message.to_bytes())?;
                },
                // Channel has disconnected.
                Err(TryRecvError::Disconnected) => {
                    let reason: String = "channel has been disconnected".to_string();
                    error!("input(): {}", reason);
                    anyhow::bail!(reason);
                },
            }

            Ok(())
        };

        Box::new(input)
    }

    fn build_output_fn(
        mut file_writer: Box<dyn Write>,
        queue: Sender<Message>,
    ) -> Box<microvm::OutputFn> {
        // Output function used for emulating I/O port writes.
        let output = move |vm: &Rc<RefCell<VirtualMemory>>, data, size| -> Result<()> {
            // Parse operand size do determine how to handle the operation.
            if size == 1 {
                // Write to the standard error device.

                // Convert data to a character.
                let ch: char = match char::from_u32(data) {
                    // Valid character.
                    Some(ch) => ch,
                    // Invalid character.
                    None => {
                        let reason: String = format!("invalid character (data={:?})", data);
                        error!("output(): {}", reason);
                        anyhow::bail!(reason);
                    },
                };

                let buf: &[u8] = &[ch as u8];

                file_writer.write_all(buf)?;

                Ok(())
            } else {
                // Write to the standard output device.
                let mut bytes: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
                vm.borrow_mut().read_bytes(data as u64, &mut bytes)?;

                let message: Message = match Message::try_from_bytes(bytes) {
                    Ok(message) => message,
                    Err(err) => {
                        let reason: String = format!("failed to parse message: {:?}", err);
                        error!("output(): {}", reason);
                        anyhow::bail!(reason);
                    },
                };

                if let Err(e) = queue.send(message) {
                    let reason: String = format!("failed to send message: {:?}", e);
                    error!("output(): {}", reason);
                    anyhow::bail!(reason);
                }

                Ok(())
            }
        };

        Box::new(output)
    }
}
