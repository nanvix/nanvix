// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use ::std::{
    env,
    path::Path,
};

use rand::{
    rngs::StdRng,
    RngExt,
    SeedableRng,
};

//==================================================================================================
// Constants
//==================================================================================================

// The FAT image is generated in two passes:
//
// **Pass 1 — Random directory tree.** A DFS walk creates directories up to `MAX_TREE_DEPTH`
// levels deep. At each directory the generator creates `1..=MAX_DIRS_PER_LEVEL` subdirectories
// and `1..=MAX_FILES_PER_DIR` files. File sizes are drawn uniformly from
// `MIN_FILE_SIZE..=max_file_size`, where `max_file_size` is derived at runtime as
// `(image_size / TREE_BUDGET_DIVISOR / TARGET_TREE_FILES).clamp(MIN_FILE_SIZE, MAX_FILE_SIZE_CAP)`.
// The walk stops when the cumulative bytes written reach `image_size / TREE_BUDGET_DIVISOR` or
// the DFS stack is exhausted.
//
// **Pass 2 — Bulk fill.** Writes `BULK_FILE_SIZE` files in the root until the image is full.
//
// Tuning guide:
//  - Deeper trees / more dirs per level → more directories, longer paths for `readdir` and
//    `stat` benchmarks. Note: the DFS stack can grow up to `MAX_DIRS_PER_LEVEL^MAX_TREE_DEPTH`
//    entries in the worst case.
//  - More files per dir → denser directories, more entries for `readdir` benchmarks.
//  - Larger `TARGET_TREE_FILES` → smaller per-file ceiling, producing many small files.
//    Smaller values → fewer but larger files (up to `MAX_FILE_SIZE_CAP`).
//  - Larger `TREE_BUDGET_DIVISOR` → less capacity reserved for the tree (more goes to bulk).
//  - `MIN_FILE_SIZE` sets a hard floor on tree file sizes; `MAX_FILE_SIZE_CAP` sets the ceiling.
//  - `BULK_FILE_SIZE` controls the granularity of the fill pass; only affects total file count
//    and the `readdir_root` benchmark.
//  - `RNG_SEED` ensures deterministic generation; changing it produces a different tree layout
//    with the same statistical properties.

/// Maximum number of subdirectories created at each level.
/// Actual count per directory is drawn from `1..=MAX_DIRS_PER_LEVEL`, so every visited directory
/// gets at least one child. Higher values create wider trees (more total directories) but the DFS
/// stack can grow up to `MAX_DIRS_PER_LEVEL ^ MAX_TREE_DEPTH` entries.
const MAX_DIRS_PER_LEVEL: usize = 6;

/// Maximum depth of the directory tree (0 = root only).
/// Directories at depth `MAX_TREE_DEPTH` do not create further subdirectories. Increasing this
/// produces longer absolute paths and deeper nesting, which stresses path-resolution and recursive
/// `readdir` performance.
const MAX_TREE_DEPTH: usize = 8;

/// Maximum number of files created in each directory.
/// Actual count per directory is drawn from `1..=MAX_FILES_PER_DIR`. Higher values produce denser
/// directories, which is useful for `readdir` and `create_unlink` benchmarks.
const MAX_FILES_PER_DIR: usize = 8;

/// Fraction of the total image capacity reserved for the random directory tree (pass 1).
/// A value of 4 means 25% goes to the tree and the remaining 75% is filled by bulk files.
/// Smaller values allocate more space to the tree, allowing more or larger tree files.
const TREE_BUDGET_DIVISOR: usize = 4;

/// Hard lower bound for per-file size in the tree (bytes).
/// No tree file will be smaller than this. Set to a small value to test operations on tiny files.
const MIN_FILE_SIZE: usize = 8;

/// Hard upper bound for per-file size in the tree (bytes).
/// The runtime ceiling `max_file_size` is clamped to this value regardless of how large the image
/// is. Prevents individual tree files from consuming a disproportionate share of the tree budget.
const MAX_FILE_SIZE_CAP: usize = 512 * 1024;

/// Target number of tree files used to derive the per-file size ceiling.
/// The runtime ceiling is `(tree_budget / TARGET_TREE_FILES).clamp(MIN_FILE_SIZE, MAX_FILE_SIZE_CAP)`.
/// Larger values produce more, smaller files; smaller values produce fewer, larger files.
const TARGET_TREE_FILES: usize = 4096;

/// Size of each bulk-fill file written in pass 2 (bytes).
/// Affects how many large files appear in the root directory and how quickly the image reaches
/// full capacity. Only the root-level `readdir_large` benchmark is sensitive to this count.
const BULK_FILE_SIZE: usize = 1024 * 1024;

/// Fixed seed for the PRNG that drives directory/file creation.
/// Changing this value produces a structurally different tree with the same statistical properties.
const RNG_SEED: u64 = 0xDEAD_BEEF;

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    //==============================================================================================
    // Link Archive
    //==============================================================================================

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    //==============================================================================================
    // Generate Dense FAT Image
    //==============================================================================================

    let out_dir: String = env::var("OUT_DIR").expect("OUT_DIR not set");
    let img_path = Path::new(&out_dir).join(vfs_bench_common::VFS_BENCH_IMG);

    generate_dense_fat_image(&img_path, build_utils::memory_size() / 2);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=MEMORY_SIZE_BYTES");
}

//==================================================================================================
// FAT Image Generation
//==================================================================================================

/// Generates a FAT filesystem image populated with a random directory tree and bulk padding.
///
/// The tree parameters scale dynamically with `image_size`:
///
/// **Pass 1 — Tree structure:** reserves `image_size /` [`TREE_BUDGET_DIVISOR`] bytes for a
/// random directory tree. Per-file sizes are derived from the budget and [`TARGET_TREE_FILES`],
/// clamped to \[[`MIN_FILE_SIZE`], [`MAX_FILE_SIZE_CAP`]\]. This keeps the total number of
/// filesystem operations roughly constant regardless of image size, so the pass completes quickly.
///
/// **Pass 2 — Bulk fill:** writes [`BULK_FILE_SIZE`] padding files in the root directory until
/// the image is full, filling the remaining capacity with very few operations.
fn generate_dense_fat_image(path: &Path, image_size: usize) {
    // Derive tree parameters from the target image size.
    let tree_budget: usize = image_size / TREE_BUDGET_DIVISOR;
    let max_file_size: usize =
        (tree_budget / TARGET_TREE_FILES).clamp(MIN_FILE_SIZE, MAX_FILE_SIZE_CAP);

    // Create and format a zeroed FAT image.
    mkramfs::mkfatfs(path, image_size).expect("failed to create/format FAT image");

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("failed to open FAT image for writing");
    let storage = fatfs::StdIoWrapper::new(file);
    let fs = fatfs::FileSystem::new(storage, fatfs::FsOptions::new())
        .expect("failed to open FAT filesystem");

    let root: fatfs::Dir<
        '_,
        fatfs::StdIoWrapper<std::fs::File>,
        fatfs::NullTimeProvider,
        fatfs::LossyOemCpConverter,
    > = fs.root_dir();

    let mut rng = StdRng::seed_from_u64(RNG_SEED);

    //==============================================================================================
    // Pass 1 — Build a random directory tree with files sized for this image.
    //==============================================================================================

    let mut stack: Vec<(String, usize)> = vec![(String::new(), 0)];
    let mut bytes_written: usize = 0;

    'tree: while let Some((dir_path, depth)) = stack.pop() {
        if bytes_written >= tree_budget {
            break;
        }

        let dir: fatfs::Dir<
            '_,
            fatfs::StdIoWrapper<std::fs::File>,
            fatfs::NullTimeProvider,
            fatfs::LossyOemCpConverter,
        > = if dir_path.is_empty() {
            fs.root_dir()
        } else {
            match root.open_dir(&dir_path) {
                Ok(d) => d,
                Err(_) => break,
            }
        };

        let files_per_dir: usize = rng.random_range(1..=MAX_FILES_PER_DIR);
        for i in 0..files_per_dir {
            if bytes_written >= tree_budget {
                break 'tree;
            }
            let size: usize = rng.random_range(MIN_FILE_SIZE..=max_file_size);
            let name: String = format!("f{i}.dat");
            let Ok(mut file) = dir.create_file(&name) else {
                break 'tree;
            };
            let pattern: u8 = rng.random_range(0..=u8::MAX);
            let data: Vec<u8> = vec![pattern; size];
            if fatfs::Write::write_all(&mut file, &data).is_err() {
                break 'tree;
            }
            if fatfs::Write::flush(&mut file).is_err() {
                break 'tree;
            }
            bytes_written += size;
        }

        if depth < MAX_TREE_DEPTH {
            let dirs_per_level: usize = rng.random_range(1..=MAX_DIRS_PER_LEVEL);
            for i in 0..dirs_per_level {
                let dirname: String = format!("d{i}");
                if dir.create_dir(&dirname).is_err() {
                    break 'tree;
                }
                let child_path: String = if dir_path.is_empty() {
                    dirname
                } else {
                    format!("{dir_path}/{dirname}")
                };
                stack.push((child_path, depth + 1));
            }
        }
    }

    //==============================================================================================
    // Pass 2 — Fill remaining capacity with large bulk files.
    //==============================================================================================

    let bulk_data: Vec<u8> = vec![0xAA; BULK_FILE_SIZE];
    for i in 0.. {
        let Ok(mut file) = root.create_file(&format!("bulk{i}.dat")) else {
            break;
        };
        if fatfs::Write::write_all(&mut file, &bulk_data).is_err() {
            break;
        }
        if fatfs::Write::flush(&mut file).is_err() {
            break;
        }
    }
}
