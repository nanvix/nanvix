// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Loading of the Nanvix guest into VM memory.
//!
//! This replicates the layout produced by the Nanvix `uservm` standalone path:
//! a 32-bit x86 kernel ELF loaded by virtual address, an optional initial RAM
//! disk at a fixed address, an optional RAM filesystem at the top of guest RAM,
//! and the shared control / pvclock pages. The MicroVM contract constants come
//! from the shared `config` crate, so this loader stays in lock-step with the
//! guest kernel.

use ::anyhow::Context as _;
use ::guestmem::GuestMemory;

/// Guest physical page size (the Nanvix guest is a 32-bit x86 kernel).
const PAGE_SIZE: u64 = 0x1000;

/// Description of the guest images to load.
pub struct GuestImage {
    /// Path to the 32-bit kernel ELF.
    pub kernel: std::path::PathBuf,
    /// Optional path to the initial RAM disk (a single guest application ELF).
    pub initrd: Option<std::path::PathBuf>,
    /// Optional command-line arguments forwarded to the initrd application.
    pub initrd_args: Option<String>,
    /// Optional kernel arguments written to the control page.
    pub kernel_args: Option<String>,
    /// Optional path to a RAM filesystem image.
    pub ramfs: Option<std::path::PathBuf>,
    /// Size of guest RAM in bytes.
    pub mem_size: u64,
    /// Host TSC base frequency in MHz, written to the control page for the
    /// guest's LAPIC timer calibration.
    pub tsc_freq_mhz: u32,
}

/// State produced by loading the guest, needed to set the boot registers.
pub struct LoadedGuest {
    /// Kernel entry point (guest physical / virtual address; identity-mapped).
    pub entry: u64,
    /// Base address of the initrd, or 0 if none.
    pub initrd_base: u64,
    /// Size of the initrd in bytes (page-rounded), or 0 if none.
    pub initrd_size: u64,
}

/// Rounds `value` up to the next multiple of [`PAGE_SIZE`].
fn page_round_up(value: u64) -> u64 {
    (value + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Loads the kernel, initrd, RAMFS, and control/pvclock pages into guest memory.
pub fn load(gm: &GuestMemory, image: &GuestImage) -> anyhow::Result<LoadedGuest> {
    let entry = load_kernel(gm, &image.kernel)?;

    let (initrd_base, initrd_size) = match &image.initrd {
        Some(path) => load_initrd(gm, path, image.initrd_args.as_deref(), image.mem_size)?,
        None => (0, 0),
    };

    // The control and pvclock pages live inside the kernel ELF's zero-filled
    // low memory, so all VMM-written control values must be written *after* the
    // kernel has been loaded.
    if let Some(ramfs) = &image.ramfs {
        let (base, size) = load_ramfs(gm, ramfs, initrd_base + initrd_size, image.mem_size)?;
        write_u32(gm, config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_BASE as u64, base as u32)?;
        write_u32(gm, config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_SIZE as u64, size as u32)?;
    }

    // Credits start at zero (no host messages pending).
    write_u32(gm, config::microvm::DEFAULT_MICROVM_CTRL_CREDITS as u64, 0)?;
    // Running state (not paused).
    write_u32(
        gm,
        config::microvm::DEFAULT_MICROVM_CTRL_PAUSE_REQUESTED as u64,
        config::microvm::RUNNING,
    )?;
    // TSC frequency for RDTSC-based LAPIC timer calibration.
    write_u32(gm, config::microvm::DEFAULT_MICROVM_CTRL_TSC_FREQ_MHZ as u64, image.tsc_freq_mhz)?;

    if let Some(args) = &image.kernel_args {
        write_kernel_args(gm, args)?;
    }

    write_pvclock_boot_time(gm)?;

    Ok(LoadedGuest {
        entry,
        initrd_base,
        initrd_size,
    })
}

/// Loads a 32-bit ELF kernel into guest memory by virtual address.
///
/// A minimal hand-rolled ELF32 parser is used (rather than a general ELF crate)
/// to keep this utility free of extra dependencies: the Nanvix kernel is always
/// a static 32-bit little-endian executable with a small number of `PT_LOAD`
/// segments.
fn load_kernel(gm: &GuestMemory, path: &std::path::Path) -> anyhow::Result<u64> {
    /// `PT_LOAD` program-header type.
    const PT_LOAD: u32 = 1;
    /// Size of an ELF32 program-header entry.
    const PHDR_SIZE: usize = 32;

    let data = std::fs::read(path).context("failed to read kernel")?;
    if data.len() < 52 || &data[0..4] != b"\x7fELF" {
        anyhow::bail!("kernel is not an ELF file");
    }
    // ELFCLASS32, ELFDATA2LSB.
    if data[4] != 1 || data[5] != 1 {
        anyhow::bail!("kernel is not a 32-bit little-endian ELF");
    }

    let rd_u32 = |off: usize| -> u32 {
        u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
    };
    let rd_u16 = |off: usize| -> u16 { u16::from_le_bytes([data[off], data[off + 1]]) };

    let entry = rd_u32(24);
    let phoff = rd_u32(28) as usize;
    let phentsize = rd_u16(42) as usize;
    let phnum = rd_u16(44) as usize;
    if phentsize < PHDR_SIZE {
        anyhow::bail!("unexpected ELF program-header entry size {phentsize}");
    }

    for i in 0..phnum {
        let base = phoff + i * phentsize;
        if base + PHDR_SIZE > data.len() {
            anyhow::bail!("ELF program header {i} out of bounds");
        }
        if rd_u32(base) != PT_LOAD {
            continue;
        }
        let p_offset = rd_u32(base + 4) as usize;
        let p_vaddr = u64::from(rd_u32(base + 8));
        let p_filesz = rd_u32(base + 16) as usize;
        if p_filesz == 0 {
            continue;
        }
        let end = p_offset
            .checked_add(p_filesz)
            .filter(|&e| e <= data.len())
            .context("ELF segment file range out of bounds")?;
        gm.write_at(p_vaddr, &data[p_offset..end])
            .with_context(|| {
                format!("failed to write kernel segment at {p_vaddr:#x} ({p_filesz} bytes)")
            })?;
        // The trailing BSS (`p_memsz > p_filesz`) is left to the demand-zero
        // guest RAM mapping, matching the Nanvix uservm loader.
    }

    Ok(u64::from(entry))
}

/// Loads the initrd at the fixed base address and, for a single-binary initrd,
/// writes a length-prefixed command-line string immediately after it.
fn load_initrd(
    gm: &GuestMemory,
    path: &std::path::Path,
    initrd_args: Option<&str>,
    mem_size: u64,
) -> anyhow::Result<(u64, u64)> {
    let base = config::microvm::DEFAULT_INITRD_BASE as u64;
    let data = std::fs::read(path).context("failed to read initrd")?;
    let size = data.len() as u64;
    let size_rounded = page_round_up(size);

    let end = base + size_rounded;
    if end > mem_size {
        anyhow::bail!(
            "initrd does not fit in guest memory (initrd_end={end:#x}, mem_size={mem_size:#x})"
        );
    }

    gm.write_at(base, &data).context("failed to write initrd")?;

    // Build the command line: "<basename> <args>".
    let mut args = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    if let Some(extra) = initrd_args {
        args.push(' ');
        args.push_str(extra);
    }
    // The guest locates the command line at the page-rounded end of the initrd
    // region (it derives the region size in pages from the boot register), so
    // the length-prefixed string must be written there, not at the raw end.
    // The check above only bounds the initrd payload; ensure the trailing
    // command line (a `u16` length followed by its bytes) also fits in guest RAM.
    let cmdline_end = end + 2 + args.len() as u64;
    if cmdline_end > mem_size {
        anyhow::bail!(
            "initrd command line does not fit in guest memory (cmdline_end={cmdline_end:#x}, \
             mem_size={mem_size:#x})"
        );
    }
    write_initrd_args(gm, base + size_rounded, &args)?;

    Ok((base, size_rounded))
}

/// Writes a `u16` little-endian length followed by the argument bytes, just
/// past the end of the initrd region.
fn write_initrd_args(gm: &GuestMemory, initrd_end: u64, args: &str) -> anyhow::Result<()> {
    let bytes = args.as_bytes();
    if bytes.len() > config::system::MAX_CMDLINE_ARGS_LEN {
        anyhow::bail!(
            "initrd command line too long (len={}, max={})",
            bytes.len(),
            config::system::MAX_CMDLINE_ARGS_LEN
        );
    }
    let len = bytes.len() as u16;
    gm.write_at(initrd_end, &len.to_le_bytes())
        .context("failed to write initrd args length")?;
    gm.write_at(initrd_end + 2, bytes)
        .context("failed to write initrd args data")?;
    Ok(())
}

/// Loads a RAM filesystem image at the top of guest RAM and returns its base
/// and (page-rounded) size.
fn load_ramfs(
    gm: &GuestMemory,
    path: &std::path::Path,
    initrd_end: u64,
    mem_size: u64,
) -> anyhow::Result<(u64, u64)> {
    /// Minimum gap required between the end of the initrd and the RAMFS.
    const RAMFS_MIN_SLACK_BYTES: u64 = 4 * 1024 * 1024;

    let data = std::fs::read(path).context("failed to read ramfs")?;
    let size = page_round_up(data.len() as u64);

    if initrd_end + RAMFS_MIN_SLACK_BYTES > mem_size {
        anyhow::bail!(
            "guest memory ({mem_size:#x}) too small for initrd end plus slack ({:#x})",
            initrd_end + RAMFS_MIN_SLACK_BYTES
        );
    }
    let base = mem_size
        .checked_sub(size)
        .context("ramfs image does not fit in guest memory")?;
    if base < initrd_end {
        anyhow::bail!("ramfs would overlap the initrd region");
    }

    gm.write_at(base, &data).context("failed to write ramfs")?;
    Ok((base, size))
}

/// Writes the kernel-arguments length and data into the control page.
fn write_kernel_args(gm: &GuestMemory, args: &str) -> anyhow::Result<()> {
    let bytes = args.as_bytes();
    if bytes.len() > config::microvm::MAX_KERNEL_ARGS_LEN {
        anyhow::bail!(
            "kernel arguments too long (len={}, max={})",
            bytes.len(),
            config::microvm::MAX_KERNEL_ARGS_LEN
        );
    }
    let len = bytes.len() as u16;
    gm.write_at(config::microvm::DEFAULT_MICROVM_CTRL_KERNEL_ARGS_LEN as u64, &len.to_le_bytes())
        .context("failed to write kernel args length")?;
    if !bytes.is_empty() {
        gm.write_at(config::microvm::DEFAULT_MICROVM_CTRL_KERNEL_ARGS_DATA as u64, bytes)
            .context("failed to write kernel args data")?;
    }
    Ok(())
}

/// Writes the current wall-clock time (UTC nanoseconds since the Unix epoch)
/// into the pvclock page so the guest can derive wall-clock time.
fn write_pvclock_boot_time(gm: &GuestMemory) -> anyhow::Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let boot_time_ns = now.as_secs() * 1_000_000_000 + u64::from(now.subsec_nanos());
    let offset = (config::microvm::DEFAULT_PVCLOCK_PAGE
        + config::microvm::PVCLOCK_BOOT_TIME_NS_OFFSET) as u64;
    gm.write_at(offset, &boot_time_ns.to_le_bytes())
        .context("failed to write pvclock boot time")?;
    Ok(())
}

/// Writes a little-endian `u32` to the given guest physical address.
fn write_u32(gm: &GuestMemory, gpa: u64, value: u32) -> anyhow::Result<()> {
    gm.write_at(gpa, &value.to_le_bytes())
        .with_context(|| format!("failed to write u32 at {gpa:#x}"))
}
