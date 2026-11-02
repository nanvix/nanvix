// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::anyhow::Result as AnyResult;
use ::std::{
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    time::Duration,
};
use ::tokio::{
    sync::mpsc,
    time::{
        sleep,
        timeout,
    },
};

// Helper to build a fresh orchestrator and all channels for each test.
struct Harness {
    orchestrator: Orchestrator,
    // Channels we observe.
    io_cmd_tx: mpsc::Sender<IoControlCommand>,
    io_resp_rx: mpsc::Receiver<IoControlResponse>,
    mem_cmd_rx: mpsc::Receiver<MemoryControlCommand>,
    vcpu_resp_tx: mpsc::Sender<VcpuControlResponse>,
    vcpu_cmd_rx: mpsc::Receiver<VcpuControlCommand>,
    pause_called: Arc<AtomicBool>,
    resume_called: Arc<AtomicBool>,
    shutdown_called: Arc<AtomicBool>,
}

fn build_harness() -> Harness {
    let (io_cmd_tx, io_cmd_rx) = mpsc::channel(8);
    let (io_resp_tx, io_resp_rx) = mpsc::channel(8);
    let (mem_resp_tx, mem_resp_rx) = mpsc::channel(1); // Memory -> VMM (unused)
    let _ = mem_resp_tx; // silence unused warning
    let (mem_cmd_tx, mem_cmd_rx) = mpsc::channel(2); // VMM -> Memory (we assert on this)
    let (vcpu_resp_tx, vcpu_resp_rx) = mpsc::channel(8); // vCPU -> VMM
    let (vcpu_cmd_tx, vcpu_cmd_rx) = mpsc::channel(8); // VMM -> vCPU

    let pause_called: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let resume_called: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let snapshot_called: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let shutdown_called: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let pause_flag: Arc<AtomicBool> = pause_called.clone();
    let resume_flag: Arc<AtomicBool> = resume_called.clone();
    let snapshot_flag: Arc<AtomicBool> = snapshot_called.clone();
    let shutdown_flag: Arc<AtomicBool> = shutdown_called.clone();

    let orchestrator: Orchestrator = Orchestrator::new(
        0, // vcpu_tid (unused for these tests)
        io_cmd_rx,
        io_resp_tx,
        mem_resp_rx,
        mem_cmd_tx,
        vcpu_resp_rx,
        vcpu_cmd_tx,
        Box::new(move || {
            let flag = pause_flag.clone();
            Box::pin(async move {
                flag.store(true, Ordering::SeqCst);
                Ok(())
            })
        }),
        Box::new(move || {
            let flag = resume_flag.clone();
            Box::pin(async move {
                flag.store(true, Ordering::SeqCst);
                Ok(())
            })
        }),
        Box::new(move || {
            let flag = snapshot_flag.clone();
            Box::pin(async move {
                flag.store(true, Ordering::SeqCst);
                Ok(())
            })
        }),
        {
            let snapshot_flag2: Arc<AtomicBool> = snapshot_called.clone();
            Box::new(move || {
                let flag = snapshot_flag2.clone();
                Box::pin(async move {
                    flag.store(true, Ordering::SeqCst);
                    Ok(())
                })
            })
        },
        Box::new(move |_vcpu_tid| shutdown_flag.store(true, Ordering::SeqCst)),
    );

    Harness {
        orchestrator,
        io_cmd_tx,
        io_resp_rx,
        mem_cmd_rx,
        vcpu_resp_tx,
        vcpu_cmd_rx,
        pause_called,
        resume_called,
        shutdown_called,
    }
}

// Utility to receive the next IO response with a timeout.
async fn recv_io_resp(rx: &mut mpsc::Receiver<IoControlResponse>) -> IoControlResponse {
    timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("io response timed out")
        .expect("io response channel closed")
}

#[tokio::test]
async fn run_shutdown_sends_cleanup_messages() -> AnyResult<()> {
    let mut h: Harness = build_harness();
    let handle: JoinHandle<Result<()>> = h.orchestrator.spawn();

    // Trigger shutdown via vCPU.
    h.vcpu_resp_tx.send(VcpuControlResponse::Shutdown).await?;

    // Expect memory shutdown command after run loop exits.
    let mem_cmd: MemoryControlCommand = timeout(Duration::from_millis(500), h.mem_cmd_rx.recv())
        .await?
        .expect("memory control channel closed unexpectedly");
    assert!(matches!(mem_cmd, MemoryControlCommand::Shutdown));

    // Expect IO shutdown response.
    let io_resp: IoControlResponse = recv_io_resp(&mut h.io_resp_rx).await;
    assert!(matches!(io_resp, IoControlResponse::Shutdown));

    // Join orchestrator task (should be Ok(()))
    let res: Result<()> = handle.await.expect("orchestrator join failed");
    assert!(res.is_ok());
    Ok(())
}

#[tokio::test]
async fn shutdown_timeout_uses_backend_callback() -> AnyResult<()> {
    let mut h: Harness = build_harness();
    h.orchestrator.state = State::ShuttingDown;
    h.orchestrator.shutdown_timeout = Duration::from_millis(1);
    let handle: JoinHandle<Result<()>> = h.orchestrator.spawn();

    let mem_cmd: MemoryControlCommand = timeout(Duration::from_millis(500), h.mem_cmd_rx.recv())
        .await?
        .expect("memory control channel closed unexpectedly");
    assert!(matches!(mem_cmd, MemoryControlCommand::Shutdown));
    assert!(h.shutdown_called.load(Ordering::SeqCst));

    let io_resp: IoControlResponse = recv_io_resp(&mut h.io_resp_rx).await;
    assert!(matches!(io_resp, IoControlResponse::Shutdown));

    let result: Result<()> = handle.await.expect("orchestrator join failed");
    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn shutdown_ignores_closed_io_control_channel() -> AnyResult<()> {
    let mut h: Harness = build_harness();
    h.orchestrator.state = State::ShuttingDown;
    drop(h.io_cmd_tx);

    let handle: JoinHandle<Result<()>> = h.orchestrator.spawn();
    h.vcpu_resp_tx.send(VcpuControlResponse::Shutdown).await?;

    let mem_cmd: MemoryControlCommand = timeout(Duration::from_millis(500), h.mem_cmd_rx.recv())
        .await?
        .expect("memory control channel closed unexpectedly");
    assert!(matches!(mem_cmd, MemoryControlCommand::Shutdown));

    let io_resp: IoControlResponse = recv_io_resp(&mut h.io_resp_rx).await;
    assert!(matches!(io_resp, IoControlResponse::Shutdown));

    let result: Result<()> = handle.await.expect("orchestrator join failed");
    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn pause_and_resume_flow() -> AnyResult<()> {
    let mut h: Harness = build_harness();
    let handle: JoinHandle<Result<()>> = h.orchestrator.spawn();

    // Move to Running state.
    h.io_cmd_tx.send(IoControlCommand::_StartMicroVm).await?;
    // Request pause.
    h.io_cmd_tx.send(IoControlCommand::_PauseMicroVm).await?;

    // Simulate the vCPU acknowledging the pause and the I/O handler completing its flush.
    let vcpu_resp_tx_clone: mpsc::Sender<VcpuControlResponse> = h.vcpu_resp_tx.clone();
    let io_cmd_tx_clone: mpsc::Sender<IoControlCommand> = h.io_cmd_tx.clone();
    tokio::spawn(async move {
        // Allow orchestrator to enter pause_protocol loop.
        sleep(Duration::from_millis(5)).await;
        let _ = vcpu_resp_tx_clone.send(VcpuControlResponse::Paused).await;
        // Allow orchestrator to send FlushOutput & FlushInput, then provide SystemCallFlushed.
        sleep(Duration::from_millis(5)).await;
        let _ = io_cmd_tx_clone
            .send(IoControlCommand::SystemCallFlushed)
            .await;
    });

    // Expect FlushOutput, FlushInput, MicroVmPaused in order.
    let r1 = recv_io_resp(&mut h.io_resp_rx).await;
    assert!(matches!(r1, IoControlResponse::FlushOutput));
    let r2 = recv_io_resp(&mut h.io_resp_rx).await;
    assert!(matches!(r2, IoControlResponse::FlushInput));
    let r3 = recv_io_resp(&mut h.io_resp_rx).await;
    assert!(matches!(r3, IoControlResponse::MicroVmPaused));
    assert!(h.pause_called.load(Ordering::SeqCst));

    // Resume.
    h.io_cmd_tx.send(IoControlCommand::_ResumeMicroVm).await?;
    // Expect a Resume command sent to vCPU thread.
    let resume_cmd: VcpuControlCommand = timeout(Duration::from_millis(500), h.vcpu_cmd_rx.recv())
        .await?
        .expect("vcpu command channel closed unexpectedly");
    assert!(matches!(resume_cmd, VcpuControlCommand::Resume));
    assert!(h.resume_called.load(Ordering::SeqCst));

    // Now shut everything down.
    h.vcpu_resp_tx.send(VcpuControlResponse::Shutdown).await?;
    let _mem_cmd = timeout(Duration::from_millis(500), h.mem_cmd_rx.recv()).await?;
    let _shutdown_resp = recv_io_resp(&mut h.io_resp_rx).await; // Shutdown

    let res = handle.await.expect("join failed");
    assert!(res.is_ok());
    Ok(())
}
