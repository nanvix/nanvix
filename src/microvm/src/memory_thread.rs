// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! This module implements the VMM "memory thread", a lightweight worker responsible for relaying
//! messages between the I/O subsystem and the guest, while participating in a simple credit-based
//! flow-control mechanism.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::orchestrator::{
    MemoryControlCommand,
    MemoryControlResponse,
};
use ::anyhow::Result;
use ::std::{
    sync::mpsc::{
        Receiver,
        Sender,
        TryRecvError,
    },
    thread::{
        self,
        JoinHandle,
    },
};
use ::sys::ipc::Message;
use ::syslog::{
    debug,
    error,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Spawns a new memory thread. This thread relays messages between the I/O subsystem and the guest,
/// while participating in a simple credit-based flow-control mechanism.
///
/// # Parameters
///
/// - `data_rx`: Receives data messages from the I/O thread.
/// - `data_tx`: Sends data messages to the virtual machine's stdin.
/// - `control_rx`: Receives control commands from the VMM.
/// - `_control_tx`: Sends control responses to the VMM.
/// - `add_credit`: Closure that adds a credit to the virtual machine credit pool.
///
/// # Returns
///
/// A handle to the memory thread.
///
pub fn spawn<F>(
    data_rx: Receiver<Message>,
    data_tx: Sender<Message>,
    control_rx: Receiver<MemoryControlCommand>,
    _control_tx: Sender<MemoryControlResponse>,
    mut add_credit: F,
) -> JoinHandle<Result<()>>
where
    F: FnMut() -> Result<()> + std::marker::Send + 'static,
{
    thread::spawn(move || {
        loop {
            match control_rx.try_recv() {
                Ok(command) => match command {
                    MemoryControlCommand::Shutdown => {
                        debug!("memory_thread(): received shutdown command");
                        break Ok(());
                    },
                },
                Err(TryRecvError::Disconnected) => {
                    debug!("memory_thread(): VMM control channel has been disconnected");
                    break Ok(());
                },
                Err(TryRecvError::Empty) => {
                    // No message available.
                },
            }
            match data_rx.try_recv() {
                Ok(mut msg) => {
                    profiler::timestamp_message!(
                        &mut msg.payload,
                        mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                            + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                    );
                    if let Err(e) = data_tx.send(msg) {
                        let reason: String = format!("failed to send message: {e:?}");
                        error!("spawn(): {reason}");
                        continue;
                    }
                    add_credit()?;
                },
                Err(TryRecvError::Disconnected) => {
                    // When the guest finishes , the vCPU thread will disconnect from this
                    // thread. This situation is normal and should not create an error log.
                    debug!("spawn(): channel has been disconnected");
                    break Ok(());
                },
                Err(TryRecvError::Empty) => {
                    // No message available.
                },
            }
        }
    })
}
