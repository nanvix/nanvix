// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use std::{
    env,
    fs::{
        self,
        File,
    },
    io::Read,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Pre-built Shared Libraries
//==================================================================================================
//
// The dlfcn tests load `libmul.so` and `libmul-pie.so` at runtime via `dlopen()`.
// These are pre-built i386 ELF shared objects checked into `libs/`.  No C compiler
// is required at build time — the build script simply copies them to LIBRARIES_DIR.
//
// If the test shared libraries ever need to be regenerated, follow these steps on a
// Linux host with `clang` and GNU `ld` installed:
//
// ## Source (libs/mul.c — removed from tree, preserved here for reference):
//
// ```c
// /*
//  * Copyright(c) The Maintainers of Nanvix.
//  * Licensed under the MIT License.
//  */
//
// // R_386_GLOB_DAT
// const char *VERSION = "0.0.1";
//
// int add(int a, int b)
// {
//     return (a + b);
// }
//
// // R_386_32
// int fast_mul(int a, int b)
// {
//     int result = 0;
//     __asm__ __volatile__("movl %1, %%ecx;"
//                          "movl $0, %0;"
//                          "test %%ecx, %%ecx;"
//                          "jz 1f;"
//                          "0:;"
//                          "pushl %2;"
//                          "pushl %0;"
//                          "call add;" // R_386_PC32
//                          "addl $8, %%esp;"
//                          "movl %%eax, %0;"
//                          "loop 0b;"
//                          "1:;"
//                          : "=r"(result)
//                          : "r"(b), "r"(a)
//                          : "ecx", "eax", "cc");
//     return result;
// }
//
// int slow_mul(int a, int b)
// {
//     int result = 0;
//
//     for (int i = 0; i < b; i++) {
//         // R_386_JUMP_SLOT
//         result = add(result, a);
//     }
//
//     return (result);
// }
//
// static int (*mul)(int, int) = &fast_mul;
//
// int multiply(int a, int b)
// {
//     return (mul(a, b));
// }
//
// const char *get_version(void)
// {
//     return (VERSION);
// }
// ```
//
// ## Build commands:
//
// ```sh
// # Compile to object file (i686 bare-metal, position-independent).
// clang --target=i686-unknown-none -m32 -march=pentiumpro \
//       -nostdlib -ffreestanding -fPIC -c mul.c -o mul.o
//
// # Link into a shared library (allow text relocations for R_386_PC32
// # from the inline-asm `call add` instruction).
// ld -shared -melf_i386 -z notext mul.o -o libmul.so
//
// # The same object produces the PIE variant.
// cp libmul.so libmul-pie.so
// ```
//
// ## Expected relocation types (manually verify with `readelf -r libmul.so`):
//
// These relocations are expected to be present in the prebuilt `libmul*.so`
// artifacts. At the moment, this build script does not enforce them
// automatically; if you regenerate the shared libraries, please confirm that
// the following relocation kinds are still present:
//
// - R_386_RELATIVE  — position-independent base relocations
// - R_386_PC32      — PC-relative call (`call add` from inline asm)
// - R_386_GLOB_DAT  — global data symbol (`VERSION`, function pointer `mul`)
// - R_386_32        — direct symbol address
// - R_386_JUMP_SLOT — PLT slot for function calls (`add` from `slow_mul`)
//

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    //==============================================================================================
    // Configuration
    //==============================================================================================

    // Get CARGO_MANIFEST_DIR Environment Variable.
    let manifest_dir: String = match env::var("CARGO_MANIFEST_DIR") {
        Ok(mdir) => mdir,
        Err(_) => panic!("failed to get CARGO_MANIFEST_DIR environment variable"),
    };

    let config_path: PathBuf = Path::new(&manifest_dir).join("config.json");
    if config_path.exists() {
        println!("cargo:rerun-if-changed=./config.json");
        let mut file: File = File::open(&config_path).expect("Failed to open file");
        let mut raw_content: String = String::new();
        let _ = file.read_to_string(&mut raw_content);
        let content = raw_content.replace("\n", "").replace(" ", "");
        println!("cargo:rustc-env=CONFIG={content}");
    }

    //==============================================================================================
    // Install Pre-built Shared Libraries (libmul.so and libmul-pie.so)
    //==============================================================================================

    // Pre-built i386 ELF shared objects are checked into libs/.
    // Rebuild them only if the pre-built binaries change.
    println!("cargo:rerun-if-changed=libs/libmul.so");
    println!("cargo:rerun-if-changed=libs/libmul-pie.so");

    // When rust-analyzer (or plain `cargo check`) runs without the full build
    // environment, LIBRARIES_DIR is absent.  Skip the shared-library copy
    // and linker configuration so the IDE can still provide diagnostics.
    println!("cargo:rerun-if-env-changed=LIBRARIES_DIR");
    let libraries_dir: String = match env::var("LIBRARIES_DIR") {
        Ok(v) => v,
        Err(_) => {
            println!(
                "cargo:warning=Skipping dlfcn shared library install and linker configuration: \
                 LIBRARIES_DIR is not set."
            );
            return;
        },
    };
    let lib_dir = Path::new(&libraries_dir);

    // Ensure the output directory exists (it may not in a fresh CI checkout).
    fs::create_dir_all(lib_dir)
        .unwrap_or_else(|e| panic!("failed to create {}: {e}", lib_dir.display()));

    let src_dir = Path::new(&manifest_dir).join("libs");

    // Copy pre-built shared libraries to LIBRARIES_DIR.
    for name in &["libmul.so", "libmul-pie.so"] {
        let src = src_dir.join(name);
        let dst = lib_dir.join(name);
        fs::copy(&src, &dst).unwrap_or_else(|e| {
            panic!("failed to copy {} to {}: {e}", src.display(), dst.display())
        });
    }

    //==============================================================================================
    // Linker Configuration
    //==============================================================================================

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86".to_string());
    println!("cargo::rustc-link-arg=-Tbuild/user/linker/{target_arch}/user.ld");

    // Build as a position-independent executable so the linker emits .dynsym,
    // .dynstr, .dynamic, and .hash sections.  Without -pie the binary is fully
    // static and --export-dynamic is silently ignored.
    println!("cargo::rustc-link-arg=-pie");

    // Export all symbols to the dynamic symbol table so that dlsym() with
    // DlHandle::GLOBAL can resolve symbols defined in the main executable.
    println!("cargo::rustc-link-arg=--export-dynamic");

    // Suppress PT_INTERP — Nanvix has no system dynamic linker.
    println!("cargo::rustc-link-arg=--no-dynamic-linker");

    // Allow text relocations (R_386_32 against local symbols from Rust
    // libraries compiled without -fPIC).  GNU ld allows these by default
    // but lld rejects them unless explicitly permitted.
    println!("cargo::rustc-link-arg=-z");
    println!("cargo::rustc-link-arg=notext");

    // Disable RELRO.  LLD's default RELRO layout creates a separate LOAD
    // segment for .dynamic right after .text.  That segment starts mid-page
    // (sharing the last page of .text) with RW permissions, while .text
    // maps the same page R+X.  The Nanvix kernel maps pages per-segment and
    // cannot remap a page with conflicting permissions — this causes a crash.
    // With -z norelro, LLD places .dynamic inside the regular data segment,
    // matching the layout GNU ld produces.
    println!("cargo::rustc-link-arg=-z");
    println!("cargo::rustc-link-arg=norelro");

    // Use legacy SysV hash style to avoid .gnu.hash / .hash section type
    // mismatch when lld merges sections specified by the linker script.
    println!("cargo::rustc-link-arg=--hash-style=sysv");
}
