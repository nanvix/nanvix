// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Spawns a new memory thread.
///
/// # Parameters
///
/// - `memory_thread_rx`: Receives messages from I/O thread.
/// - `memory_thread_tx`: Sends messages to the virtual machine's stdin.
/// - `add_credit`: Closure that adds a credit to the virtual machine credit pool.
///
/// # Returns
///
/// A handle to the I/O thread.
///
pub fn spawn<F>(
    memory_thread_rx: Receiver<Message>,
    memory_thread_tx: Sender<Message>,
    mut add_credit: F,
) -> JoinHandle<Result<()>>
where
    F: FnMut() -> Result<()> + std::marker::Send + 'static,
{
    thread::spawn(move || {
        loop {
            match memory_thread_rx.try_recv() {
                Ok(mut msg) => {
                    profiler::timestamp_message!(
                        &mut msg.payload,
                        mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                            + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                    );
                    if let Err(e) = memory_thread_tx.send(msg) {
                        let reason: String = format!("failed to send message: {e:?}");
                        error!("memory_thread(): {reason}");
                        continue;
                    }
                    add_credit()?;
                },
                Err(TryRecvError::Disconnected) => {
                    // When the guest finishes , the vCPU thread will disconnect from this
                    // thread. This situation is normal and should not create an error log.
                    debug!("memory_thread(): channel has been disconnected");
                    break Ok(());
                },
                Err(TryRecvError::Empty) => {
                    // No message available.
                },
            }
        }
    })
}
