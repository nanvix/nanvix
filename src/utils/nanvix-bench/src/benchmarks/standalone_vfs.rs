// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Standalone deployment of the `vfs-bench` benchmark.
//!
//! Boots `nanvixd` with a regular standalone image and drives the `vfs-bench-nostd` guest over the
//! console. The guest is an ordinary process: each request asks it to run one file system
//! operation through the system call interface, so every measured operation crosses the kernel and
//! `vfsd`, which is the sole owner of the benchmark image.
//!
//! The benchmark has two phases that differ in how `vfsd` mounts the image at `/`: writable, in
//! which all operations are measured, and read-only, in which only the operations that do not
//! modify the file system are measured. The host picks the mount mode of each VM by wrapping the
//! image in a multi-image container, whose `ROOTFS` entry carries the read-only flag or not, and
//! booting `nanvixd` with that container as its ramfs.

//==================================================================================================
// Imports
//==================================================================================================

use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::log::error;
use ::multiimage::{
    FLAG_READONLY,
    ImageDescriptor,
    ROOTFS_MMIO_TAG,
};
use ::rand::{
    RngExt,
    SeedableRng,
    rngs::StdRng,
};
use ::std::{
    path::{
        Path,
        PathBuf,
    },
    process::{
        ExitStatus,
        Stdio,
    },
    sync::{
        Arc,
        Mutex,
        PoisonError,
    },
    time::Instant,
};
use ::tempfile::TempDir;
use ::tokio::{
    io::{
        AsyncRead,
        AsyncReadExt,
        AsyncWriteExt,
    },
    process::{
        Child,
        ChildStderr,
        ChildStdin,
        ChildStdout,
        Command,
    },
    task::JoinHandle,
    time::{
        Duration,
        timeout,
    },
};
use ::vfs_bench_common::{
    ACK_OK,
    ACK_READ_ONLY,
    MAX_PATH_LEN,
    SCRATCH_DIR,
    VfsOp,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum time to wait for the guest to boot and report that it is ready, and for the
/// acknowledgement of each request.
const VFS_BENCH_TIMEOUT: Duration = Duration::from_secs(300);

/// Maximum time to wait for `nanvixd` to exit once the guest console is closed.
const TEARDOWN_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum time to wait for the standard error stream of `nanvixd` to end once it exits.
const STDERR_TIMEOUT: Duration = Duration::from_secs(5);

/// Size of the chunks in which the standard error stream of `nanvixd` is read.
const STDERR_CHUNK_SIZE: usize = 4096;

/// Name of the multi-image container that wraps the benchmark image.
const CONTAINER_FILENAME: &str = "vfs-bench-container.img";

/// Fixed seed for the host-side RNG that picks random files/dirs per iteration.
const HOST_RNG_SEED: u64 = 0xCAFE_BABE;

/// Decomposition rules for estimating individual operation costs via paired measurement.
///
/// Each entry is `(display_label, minuend_opcode, optional_subtrahend_opcode)`. For entries with
/// a subtrahend, the host runs both operations back-to-back in the same VM instance per iteration
/// (alternating order on even/odd iterations to cancel ordering bias) and stores
/// `t_minuend − t_subtrahend`. Percentiles are computed over these per-iteration deltas, avoiding
/// the error of subtracting aggregate percentiles from independent runs.
///
/// The write operations run in the scratch directory of the image: each one creates a file or a
/// directory and removes it again, so that the file system is back to its initial state once the
/// operation completes. No system call creates a file and leaves it for the caller to remove, so
/// the creation cannot be isolated and is reported together with the removal that undoes it.
/// `write+flush` also includes the cost of releasing the written data when the file is removed.
const DECOMPOSED_OPS: &[(&str, VfsOp, Option<VfsOp>)] = &[
    // Level 0 — raw latency (no subtrahend).
    ("protocol", VfsOp::Noop, None),
    // Level 1 — subtract noop.
    ("stat", VfsOp::Stat, Some(VfsOp::Noop)),
    ("open+close", VfsOp::OpenClose, Some(VfsOp::Noop)),
    ("readdir", VfsOp::Readdir, Some(VfsOp::Noop)),
    ("create+close+unlink", VfsOp::CreateUnlink, Some(VfsOp::Noop)),
    ("mkdir+rmdir", VfsOp::MkdirRmdir, Some(VfsOp::Noop)),
    // Level 2 — subtract a level-1 operation.
    ("seq_read", VfsOp::SeqRead, Some(VfsOp::OpenClose)),
    ("rename", VfsOp::Rename, Some(VfsOp::CreateUnlink)),
    ("write+flush", VfsOp::SeqWrite, Some(VfsOp::CreateUnlink)),
];

/// Per-phase timings that `nanvixd` reports on its standard error stream.
type PhaseTimings = serde_json::Map<String, serde_json::Value>;

/// Returns the time, in microseconds, that `nanvixd` took to load the ramfs, if it reported it.
fn ramfs_load_us(timings: Option<PhaseTimings>) -> Option<u128> {
    timings?.get("ramfs_load_us")?.as_u64().map(u128::from)
}

/// Returns `true` if the given operation mutates the filesystem.
fn is_write_op(op: VfsOp) -> bool {
    matches!(op, VfsOp::CreateUnlink | VfsOp::MkdirRmdir | VfsOp::Rename | VfsOp::SeqWrite)
}

/// Returns the decomposition rules that apply to a mount mode.
///
/// A read-only mount rejects the operations that modify the file system, so it leaves out every
/// rule that involves one of them.
fn decomposed_ops(readonly: bool) -> Vec<(&'static str, VfsOp, Option<VfsOp>)> {
    DECOMPOSED_OPS
        .iter()
        .copied()
        .filter(|&(_, minuend, subtrahend)| {
            !readonly || (!is_write_op(minuend) && subtrahend.is_none_or(|op| !is_write_op(op)))
        })
        .collect()
}

/// Checks that the acknowledgement of a write operation shows the requested mount mode.
///
/// A write operation must succeed on a writable mount and be rejected as read-only on a read-only
/// one. Any other failure says nothing about the mount mode, so it does not pass for a rejection:
/// otherwise, an unrelated error could let a writable mount go by as a read-only one.
fn verify_mount_mode(readonly: bool, ack: u8) -> Result<()> {
    match (readonly, ack) {
        (false, ACK_OK) | (true, ACK_READ_ONLY) => Ok(()),
        (false, ACK_READ_ONLY) => {
            anyhow::bail!("guest could not write to the image, which should be writable")
        },
        (true, ACK_OK) => {
            anyhow::bail!("guest could write to the image, which should be read-only")
        },
        (_, ack) => {
            let mode: &str = if readonly { "read-only" } else { "writable" };
            anyhow::bail!("guest failed to check that the image is {mode} (ack={ack:#04x})")
        },
    }
}

//==================================================================================================
// FAT Image Manifest
//==================================================================================================

/// Lists of file and directory paths discovered by scanning a FAT image.
struct FatManifest {
    /// Absolute paths of all regular files (e.g. `"/d0/f0.dat"`).
    files: Vec<String>,
    /// Size in bytes of each file (same order as `files`).
    file_sizes: Vec<u64>,
    /// Absolute paths of all directories, including `"/"` (e.g. `"/d0"`).
    dirs: Vec<String>,
}

impl FatManifest {
    /// Returns the maximum directory depth (root = 0).
    fn max_depth(&self) -> usize {
        self.dirs
            .iter()
            .map(|p| p.matches('/').count().saturating_sub(1))
            .max()
            .unwrap_or(0)
    }

    /// Returns the smallest file size, or 0 if there are no files.
    fn min_file_size(&self) -> u64 {
        self.file_sizes.iter().copied().min().unwrap_or(0)
    }

    /// Returns the largest file size, or 0 if there are no files.
    fn max_file_size(&self) -> u64 {
        self.file_sizes.iter().copied().max().unwrap_or(0)
    }
}

/// Scans a FAT image on disk and returns the manifest of all files and directories.
///
/// The scratch directory is left out: the write operations work in it, so its contents change
/// while they run and the read operations must not pick it.
fn scan_fat_image(image_path: &Path) -> Result<FatManifest> {
    let file = std::fs::File::open(image_path)?;
    let storage = fatfs::StdIoWrapper::new(file);
    let fs = fatfs::FileSystem::new(storage, fatfs::FsOptions::new())
        .map_err(|e| anyhow::anyhow!("failed to open FAT image: {e}"))?;

    let scratch_name: &str = SCRATCH_DIR.trim_start_matches('/');

    let mut files: Vec<String> = Vec::new();
    let mut file_sizes: Vec<u64> = Vec::new();
    let mut dirs: Vec<String> = vec!["/".to_string()];

    // Walk the directory tree with a stack of (Dir, absolute_path) pairs.
    let mut stack: Vec<String> = vec![String::new()];
    while let Some(dir_path) = stack.pop() {
        let dir = if dir_path.is_empty() {
            fs.root_dir()
        } else {
            fs.root_dir()
                .open_dir(&dir_path)
                .map_err(|e| anyhow::anyhow!("failed to open {dir_path}: {e}"))?
        };

        for entry in dir.iter() {
            let entry = entry.map_err(|e| anyhow::anyhow!("failed to read entry: {e}"))?;
            let name: String = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            if dir_path.is_empty() && name == scratch_name {
                continue;
            }

            let abs_path: String = if dir_path.is_empty() {
                format!("/{name}")
            } else {
                format!("/{dir_path}/{name}")
            };

            if entry.is_dir() {
                dirs.push(abs_path.clone());
                let rel: String = if dir_path.is_empty() {
                    name
                } else {
                    format!("{dir_path}/{name}")
                };
                stack.push(rel);
            } else {
                files.push(abs_path);
                file_sizes.push(entry.len());
            }
        }
    }

    Ok(FatManifest {
        files,
        file_sizes,
        dirs,
    })
}

//==================================================================================================
// Root Container
//==================================================================================================

/// Wraps the benchmark image in a multi-image container and writes it to `output`.
///
/// The image is the `ROOTFS` entry of the container, which `vfsd` mounts at `/`. The entry carries
/// the read-only flag when `readonly` is set, so this is how the host picks the mount mode of the
/// VM that boots with the container.
fn build_root_container(image: &Path, readonly: bool, output: &Path) -> Result<()> {
    let flags: u32 = if readonly { FLAG_READONLY } else { 0 };
    let descriptors: [ImageDescriptor<'_>; 1] = [ImageDescriptor {
        tag: ROOTFS_MMIO_TAG,
        path: image,
        flags,
    }];

    ::multiimage::build_multiimage(&descriptors, output).map_err(|error| {
        anyhow::anyhow!(
            "failed to wrap {} in a multi-image container at {}: {error}",
            image.display(),
            output.display()
        )
    })
}

//==================================================================================================
// Virtual Machine
//==================================================================================================

/// Collector of the standard error stream of `nanvixd`.
///
/// A background task drains the stream into a buffer that the collector shares with it, so that
/// what the task read survives its cancellation.
struct StderrCollector {
    /// What the task read so far.
    output: Arc<Mutex<Vec<u8>>>,
    /// Task that reads the stream until it ends.
    task: JoinHandle<()>,
}

impl StderrCollector {
    /// Starts to collect `stream` in the background, so that `nanvixd` never blocks on a full pipe
    /// while the benchmark runs.
    fn spawn<R>(mut stream: R) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        let output: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let shared: Arc<Mutex<Vec<u8>>> = Arc::clone(&output);
        let task: JoinHandle<()> = ::tokio::spawn(async move {
            let mut chunk: [u8; STDERR_CHUNK_SIZE] = [0; STDERR_CHUNK_SIZE];
            loop {
                match stream.read(&mut chunk).await {
                    Ok(0) | Err(_) => break,
                    Ok(size) => shared
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .extend_from_slice(&chunk[..size]),
                }
            }
        });

        Self { output, task }
    }

    /// Waits for the stream to end and returns what was read from it.
    ///
    /// If the stream does not end within `limit`, the task is cancelled, because dropping its
    /// handle would only detach it, which would leave it holding the pipe for as long as the stream
    /// stays open. What the task read until then is returned all the same.
    async fn collect(mut self, limit: Duration) -> String {
        if timeout(limit, &mut self.task).await.is_err() {
            self.task.abort();
            let _ = (&mut self.task).await;
        }

        let bytes: Vec<u8> = self
            .output
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();

        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// A `nanvixd` process that boots the `vfs-bench-nostd` guest.
///
/// The process is killed when the value is dropped, so that no VM outlives an early return.
struct VfsBenchVm {
    /// The `nanvixd` process.
    child: Child,
    /// Console input of the guest.
    stdin: ChildStdin,
    /// Console output of the guest.
    stdout: ChildStdout,
    /// Collector of what `nanvixd` writes to its standard error stream.
    stderr: StderrCollector,
}

impl VfsBenchVm {
    /// Boots the guest `program` on a fresh `nanvixd` process that serves `ramfs`, and waits for
    /// the guest to report that it is ready.
    async fn boot(
        nanvixd_bin: &Path,
        ramfs: &Path,
        program: &str,
        workspace_root: &Path,
    ) -> Result<Self> {
        let mut child: Child = Command::new(nanvixd_bin)
            .arg(::nanvixd::args::Args::OPT_RAMFS_FILENAME)
            .arg(ramfs)
            .arg(::nanvixd::args::Args::OPT_SEPARATOR)
            .arg(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(workspace_root)
            .kill_on_drop(true)
            .spawn()?;

        let stdin: ChildStdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdin"))?;
        let stdout: ChildStdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdout"))?;
        let stderr: ChildStderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stderr"))?;

        let mut vm: Self = Self {
            child,
            stdin,
            stdout,
            stderr: StderrCollector::spawn(stderr),
        };
        if let Err(error) = vm.wait_until_ready().await {
            vm.abort().await;
            return Err(error);
        }

        Ok(vm)
    }

    /// Waits for the guest to report that it is ready to serve requests.
    async fn wait_until_ready(&mut self) -> Result<()> {
        let mut ack: [u8; 1] = [0; 1];
        timeout(VFS_BENCH_TIMEOUT, self.stdout.read_exact(&mut ack))
            .await
            .map_err(|_| {
                anyhow::anyhow!("guest was not ready after {}s", VFS_BENCH_TIMEOUT.as_secs())
            })?
            .map_err(|error| anyhow::anyhow!("failed to read guest readiness: {error}"))?;

        if ack[0] != ACK_OK {
            anyhow::bail!("guest failed to set up the benchmark image");
        }

        Ok(())
    }

    /// Sends one request to the guest and returns its acknowledgement and its round-trip time.
    ///
    /// Wire format: `[opcode: u8][path_len: u8][path: path_len bytes]` → `[ack: u8]`.
    async fn exchange(&mut self, op: VfsOp, path: &str) -> Result<(u8, Duration)> {
        let path_bytes: &[u8] = path.as_bytes();
        let path_len: u8 = u8::try_from(path_bytes.len()).map_err(|_| {
            anyhow::anyhow!("path {path:?} exceeds the protocol limit of {MAX_PATH_LEN} bytes")
        })?;

        let mut ack: [u8; 1] = [0; 1];
        let start: Instant = Instant::now();
        timeout(VFS_BENCH_TIMEOUT, async {
            self.stdin.write_all(&[op.as_u8(), path_len]).await?;
            self.stdin.write_all(path_bytes).await?;
            self.stdin.flush().await?;
            self.stdout.read_exact(&mut ack).await
        })
        .await
        .map_err(|_| {
            anyhow::anyhow!("timed out after {}s waiting for {op:?}", VFS_BENCH_TIMEOUT.as_secs())
        })?
        .map_err(|error| anyhow::anyhow!("failed to complete {op:?} (path={path:?}): {error}"))?;

        Ok((ack[0], start.elapsed()))
    }

    /// Runs one operation on the guest and returns its latency in microseconds.
    async fn run_op(&mut self, op: VfsOp, path: &str) -> Result<u128> {
        let (ack, elapsed): (u8, Duration) = self.exchange(op, path).await?;
        if ack != ACK_OK {
            anyhow::bail!("guest failed to run {op:?} (path={path:?}, ack={ack:#04x})");
        }

        Ok(elapsed.as_micros())
    }

    /// Checks that `vfsd` mounted the image with the requested mode.
    ///
    /// Runs a write operation, which must succeed on a writable mount and be rejected as
    /// read-only on a read-only one. A wrong mode would silently skew the results of the phase.
    async fn check_mount_mode(&mut self, readonly: bool) -> Result<()> {
        let (ack, _): (u8, Duration) = self.exchange(VfsOp::CreateUnlink, "").await?;

        verify_mount_mode(readonly, ack)
    }

    /// Tears the VM down and combines the outcome with the one of the measurement that ran on it.
    ///
    /// The VM is torn down whatever the outcome of the measurement. On success, returns the value
    /// of the measurement and the timings that `nanvixd` reported, if any.
    async fn finish<T>(self, result: Result<T>) -> Result<(T, Option<PhaseTimings>)> {
        let teardown: Result<Option<PhaseTimings>> = self.shutdown(result.is_err()).await;

        match (result, teardown) {
            (Ok(value), Ok(timings)) => Ok((value, timings)),
            (Err(error), Ok(_)) | (Ok(_), Err(error)) => Err(error),
            (Err(error), Err(teardown_error)) => {
                anyhow::bail!("{error}; additionally failed to tear down nanvixd: {teardown_error}")
            },
        }
    }

    /// Closes the guest console, which makes the guest exit, and waits for `nanvixd` to exit.
    ///
    /// Logs what `nanvixd` wrote to its standard error stream if the teardown fails, or if
    /// `measurement_failed` is set: the guest keeps serving requests after one fails, so a failed
    /// measurement can still end in a clean exit, and that output is then the only trace of why.
    ///
    /// Returns the timings that `nanvixd` reported on its standard error stream, when the
    /// `profile-time` feature is enabled.
    async fn shutdown(self, measurement_failed: bool) -> Result<Option<PhaseTimings>> {
        let Self {
            mut child,
            stdin,
            mut stdout,
            stderr,
        } = self;

        drop(stdin);

        let exit: Result<ExitStatus> = match timeout(TEARDOWN_TIMEOUT, async {
            // Drain the console so that nanvixd never blocks on a full pipe while exiting.
            ::tokio::io::copy(&mut stdout, &mut ::tokio::io::sink()).await?;
            child.wait().await
        })
        .await
        {
            Ok(Ok(status)) => Ok(status),
            Ok(Err(error)) => Err(anyhow::anyhow!("failed to wait for nanvixd to exit: {error}")),
            Err(_) => Err(anyhow::anyhow!(
                "timed out after {}s waiting for nanvixd to exit",
                TEARDOWN_TIMEOUT.as_secs()
            )),
        };
        if exit.is_err() {
            let _ = child.kill().await;
        }
        let output: String = stderr.collect(STDERR_TIMEOUT).await;

        let outcome: Result<()> = exit.and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("nanvixd exited with {status}"))
            }
        });
        if (measurement_failed || outcome.is_err()) && !output.is_empty() {
            error!("nanvixd stderr:\n{output}");
        }
        outcome?;

        #[cfg(feature = "profile-time")]
        let timings: Option<PhaseTimings> =
            super::standalone::parse_perf_timings(output.as_bytes());
        #[cfg(not(feature = "profile-time"))]
        let timings: Option<PhaseTimings> = None;

        Ok(timings)
    }

    /// Kills the VM and logs what it wrote to its standard error stream.
    async fn abort(mut self) {
        let _ = self.child.kill().await;
        let output: String = self.stderr.collect(STDERR_TIMEOUT).await;
        if !output.is_empty() {
            error!("nanvixd stderr:\n{output}");
        }
    }
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Benchmark {
    ///
    /// # Description
    ///
    /// Runs the standalone VFS benchmark in two phases:
    ///
    /// 1. **Writable mount** — `vfsd` mounts the image writable, so write operations are
    ///    permitted.
    /// 2. **Read-only mount** — `vfsd` mounts the image read-only, so write operations are
    ///    rejected and left out.
    ///
    /// Each phase measures all applicable VFS operations via paired decomposition and prints a
    /// percentile latency table. The host controls the mount mode by wrapping the image in a
    /// multi-image container before booting each nanvixd process.
    ///
    pub async fn run_vfs_bench_standalone(&mut self) -> Result<()> {
        let nanvixd_bin: PathBuf = self.standalone_nanvixd_path();
        let program: String = self.flavour.get_program(&self.workspace_root);
        let ramfs: String = self
            .flavour
            .get_ramfs(&self.workspace_root)
            .ok_or_else(|| anyhow::anyhow!("vfs-bench requires a ramfs image"))?;

        if !nanvixd_bin.exists() {
            let reason: String = format!("nanvixd binary not found at {}", nanvixd_bin.display());
            error!("{reason}");
            anyhow::bail!(reason);
        }

        let ramfs_path: PathBuf = PathBuf::from(&ramfs);
        if !ramfs_path.exists() {
            let reason: String = format!("ramfs image not found at {}", ramfs_path.display());
            error!("{reason}");
            anyhow::bail!(reason);
        }

        let program_path: PathBuf = PathBuf::from(&program);
        if !program_path.exists() {
            let reason: String =
                format!("benchmark program not found at {}", program_path.display());
            error!("{reason}");
            anyhow::bail!(reason);
        }

        // Scan the FAT image to discover available files and directories.
        let manifest: FatManifest = scan_fat_image(&ramfs_path)?;
        if manifest.files.is_empty() {
            anyhow::bail!("FAT image contains no files");
        }
        if manifest.dirs.is_empty() {
            anyhow::bail!("FAT image contains no directories");
        }

        // Print filesystem statistics.
        println!("\nFAT image: {}", ramfs_path.display());
        println!("  Files:         {}", manifest.files.len());
        println!("  Directories:   {}", manifest.dirs.len());
        println!("  Max depth:     {}", manifest.max_depth());
        println!("  Min file size: {} bytes", manifest.min_file_size());
        println!("  Max file size: {} bytes", manifest.max_file_size());

        // Phase 1: writable mount.
        let mut writable_deltas: Vec<(&str, Vec<u128>)> = self
            .run_vfs_bench_section(
                false,
                "VFS (writable):",
                &manifest,
                &nanvixd_bin,
                &ramfs_path,
                &program,
            )
            .await?;
        super::print_latency_table("Writable mount", "us", &mut writable_deltas);

        // Phase 2: read-only mount.
        let mut readonly_deltas: Vec<(&str, Vec<u128>)> = self
            .run_vfs_bench_section(
                true,
                "VFS (readonly):",
                &manifest,
                &nanvixd_bin,
                &ramfs_path,
                &program,
            )
            .await?;
        super::print_latency_table("Read-only mount", "us", &mut readonly_deltas);

        Ok(())
    }

    /// Runs one full benchmark section (all applicable decomposed operations) with the given mount
    /// mode. Write operations are excluded when `readonly` is `true`.
    ///
    /// Each VM is booted fresh, with the image mounted in the requested mode by `vfsd`, and
    /// measured. Returns the per-operation paired deltas for printing.
    async fn run_vfs_bench_section(
        &self,
        readonly: bool,
        progress_label: &str,
        manifest: &FatManifest,
        nanvixd_bin: &Path,
        image: &Path,
        program: &str,
    ) -> Result<Vec<(&'static str, Vec<u128>)>> {
        // Wrap the image in a container that makes vfsd mount it in the requested mode. Every VM of
        // the section boots with the same container, which is removed once the section ends.
        let container_dir: TempDir = ::tempfile::tempdir()?;
        let container: PathBuf = container_dir.path().join(CONTAINER_FILENAME);
        build_root_container(image, readonly, &container)?;

        // Reseed RNG for reproducibility across sections.
        let mut rng: StdRng = StdRng::seed_from_u64(HOST_RNG_SEED);

        // Filter out write operations when running in read-only mode.
        let ops: Vec<(&'static str, VfsOp, Option<VfsOp>)> = decomposed_ops(readonly);

        // Progress bar: NOOP (1) + paired entries (those with a subtrahend).
        let paired_count: usize = ops.iter().filter(|(_, _, sub)| sub.is_some()).count();
        let total_steps: u64 = ((1 + paired_count) * self.iterations) as u64;
        let pb: ProgressBar = ProgressBar::new(total_steps);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message(progress_label.to_string());

        // Time that nanvixd took to load the ramfs in each VM. Only reported when profiling.
        let mut ramfs_load_samples: Vec<u128> = Vec::new();

        // Measure the NOOP (protocol overhead) first. The first VM of the section also checks the
        // mount mode, which is the same for all the VMs of the section.
        let noop_latencies: Vec<u128> = {
            let mut vm: VfsBenchVm =
                VfsBenchVm::boot(nanvixd_bin, &container, program, &self.workspace_root).await?;

            let measured: Result<Vec<u128>> = async {
                vm.check_mount_mode(readonly).await?;
                Self::measure_iterations(
                    &mut vm,
                    VfsOp::Noop,
                    manifest,
                    &mut rng,
                    self.iterations,
                    &pb,
                )
                .await
            }
            .await;

            let (latencies, timings): (Vec<u128>, Option<PhaseTimings>) =
                vm.finish(measured).await?;
            ramfs_load_samples.extend(ramfs_load_us(timings));
            latencies
        };

        // Paired decomposition (one VM per pair, back-to-back measurement).
        let mut paired_deltas: Vec<(&str, Vec<u128>)> = Vec::with_capacity(ops.len());

        for &(label, minuend, subtrahend) in &ops {
            let sub_op: VfsOp = match subtrahend {
                Some(op) => op,
                None => {
                    // No subtrahend (NOOP) — use the pre-measured baseline.
                    paired_deltas.push((label, noop_latencies.clone()));
                    continue;
                },
            };

            let mut vm: VfsBenchVm =
                VfsBenchVm::boot(nanvixd_bin, &container, program, &self.workspace_root).await?;

            let measured: Result<Vec<u128>> = Self::measure_paired_iterations(
                &mut vm,
                minuend,
                sub_op,
                manifest,
                &mut rng,
                self.iterations,
                &pb,
            )
            .await;

            let (deltas, timings): (Vec<u128>, Option<PhaseTimings>) = vm.finish(measured).await?;
            paired_deltas.push((label, deltas));
            ramfs_load_samples.extend(ramfs_load_us(timings));
        }

        pb.finish();

        // Print VM setup timing breakdown (RAMFS load) when profiling is enabled.
        if !ramfs_load_samples.is_empty() {
            let mut setup_data: Vec<(&str, Vec<u128>)> = vec![("ramfs_load", ramfs_load_samples)];
            super::print_latency_table("VM setup", "us", &mut setup_data);
        }

        Ok(paired_deltas)
    }

    /// Picks a random file or directory path appropriate for the given operation.
    ///
    /// Read-only file operations (stat, open/close, seq_read) get a random file path.
    /// Directory operations (readdir) get a random directory path.
    /// Write operations work in the scratch directory of the image and return an empty path.
    fn pick_path<'a>(opcode: VfsOp, manifest: &'a FatManifest, rng: &mut StdRng) -> &'a str {
        match opcode {
            VfsOp::Stat | VfsOp::OpenClose | VfsOp::SeqRead => {
                let idx: usize = rng.random_range(0..manifest.files.len());
                &manifest.files[idx]
            },
            VfsOp::Readdir => {
                let idx: usize = rng.random_range(0..manifest.dirs.len());
                &manifest.dirs[idx]
            },
            VfsOp::Noop
            | VfsOp::CreateUnlink
            | VfsOp::MkdirRmdir
            | VfsOp::Rename
            | VfsOp::SeqWrite => "",
        }
    }

    /// Runs `count` iterations of a single VFS operation, returning per-iteration latencies.
    async fn measure_iterations(
        vm: &mut VfsBenchVm,
        opcode: VfsOp,
        manifest: &FatManifest,
        rng: &mut StdRng,
        count: usize,
        pb: &ProgressBar,
    ) -> Result<Vec<u128>> {
        let mut latencies: Vec<u128> = Vec::with_capacity(count);
        for _ in 0..count {
            let path: &str = Self::pick_path(opcode, manifest, rng);
            latencies.push(vm.run_op(opcode, path).await?);
            pb.inc(1);
        }
        Ok(latencies)
    }

    /// Runs `count` iterations of paired (minuend, subtrahend) measurement in the same VM.
    ///
    /// On even iterations the subtrahend runs first; on odd iterations the minuend runs first.
    /// This alternation cancels systematic ordering bias (warm i-cache, branch predictor state).
    /// Each iteration stores `t_minuend − t_subtrahend` (saturating), so percentiles computed
    /// over the resulting vector reflect per-iteration paired differences.
    async fn measure_paired_iterations(
        vm: &mut VfsBenchVm,
        minuend: VfsOp,
        subtrahend: VfsOp,
        manifest: &FatManifest,
        rng: &mut StdRng,
        count: usize,
        pb: &ProgressBar,
    ) -> Result<Vec<u128>> {
        let mut deltas: Vec<u128> = Vec::with_capacity(count);
        for i in 0..count {
            let min_path: &str = Self::pick_path(minuend, manifest, rng);
            let sub_path: &str = Self::pick_path(subtrahend, manifest, rng);

            // Alternate execution order: on even iterations the subtrahend runs first,
            // on odd iterations the minuend runs first. This cancels systematic ordering
            // bias (warm i-cache, branch predictor, TLB) so that only the true latency
            // difference remains across the full sample.
            let (t_min, t_sub): (u128, u128) = if i % 2 == 0 {
                // Subtrahend first.
                let t_sub: u128 = vm.run_op(subtrahend, sub_path).await?;
                let t_min: u128 = vm.run_op(minuend, min_path).await?;
                (t_min, t_sub)
            } else {
                // Minuend first.
                let t_min: u128 = vm.run_op(minuend, min_path).await?;
                let t_sub: u128 = vm.run_op(subtrahend, sub_path).await?;
                (t_min, t_sub)
            };

            deltas.push(t_min.saturating_sub(t_sub));
            pb.inc(1);
        }
        Ok(deltas)
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests;
