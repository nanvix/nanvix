// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    time::Instant,
};
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    process::Command,
    time::Duration,
};
use ::vfs_bench_common::{
    ACK_OK,
    VfsOp,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Timeout (in seconds) for the VFS benchmark guest application.
const VFS_BENCH_TIMEOUT_SECS: u64 = 300;

/// Fixed seed for the host-side RNG that picks random files/dirs per iteration.
const HOST_RNG_SEED: u64 = 0xCAFE_BABE;

/// Decomposition rules for estimating individual operation costs via paired measurement.
///
/// Each entry is `(display_label, minuend_opcode, optional_subtrahend_opcode)`. For entries with
/// a subtrahend, the host runs both operations back-to-back in the same VM instance per iteration
/// (alternating order on even/odd iterations to cancel ordering bias) and stores
/// `t_minuend − t_subtrahend`. Percentiles are computed over these per-iteration deltas, avoiding
/// the error of subtracting aggregate percentiles from independent runs.
const DECOMPOSED_OPS: &[(&str, VfsOp, Option<VfsOp>)] = &[
    // Level 0 — raw latency (no subtrahend).
    ("protocol", VfsOp::Noop, None),
    // Level 1 — subtract noop.
    ("stat", VfsOp::Stat, Some(VfsOp::Noop)),
    ("open+close", VfsOp::OpenClose, Some(VfsOp::Noop)),
    ("readdir", VfsOp::Readdir, Some(VfsOp::Noop)),
    ("create_scratch", VfsOp::CreateScratch, Some(VfsOp::Noop)),
    // Level 2 — subtract a level-1 operation.
    ("seq_read", VfsOp::SeqRead, Some(VfsOp::OpenClose)),
    ("create+close+unlink", VfsOp::CreateUnlink, Some(VfsOp::CreateScratch)),
    ("mkdir+rmdir", VfsOp::MkdirRmdir, Some(VfsOp::CreateScratch)),
    ("create+close+rename", VfsOp::Rename, Some(VfsOp::CreateScratch)),
    // Level 3 — subtract a level-2 operation.
    ("write+flush", VfsOp::SeqWrite, Some(VfsOp::CreateUnlink)),
];

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
fn scan_fat_image(image_path: &Path) -> Result<FatManifest> {
    let file = std::fs::File::open(image_path)?;
    let storage = fatfs::StdIoWrapper::new(file);
    let fs = fatfs::FileSystem::new(storage, fatfs::FsOptions::new())
        .map_err(|e| anyhow::anyhow!("failed to open FAT image: {e}"))?;

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
// Implementations
//==================================================================================================

impl Benchmark {
    ///
    /// # Description
    ///
    /// Runs the standalone VFS benchmark. For each VFS operation, spawns a fresh nanvixd process
    /// in interactive mode with the VFS benchmark guest and a ramfs image. The host scans the FAT
    /// image to discover available files and directories, then drives each operation by sending
    /// `[opcode][path_len][path]` on stdin and reading a one-byte acknowledgement from stdout,
    /// measuring the round-trip latency with `Instant::now()`.
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

        let mut rng: StdRng = StdRng::seed_from_u64(HOST_RNG_SEED);

        // Progress bar: NOOP (1) + paired entries (those with a subtrahend).
        let paired_count: usize = DECOMPOSED_OPS
            .iter()
            .filter(|(_, _, sub)| sub.is_some())
            .count();
        let total_steps: u64 = ((1 + paired_count) * self.iterations) as u64;
        let pb: ProgressBar = ProgressBar::new(total_steps);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("VFS benchmark:");

        // Measure the NOOP (protocol overhead) first — it is the only operation without a
        // subtrahend and serves as the baseline for all decomposed measurements.
        let noop_latencies: Vec<u128>;
        {
            let (mut child, mut stdin, mut stdout) =
                Self::spawn_nanvixd(&nanvixd_bin, &ramfs, &program, &self.workspace_root)?;

            noop_latencies = Self::measure_iterations(
                &mut stdin,
                &mut stdout,
                VfsOp::Noop,
                &manifest,
                &mut rng,
                self.iterations,
                &pb,
            )
            .await?;

            Self::teardown_nanvixd(&mut child, stdin).await?;
        }

        // Paired decomposition (one VM per pair, back-to-back measurement).
        let mut paired_deltas: Vec<(&str, Vec<u128>)> = Vec::with_capacity(DECOMPOSED_OPS.len());

        for &(label, minuend, subtrahend) in DECOMPOSED_OPS {
            let sub_op: VfsOp = match subtrahend {
                Some(op) => op,
                None => {
                    // No subtrahend (NOOP) — use the pre-measured baseline.
                    paired_deltas.push((label, noop_latencies.clone()));
                    continue;
                },
            };

            let (mut child, mut stdin, mut stdout) =
                Self::spawn_nanvixd(&nanvixd_bin, &ramfs, &program, &self.workspace_root)?;

            let deltas: Vec<u128> = Self::measure_paired_iterations(
                &mut stdin,
                &mut stdout,
                minuend,
                sub_op,
                &manifest,
                &mut rng,
                self.iterations,
                &pb,
            )
            .await?;
            paired_deltas.push((label, deltas));

            Self::teardown_nanvixd(&mut child, stdin).await?;
        }

        pb.finish();

        // Print decomposed latency table.
        Self::print_latency_table("Decomposed (paired)", &mut paired_deltas);

        Ok(())
    }

    /// Picks a random file or directory path appropriate for the given operation.
    ///
    /// Read-only file operations (stat, open/close, seq_read) get a random file path.
    /// Directory operations (readdir) get a random directory path.
    /// Scratch-based operations (write, mkdir, create, rename) return an empty path.
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
            | VfsOp::CreateScratch
            | VfsOp::CreateUnlink
            | VfsOp::MkdirRmdir
            | VfsOp::Rename
            | VfsOp::SeqWrite => "",
        }
    }

    /// Runs `count` iterations of a single VFS operation, returning per-iteration latencies.
    async fn measure_iterations(
        stdin: &mut ::tokio::process::ChildStdin,
        stdout: &mut ::tokio::process::ChildStdout,
        opcode: VfsOp,
        manifest: &FatManifest,
        rng: &mut StdRng,
        count: usize,
        pb: &ProgressBar,
    ) -> Result<Vec<u128>> {
        let mut latencies: Vec<u128> = Vec::with_capacity(count);
        for _ in 0..count {
            let path: &str = Self::pick_path(opcode, manifest, rng);
            latencies.push(Self::timed_send_op(stdin, stdout, opcode.as_u8(), path).await?);
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
    #[allow(clippy::too_many_arguments)]
    async fn measure_paired_iterations(
        stdin: &mut ::tokio::process::ChildStdin,
        stdout: &mut ::tokio::process::ChildStdout,
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
                let t_sub: u128 =
                    Self::timed_send_op(stdin, stdout, subtrahend.as_u8(), sub_path).await?;
                let t_min: u128 =
                    Self::timed_send_op(stdin, stdout, minuend.as_u8(), min_path).await?;
                (t_min, t_sub)
            } else {
                // Minuend first.
                let t_min: u128 =
                    Self::timed_send_op(stdin, stdout, minuend.as_u8(), min_path).await?;
                let t_sub: u128 =
                    Self::timed_send_op(stdin, stdout, subtrahend.as_u8(), sub_path).await?;
                (t_min, t_sub)
            };

            deltas.push(t_min.saturating_sub(t_sub));
            pb.inc(1);
        }
        Ok(deltas)
    }

    /// Sends a single operation and returns its latency in microseconds.
    async fn timed_send_op(
        stdin: &mut ::tokio::process::ChildStdin,
        stdout: &mut ::tokio::process::ChildStdout,
        opcode: u8,
        path: &str,
    ) -> Result<u128> {
        let start: Instant = Instant::now();
        ::tokio::time::timeout(
            Duration::from_secs(VFS_BENCH_TIMEOUT_SECS),
            Self::send_op(stdin, stdout, opcode, path),
        )
        .await
        .map_err(|_| {
            anyhow::anyhow!("VFS benchmark timed out after {VFS_BENCH_TIMEOUT_SECS}s")
        })??;
        Ok(start.elapsed().as_micros())
    }

    /// Prints a percentile latency table (p50, p95, p99) with the given header.
    fn print_latency_table(header: &str, data: &mut [(&str, Vec<u128>)]) {
        println!(
            "\n{:<22} {:>8} {:>10} {:>10} {:>10}",
            header, "Samples", "p50 (us)", "p95 (us)", "p99 (us)"
        );
        println!("{}", "-".repeat(62));

        for (name, latencies) in data.iter_mut() {
            latencies.sort();
            let len: usize = latencies.len();
            let p50: u128 = latencies[((len as f64 * 0.5) as usize).min(len - 1)];
            let p95: u128 = latencies[((len as f64 * 0.95) as usize).min(len - 1)];
            let p99: u128 = latencies[((len as f64 * 0.99) as usize).min(len - 1)];
            println!("{:<22} {:>8} {:>10} {:>10} {:>10}", name, len, p50, p95, p99);
        }
    }

    /// Spawns a nanvixd process in interactive mode and returns the child, stdin, and stdout.
    fn spawn_nanvixd(
        nanvixd_bin: &Path,
        ramfs: &str,
        program: &str,
        workspace_root: &Path,
    ) -> Result<(
        ::tokio::process::Child,
        ::tokio::process::ChildStdin,
        ::tokio::process::ChildStdout,
    )> {
        let mut cmd: Command = Command::new(nanvixd_bin);
        cmd.arg(::nanvixd::args::Args::OPT_RAMFS_FILENAME)
            .arg(ramfs)
            .arg(::nanvixd::args::Args::OPT_SEPARATOR)
            .arg(program)
            .stdin(::std::process::Stdio::piped())
            .stdout(::std::process::Stdio::piped())
            .stderr(::std::process::Stdio::piped())
            .current_dir(workspace_root)
            .kill_on_drop(true);

        let mut child: ::tokio::process::Child = cmd.spawn()?;

        let stdin: ::tokio::process::ChildStdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdin"))?;
        let stdout: ::tokio::process::ChildStdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stdout"))?;

        Ok((child, stdin, stdout))
    }

    /// Tears down a nanvixd process by closing stdin and waiting for exit.
    async fn teardown_nanvixd(
        child: &mut ::tokio::process::Child,
        stdin: ::tokio::process::ChildStdin,
    ) -> Result<()> {
        // Close stdin to signal the guest to exit.
        drop(stdin);

        // Read any stderr output for diagnostics before waiting for exit.
        let stderr_output: String = {
            let mut stderr: ::tokio::process::ChildStderr = child
                .stderr
                .take()
                .ok_or_else(|| anyhow::anyhow!("failed to take nanvixd stderr"))?;
            let mut buf: Vec<u8> = Vec::new();
            let _ = stderr.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        };

        let status: ::std::process::ExitStatus = child.wait().await?;
        if !status.success() {
            if !stderr_output.is_empty() {
                error!("nanvixd stderr:\n{stderr_output}");
            }
            anyhow::bail!("nanvixd exited with {status}");
        }

        Ok(())
    }

    /// Sends a single opcode and path to the guest and waits for the one-byte acknowledgement.
    ///
    /// Wire format: `[opcode: u8][path_len: u8][path: path_len bytes]` → `[ack: u8]`.
    async fn send_op(
        stdin: &mut ::tokio::process::ChildStdin,
        stdout: &mut ::tokio::process::ChildStdout,
        opcode: u8,
        path: &str,
    ) -> Result<()> {
        let path_bytes: &[u8] = path.as_bytes();
        let path_len: u8 =
            u8::try_from(path_bytes.len()).map_err(|_| anyhow::anyhow!("path too long"))?;
        stdin.write_all(&[opcode, path_len]).await?;
        if path_len > 0 {
            stdin.write_all(path_bytes).await?;
        }
        stdin.flush().await?;
        let mut ack: [u8; 1] = [0u8; 1];
        stdout.read_exact(&mut ack).await?;
        if ack[0] != ACK_OK {
            anyhow::bail!("guest reported error for opcode {opcode:#04x} path={path:?}");
        }
        Ok(())
    }
}
