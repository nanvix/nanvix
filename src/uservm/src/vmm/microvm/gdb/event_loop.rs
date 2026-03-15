// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! GDB server event loop for the microvm machine type.
//!
//! This module ties the `gdbstub` state machine to the KVM vCPU run loop. It listens on a TCP
//! port for a GDB client connection, then alternates between processing GDB commands and running
//! the vCPU.

//==================================================================================================
// Imports
//==================================================================================================

use super::target::{
    NanvixTarget,
    configure_guest_debug,
};
use crate::vmm::microvm::{
    InteriorMicroVmHandle,
    kvm::{
        vcpu::{
            VirtualProcessor,
            VirtualProcessorExitContext,
        },
        vmem::VirtualMemory,
    },
};
use ::anyhow::Result;
use ::gdbstub::{
    common::Signal,
    conn::ConnectionExt,
    stub::{
        DisconnectReason,
        GdbStub,
        SingleThreadStopReason,
        run_blocking,
    },
};
use ::log::{
    error,
    info,
    warn,
};
use ::std::{
    collections::HashMap,
    io::{
        Read,
        Write,
    },
    net::{
        TcpListener,
        TcpStream,
    },
    sync::Arc,
};
use ::tokio::sync::Mutex;

//==================================================================================================
// TCP Connection Wrapper
//==================================================================================================

/// Wraps a [`TcpStream`] to implement [`gdbstub::conn::Connection`].
struct GdbTcpConnection {
    stream: TcpStream,
}

impl gdbstub::conn::Connection for GdbTcpConnection {
    type Error = std::io::Error;

    fn write(&mut self, byte: u8) -> std::result::Result<(), Self::Error> {
        Write::write_all(&mut self.stream, &[byte])
    }

    fn write_all(&mut self, buf: &[u8]) -> std::result::Result<(), Self::Error> {
        Write::write_all(&mut self.stream, buf)
    }

    fn flush(&mut self) -> std::result::Result<(), Self::Error> {
        Write::flush(&mut self.stream)
    }

    fn on_session_start(&mut self) -> std::result::Result<(), Self::Error> {
        // Disable Nagle's algorithm for low-latency packet delivery.
        self.stream.set_nodelay(true)?;
        Ok(())
    }
}

impl ConnectionExt for GdbTcpConnection {
    fn read(&mut self) -> std::result::Result<u8, Self::Error> {
        let mut buf = [0u8; 1];
        Read::read_exact(&mut self.stream, &mut buf)?;
        Ok(buf[0])
    }

    fn peek(&mut self) -> std::result::Result<Option<u8>, Self::Error> {
        let mut buf = [0u8; 1];
        match self.stream.peek(&mut buf) {
            Ok(1) => Ok(Some(buf[0])),
            Ok(_) => Ok(None),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}

//==================================================================================================
// Blocking Event Loop
//==================================================================================================

/// The blocking event loop that bridges `gdbstub`'s state machine with KVM vCPU execution.
struct NanvixEventLoop;

impl run_blocking::BlockingEventLoop for NanvixEventLoop {
    type Target = NanvixTarget;
    type Connection = GdbTcpConnection;
    type StopReason = SingleThreadStopReason<u64>;

    fn wait_for_stop_reason(
        target: &mut Self::Target,
        conn: &mut Self::Connection,
    ) -> std::result::Result<
        run_blocking::Event<Self::StopReason>,
        run_blocking::WaitForStopReasonError<
            <Self::Target as gdbstub::target::Target>::Error,
            <Self::Connection as gdbstub::conn::Connection>::Error,
        >,
    > {
        // Step over any software breakpoint at the current RIP before resuming execution.
        // This restores the original byte, single-steps past it, then re-inserts INT3.
        if let Some(bp_addr) = target.pending_bp_addr.take()
            && let Some(&orig_byte) = target.sw_breakpoints.get(&bp_addr)
        {
            // Restore the original instruction byte.
            {
                let mut vmem = target.vmem.blocking_lock();
                let _ = vmem.write_bytes(bp_addr, &[orig_byte]);
            }
            // Single-step one instruction past the breakpoint.
            configure_guest_debug(&target.vcpu, true).map_err(|e| {
                error!("gdb: failed to configure single-step for bp step-over: {e:?}");
                run_blocking::WaitForStopReasonError::Target("configure_guest_debug failed")
            })?;
            let step_exit: VirtualProcessorExitContext = {
                let mut vcpu = target.vcpu.blocking_lock();
                vcpu.run()
            };
            // If the step-over caused a non-debug exit, propagate it immediately
            // without re-inserting the breakpoint.
            match step_exit {
                VirtualProcessorExitContext::DebugEvent => {
                    // Expected: single-step completed normally.
                },
                VirtualProcessorExitContext::Halt | VirtualProcessorExitContext::Shutdown => {
                    return Ok(run_blocking::Event::TargetStopped(
                        SingleThreadStopReason::Terminated(Signal::SIGTERM),
                    ));
                },
                VirtualProcessorExitContext::Interrupted => {
                    return Ok(run_blocking::Event::TargetStopped(SingleThreadStopReason::Signal(
                        Signal::SIGINT,
                    )));
                },
                VirtualProcessorExitContext::Pmio(access) => {
                    // Handle I/O, then re-insert the breakpoint and continue.
                    let exit_status = target
                        .inner
                        .blocking_lock()
                        .emulator_mut()
                        .handle_pmio_access(&access)
                        .map_err(|e| {
                            error!("gdb: emulator failed on PMIO during bp step-over: {e:?}");
                            run_blocking::WaitForStopReasonError::Target(
                                "emulator PMIO handling failed",
                            )
                        })?;
                    if let Some(status) = exit_status
                        && status != ::config::microvm::DEFAULT_VMM_PAUSE_CMD
                        && status != ::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD
                    {
                        return Ok(run_blocking::Event::TargetStopped(
                            SingleThreadStopReason::Exited(u8::try_from(status).unwrap_or(1)),
                        ));
                    }
                },
                VirtualProcessorExitContext::Unknown => {
                    warn!("gdb: unknown vCPU exit during bp step-over");
                    return Ok(run_blocking::Event::TargetStopped(SingleThreadStopReason::Signal(
                        Signal::SIGABRT,
                    )));
                },
            }
            // Re-insert the INT3 breakpoint.
            {
                let mut vmem = target.vmem.blocking_lock();
                let _ = vmem.write_bytes(bp_addr, &[super::target::INT3]);
            }
            // If the user requested single-step, we've done the step — return now.
            if target.single_step {
                target.single_step = false;
                return Ok(run_blocking::Event::TargetStopped(SingleThreadStopReason::DoneStep));
            }
        }

        loop {
            // Check for incoming GDB data (e.g., Ctrl+C break request) before running the vCPU.
            conn.stream
                .set_nonblocking(true)
                .map_err(run_blocking::WaitForStopReasonError::Connection)?;
            let has_incoming = conn
                .peek()
                .map_err(run_blocking::WaitForStopReasonError::Connection)?;
            conn.stream
                .set_nonblocking(false)
                .map_err(run_blocking::WaitForStopReasonError::Connection)?;
            if let Some(byte) = has_incoming {
                return Ok(run_blocking::Event::IncomingData(byte));
            }

            // Configure KVM debug flags before each run.
            configure_guest_debug(&target.vcpu, target.single_step).map_err(|e| {
                error!("gdb: failed to configure guest debug: {e:?}");
                run_blocking::WaitForStopReasonError::Target("configure_guest_debug failed")
            })?;

            // Run the vCPU until it exits.
            let exit_context: VirtualProcessorExitContext = {
                let mut vcpu = target.vcpu.blocking_lock();
                vcpu.run()
            };

            match exit_context {
                VirtualProcessorExitContext::DebugEvent => {
                    // A debug exit may be caused by a software breakpoint (INT3) or single-step.
                    if target.single_step {
                        target.single_step = false;
                        return Ok(run_blocking::Event::TargetStopped(
                            SingleThreadStopReason::DoneStep,
                        ));
                    } else {
                        // Software breakpoint: rewind RIP past the INT3 byte and record
                        // the breakpoint address for step-over on the next resume.
                        let vcpu = target.vcpu.blocking_lock();
                        if let Ok(regs) = vcpu.get_regs() {
                            // INT3 is 1 byte, so RIP points one byte past the breakpoint.
                            let bp_addr: u64 = regs.rip.wrapping_sub(1);
                            if target.sw_breakpoints.contains_key(&bp_addr) {
                                let mut regs_fixed = regs;
                                regs_fixed.rip = bp_addr;
                                if let Err(e) = vcpu.set_regs(&regs_fixed) {
                                    error!("gdb: failed to rewind RIP: {e:?}");
                                }
                                target.pending_bp_addr = Some(bp_addr);
                            }
                        }
                        return Ok(run_blocking::Event::TargetStopped(
                            SingleThreadStopReason::SwBreak(()),
                        ));
                    }
                },
                VirtualProcessorExitContext::Halt => {
                    return Ok(run_blocking::Event::TargetStopped(
                        SingleThreadStopReason::Terminated(Signal::SIGTERM),
                    ));
                },
                VirtualProcessorExitContext::Shutdown => {
                    return Ok(run_blocking::Event::TargetStopped(
                        SingleThreadStopReason::Terminated(Signal::SIGTERM),
                    ));
                },
                VirtualProcessorExitContext::Interrupted => {
                    return Ok(run_blocking::Event::TargetStopped(SingleThreadStopReason::Signal(
                        Signal::SIGINT,
                    )));
                },
                VirtualProcessorExitContext::Pmio(access) => {
                    // Emulate the port I/O access (stdout, stdin, VMM commands) and resume.
                    let exit_status = target
                        .inner
                        .blocking_lock()
                        .emulator_mut()
                        .handle_pmio_access(&access)
                        .map_err(|e| {
                            error!("gdb: emulator failed on PMIO: {e:?}");
                            run_blocking::WaitForStopReasonError::Target(
                                "emulator PMIO handling failed",
                            )
                        })?;
                    match exit_status {
                        Some(status)
                            if status != ::config::microvm::DEFAULT_VMM_PAUSE_CMD
                                && status != ::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD =>
                        {
                            return Ok(run_blocking::Event::TargetStopped(
                                SingleThreadStopReason::Exited(u8::try_from(status).unwrap_or(1)),
                            ));
                        },
                        _ => {
                            // Normal I/O or pause/snapshot — continue the run loop.
                            continue;
                        },
                    }
                },
                VirtualProcessorExitContext::Unknown => {
                    warn!("gdb: unknown vCPU exit during debug run");
                    return Ok(run_blocking::Event::TargetStopped(SingleThreadStopReason::Signal(
                        Signal::SIGABRT,
                    )));
                },
            }
        }
    }

    fn on_interrupt(
        _target: &mut Self::Target,
    ) -> std::result::Result<
        Option<Self::StopReason>,
        <Self::Target as gdbstub::target::Target>::Error,
    > {
        // GDB sent a break (Ctrl+C). Return a signal to pause the guest.
        Ok(Some(SingleThreadStopReason::Signal(Signal::SIGINT)))
    }
}

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Runs the GDB server, blocking until the GDB session ends or the guest terminates.
///
/// This function:
/// 1. Listens on `port` for a GDB client connection.
/// 2. Enables KVM guest debug mode.
/// 3. Enters the `gdbstub` state machine loop.
/// 4. Returns the guest exit status when the session ends.
///
/// # Safety
///
/// Must be called from the vCPU thread (the same thread that calls `KVM_RUN`).
pub(crate) fn run_gdb_server(
    port: u16,
    vcpu: Arc<Mutex<VirtualProcessor>>,
    vmem: Arc<Mutex<VirtualMemory>>,
    inner: Arc<Mutex<InteriorMicroVmHandle>>,
) -> Result<u16> {
    let addr = format!("127.0.0.1:{port}");
    info!("gdb: listening for GDB connection on {addr}");

    let listener = TcpListener::bind(&addr)
        .map_err(|e| anyhow::anyhow!("gdb: failed to bind TCP listener on {addr}: {e:?}"))?;

    info!("gdb: waiting for GDB client to connect...");
    let (stream, peer) = listener
        .accept()
        .map_err(|e| anyhow::anyhow!("gdb: failed to accept TCP connection: {e:?}"))?;
    info!("gdb: client connected from {peer}");

    // Enable guest debug mode before starting.
    configure_guest_debug(&vcpu, false)?;

    let connection = GdbTcpConnection { stream };
    let gdb = GdbStub::new(connection);

    let mut target = NanvixTarget {
        vcpu: vcpu.clone(),
        vmem,
        inner,
        sw_breakpoints: HashMap::new(),
        single_step: false,
        pending_bp_addr: None,
    };

    match gdb.run_blocking::<NanvixEventLoop>(&mut target) {
        Ok(disconnect_reason) => {
            match disconnect_reason {
                DisconnectReason::Disconnect => {
                    info!("gdb: client disconnected");
                    // Disable guest debug mode so the VM can run freely.
                    let dbg = kvm_bindings::kvm_guest_debug::default();
                    let _ = vcpu.blocking_lock().set_guest_debug(&dbg);
                },
                DisconnectReason::TargetExited(code) => {
                    info!("gdb: target exited with code {code}");
                    return Ok(u16::from(code));
                },
                DisconnectReason::TargetTerminated(sig) => {
                    info!("gdb: target terminated with signal {sig:?}");
                },
                DisconnectReason::Kill => {
                    info!("gdb: target killed by GDB");
                },
            }
            Ok(vcpu.blocking_lock().exit_status())
        },
        Err(e) => {
            error!("gdb: stub error: {e:?}");
            anyhow::bail!("GDB stub error: {e:?}")
        },
    }
}
