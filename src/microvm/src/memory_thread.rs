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

use crate::orchestrator::{
    MemoryControlCommand,
    MemoryControlResponse,
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
/// - `control_tx`: Sends control responses to the VMM.
/// - `add_credit`: Closure that adds a credit to the virtual machine credit pool.
/// - `pause_microvm`: Closure that writes to the kernel's memory to pause the MicroVM.
/// - `resume_microvm`: Closure that writes to the kernel's memory it shouldn't pause the MicroVM.
///
/// # Returns
///
/// A handle to the memory thread.
///
pub fn spawn<F1, F2, F3>(
    data_rx: Receiver<Message>,
    data_tx: Sender<Message>,
    control_rx: Receiver<MemoryControlCommand>,
    control_tx: Sender<MemoryControlResponse>,
    mut add_credit: F1,
    mut pause_microvm: F2,
    mut resume_microvm: F3,
) -> JoinHandle<Result<()>>
where
    F1: FnMut() -> Result<()> + std::marker::Send + 'static,
    F2: FnMut() -> Result<()> + std::marker::Send + 'static,
    F3: FnMut() -> Result<()> + std::marker::Send + 'static,
{
    thread::spawn(move || {
        loop {
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
            match control_rx.try_recv() {
                Ok(MemoryControlCommand::Pause) => {
                    trace!("pause()");
                    crate::timer!("vm_pause");
                    if pause_microvm().is_err() {
                        control_tx.send(MemoryControlResponse::PauseError)?;
                    }
                },
                Ok(MemoryControlCommand::Resume) => {
                    trace!("resume()");
                    crate::timer!("vm_resume");
                    if resume_microvm().is_err() {
                        control_tx.send(MemoryControlResponse::ResumeError)?;
                    }
                    control_tx.send(MemoryControlResponse::ResumeWritten)?;
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
