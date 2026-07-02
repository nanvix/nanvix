# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Bundled Nanvix C Library Artifacts: libc.a, libm.a, libc.so, crt0.o, stub archives
#===================================================================================================
#
# This fragment stages the complete newlib-free runtime surface the patched
# gcc/clang toolchains link by default:
#
#   - libc.a   : static archive  (libc_* objects + syscall backend + runtime)
#   - libm.a   : static archive  (the nanvix_libm math archive)
#   - libc.so  : shared object   (the same exported symbol surface, for dlopen)
#   - crt0.o   : startup object  (nvx-crt0 entry; crt1.o / Scrt1.o aliases)
#   - libdl.a libpthread.a librt.a : empty stub archives so legacy `-ldl` /
#                `-lpthread` / `-lrt` resolve with no spec drop-list
#
# libc.a embeds the Nanvix system-call backend (carrying `write`, `read`, `open`,
# `__nanvix_libc_start_main`, and the typed kernel-call wrappers), while libm.a
# carries the math routines. The startfile `crt0.o` provides `_do_start`
# unconditionally, and the empty stub archives satisfy ports that still pass
# `-ldl` / `-lpthread` / `-lrt`.
#
# Binutils: the bundle uses host `ld`/`ar`/`ranlib`. The shared object is linked
# with the target's ELF emulation ($(NANVIX_LIBC_ELF_EMULATION)) and `-z notext`
# because the libc objects are built with the target's
# `relocation-model = static`; the resulting text relocations are exactly the
# relocation types the Nanvix dynamic loader already handles for the dlfcn test
# fixtures (libmul.so). `-z muldefs` on the libc.so link tolerates the Rust
# allocator-shim duplicates (`__rust_alloc`, ...) that remain until the deeper
# allocator work lands; the panic handler is already weak and
# `__errno_location` already has a single owner.

# Host binutils used to assemble the bundle (overridable).
#
# Windows has no GNU binutils. Rather than assume a separate LLVM install (the
# benchmark runners have none), the bundle sources its binutils from the SAME
# Rust toolchain that already links the guest (rust-lld, per
# build/targets/$(TARGET)-user.json): the base toolchain ships the flavor-locked
# `gcc-ld/ld.lld`, and the `llvm-tools` rustup component (declared in
# rust-toolchain, so rustup auto-installs it) adds `llvm-ar`/`llvm-objcopy` under
# `lib/rustlib/<host>/bin`. These accept the same flags this bundle uses
# (-shared/-r/-m<emulation>, --whole-archive, -z notext/muldefs,
# --localize-symbol). A standalone `C:/Program Files/LLVM` install and bare PATH
# lookups remain as fallbacks.
ifeq ($(IS_WINDOWS),yes)
# Active Rust toolchain binutils directory: <sysroot>/lib/rustlib/<host>/bin.
# Evaluated from the repo root so the rust-toolchain override (channel + the
# auto-installed llvm-tools component) is in effect. `rustc --print sysroot`
# emits a native (backslash) path, so normalize it to forward slashes.
NANVIX_RUST_SYSROOT := $(subst \,/,$(shell "$(RUSTC)" --print sysroot 2>/dev/null))
NANVIX_RUST_HOST    := $(shell "$(RUSTC)" -vV 2>/dev/null | sed -n 's/^host: //p')
NANVIX_RUST_TC_BIN  := $(NANVIX_RUST_SYSROOT)/lib/rustlib/$(NANVIX_RUST_HOST)/bin
# Legacy fallback: a standalone LLVM install (e.g. developer machines).
NANVIX_DEFAULT_LLVM_BIN := $(shell if [ -d "C:/Program Files/LLVM/bin" ]; then printf '%s' "C:/Program Files/LLVM/bin"; fi)
# `gcc-ld/ld.lld` ships with the base toolchain (no component needed), so its
# presence is a reliable signal that the toolchain bin resolved correctly.
ifneq ($(wildcard $(NANVIX_RUST_TC_BIN)/gcc-ld/ld.lld.exe),)
NANVIX_LIBC_LD      ?= "$(NANVIX_RUST_TC_BIN)/gcc-ld/ld.lld.exe"
NANVIX_LIBC_AR      ?= "$(NANVIX_RUST_TC_BIN)/llvm-ar.exe"
NANVIX_LIBC_RANLIB  ?= "$(NANVIX_RUST_TC_BIN)/llvm-ar.exe"
NANVIX_LIBC_OBJCOPY ?= "$(NANVIX_RUST_TC_BIN)/llvm-objcopy.exe"
else ifneq ($(NANVIX_DEFAULT_LLVM_BIN),)
NANVIX_LIBC_LD      ?= "$(NANVIX_DEFAULT_LLVM_BIN)/ld.lld.exe"
NANVIX_LIBC_AR      ?= "$(NANVIX_DEFAULT_LLVM_BIN)/llvm-ar.exe"
NANVIX_LIBC_RANLIB  ?= "$(NANVIX_DEFAULT_LLVM_BIN)/llvm-ranlib.exe"
NANVIX_LIBC_OBJCOPY ?= "$(NANVIX_DEFAULT_LLVM_BIN)/llvm-objcopy.exe"
else
NANVIX_LIBC_LD      ?= ld.lld
NANVIX_LIBC_AR      ?= llvm-ar
NANVIX_LIBC_RANLIB  ?= llvm-ranlib
NANVIX_LIBC_OBJCOPY ?= llvm-objcopy
endif
else
NANVIX_LIBC_LD      ?= ld
NANVIX_LIBC_AR      ?= ar
NANVIX_LIBC_RANLIB  ?= ranlib
NANVIX_LIBC_OBJCOPY ?= objcopy
endif

# ELF output emulation for the host linker, derived from the guest $(TARGET) so
# the relocatable crt0.o and the libc.so link match the architecture of the
# cargo-built objects (mirroring the `gnu-lld` `-melf_*` arg in
# build/targets/$(TARGET)-user.json). Linking with the wrong emulation fails
# with an architecture-mismatch error (notably folding the 64-bit libnvx_crt0.a
# with elf_i386 on an x86_64 build).
ifeq ($(TARGET),x86_64)
NANVIX_LIBC_ELF_EMULATION := elf_x86_64
else
NANVIX_LIBC_ELF_EMULATION := elf_i386
endif

# Outputs. `libc.a` is now produced DIRECTLY by `all-guest-staticlibs`: a single
# `cargo build -p nanvix_libc` compiles the C library together with the POSIX
# syscall backend and start-of-day driver (pulled in via nanvix_libc's
# `backend-nanvix` feature), so the staticlib already embeds `open`/`read`/`write`/
# `__nanvix_libc_start_main` and a SINGLE unified `sysalloc`/`libc_stdlib`/`sys`/
# `nvx`. It is staged straight under its conventional `-lc` name (`libc.a`) — no
# `ar` merge and no separate `libnanvix_libc.a`, and no standalone `libposix.a`;
# `libc.a` is the only library C applications link against. The old two-archive
# libc merge embedded DUPLICATE instances of those crates (each `sysalloc` with
# its own `HEAP` static): exactly the duplicate-HEAP regression this restructure
# structurally eliminates.
NANVIX_LIBC_BUNDLE_AR := $(LIBRARIES_DIR)/libc.a
NANVIX_LIBC_BUNDLE_SO := $(LIBRARIES_DIR)/libc.so
# Standalone math archive (the `nanvix_libm` crate: libc_math + nvx panic
# handler, no sysalloc). Staged directly by all-guest-staticlibs, exactly like
# libc.a. Shipped alongside libc.a + libnvx_crt0.a as the newlib libc+libm
# replacement.
NANVIX_LIBM_BUNDLE_AR := $(LIBRARIES_DIR)/libm.a
# Shared math surface (`libm.so`), mirroring `libc.so` for dlopen consumers. Its
# forwarding allocator shims reference `libc.so`'s real allocator and resolve at
# load time, so no allocator state is duplicated.
NANVIX_LIBM_BUNDLE_SO := $(LIBRARIES_DIR)/libm.so

# Startup object (the `nvx-crt0` entry: `_do_start` / `_start` / `__nanvix_main`).
#
# Emitted as a single relocatable object. A startfile object is linked
# UNCONDITIONALLY by the native gcc/clang specs and guarantees `_do_start` is
# present — unlike an archive member, which is only pulled if a prior reference
# is undefined. `crt1.o` /
# `Scrt1.o` are glibc-style symlink aliases so build systems expecting those
# names resolve. The existing `libnvx_crt0.a` archive remains for the Rust
# no_std (cargo-lib) path.
#
# NOTE: the object is produced with `ld -r --whole-archive libnvx_crt0.a`, NOT
# the literal relibc `cargo rustc -- --emit obj` recipe. `--emit obj` emits only
# the `nvx-crt0` crate's own object, which leaves an UNDEFINED, crate-hashed
# reference to `nvx::pie::relocate_pie_binary` (nvx-crt0 links `nvx` with
# `default-features = false`; `libc.a` links it with `runtime`, so the v0-mangled
# hashes differ and the symbol never resolves at the native link). Folding the
# whole `libnvx_crt0.a` archive into one relocatable object makes `crt0.o`
# self-contained: it then references only STABLE C-ABI symbols
# (`__nanvix_libc_start_main`, the `__nanvix_rust_*` allocator bridges, `main`,
# `memcpy`, ...) that `libc.a` provides. This guarantees a single startfile that
# provides `_do_start`, and avoids the residual risk of the `--emit obj` path
# leaving an unresolved crate-hashed reference.
NANVIX_CRT0_ARCHIVE := $(LIBRARIES_DIR)/libnvx_crt0.a
NANVIX_CRT0_OBJECT := $(LIBRARIES_DIR)/crt0.o
NANVIX_CRT0_ALIAS_NAMES := crt1.o Scrt1.o
NANVIX_CRT0_ALIAS_OBJECTS := $(foreach alias,$(NANVIX_CRT0_ALIAS_NAMES),$(LIBRARIES_DIR)/$(alias))

# Empty stub archives so ports that still pass `-ldl` / `-lpthread` / `-lrt`
# resolve with NO spec drop-list — every real symbol already lives in `libc.a`
# (the relibc approach). `-lm` resolves to the real `libm.a` above. There is no
# `-lposix`: the syscall backend is embedded in `libc.a`.
NANVIX_LIBC_STUB_ARCHIVES := \
	$(LIBRARIES_DIR)/libdl.a \
	$(LIBRARIES_DIR)/libpthread.a \
	$(LIBRARIES_DIR)/librt.a

# Artifacts that must be installed into the sysroot/release lib directory for
# native C/C++ toolchains: the libc/libm archives, shared libc surface,
# startfile objects/aliases, and compatibility stub archives.
NANVIX_LIBC_BUNDLE_INSTALL_ARTIFACTS := \
	$(NANVIX_LIBC_BUNDLE_AR) \
	$(NANVIX_LIBM_BUNDLE_AR) \
	$(NANVIX_LIBC_BUNDLE_SO) \
	$(NANVIX_LIBM_BUNDLE_SO) \
	$(NANVIX_CRT0_OBJECT) \
	$(NANVIX_CRT0_ALIAS_OBJECTS) \
	$(NANVIX_LIBC_STUB_ARCHIVES)

.PHONY: nanvix-libc-bundle clean-nanvix-libc-bundle

# Bridge: libc.a / libm.a / libnvx_crt0.a are created (with preserved mtimes) by
# the phony all-guest-staticlibs target. These empty-recipe rules let the
# libc.so / crt0.o targets and external dependents key off their mtimes for
# incremental rebuilds, mirroring the guest-ELF bridge rule
# (`$(BINARIES_DIR)/%.elf: all-guest-binaries ;`).
$(NANVIX_LIBC_BUNDLE_AR): all-guest-staticlibs ;
# libm.a embeds two families of DEAD DUPLICATE globals that `libc.a` also owns,
# so a default (no `-z muldefs` / no `-Wl,--allow-multiple-definition`) static
# link of `libc.a + libm.a` would otherwise fail with "multiple definition of
# ...". Demote both to LOCAL so `libc.a` stays their single owner, mirroring the
# crt0.o treatment below.
#
#   1. Rust global-allocator shims (`__rust_alloc`, `__rust_dealloc`,
#      `__rust_realloc`). `nanvix_libm` carries only a *forwarding*
#      `#[global_allocator]` (see nanvix_libm/src/lib.rs) — rustc still emits
#      these global shim symbols because every `alloc`-linking staticlib must
#      declare a global allocator, but the real allocator lives in `libc.a`.
#
#   2. The `sys`-crate kernel-call / fork / thread startup stubs (`__kcall_*`,
#      `fork_save_context`, `fork_trampoline`, `_do_start_thread`,
#      `_do_exit_thread`). `nanvix_libm` links `nvx` (for the `#[panic_handler]`),
#      and `nvx` depends on `sys` with the `kcall` feature, so the WHOLE `sys`
#      kernel-call backend — whose C-ABI wrappers are `#[no_mangle]`, hence always
#      emitted — is folded into `libm.a`. The math routines are pure-computational
#      and the panic handler only issues `ud2`, so these stubs are dead here;
#      `libc.a` embeds the same backend and is their single owner. The `__kcall_*`
#      pattern is anchored (no leading `*`) so it matches only the C-ABI wrappers
#      and leaves any name-mangled Rust `__kcall_*` reference untouched.
$(NANVIX_LIBM_BUNDLE_AR): all-guest-staticlibs
	@echo "[nanvix-libc] demoting libm.a allocator shims + sys stubs to local (single-owner)"
	$(NANVIX_LIBC_OBJCOPY) --wildcard \
		-L '*__rust_alloc*' -L '*__rust_dealloc*' -L '*__rust_realloc*' \
		-L '__kcall_*' -L 'fork_save_context' -L 'fork_trampoline' \
		-L '_do_start_thread' -L '_do_exit_thread' \
		$@
$(NANVIX_CRT0_ARCHIVE): all-guest-staticlibs ;

# libc.so: shared object (same exported surface, for dlopen), linked from libc.a.
$(NANVIX_LIBC_BUNDLE_SO): $(NANVIX_LIBC_BUNDLE_AR)
	@echo "[nanvix-libc] linking libc.so (shared)"
	$(NANVIX_LIBC_LD) -shared -m $(NANVIX_LIBC_ELF_EMULATION) -z notext -z muldefs \
		--whole-archive $(NANVIX_LIBC_BUNDLE_AR) --no-whole-archive \
		-o $@

# libm.so: shared math object. Linked from the (single-owner) libm.a; its
# forwarding allocator shims are local, so the link needs no `-z muldefs` and the
# real allocator stays in libc.so. `-z notext` allows the R_386_* text
# relocations the static-relocation-model objects emit, exactly like libc.so.
$(NANVIX_LIBM_BUNDLE_SO): $(NANVIX_LIBM_BUNDLE_AR)
	@echo "[nanvix-libc] linking libm.so (shared)"
	$(NANVIX_LIBC_LD) -shared -m $(NANVIX_LIBC_ELF_EMULATION) -z notext \
		--whole-archive $(NANVIX_LIBM_BUNDLE_AR) --no-whole-archive \
		-o $@

# crt0.o: single relocatable startup object folded from libnvx_crt0.a (see the
# NOTE above). The crt1.o / Scrt1.o aliases are regular object copies so the
# sysroot/release bundle does not depend on symlink support.
#
# After folding, two families of DEAD DUPLICATE globals that `libc.a` also owns
# are demoted to LOCAL, so `libc.a` stays their single owner and a default
# (no `-z muldefs` / no `-Wl,--allow-multiple-definition`) static link of
# `crt0.o + libc.a` no longer fails with "multiple definition of ...". As ARCHIVE
# members these dead copies are harmless (the linker skips them once `libc.a`
# satisfies the reference), but `ld -r --whole-archive` folds EVERY member into
# `crt0.o` as an UNCONDITIONAL global definition, so any symbol `crt0` merely
# carries (but does not itself reference) collides with `libc.a`'s copy at the
# native link.
#
#   1. Rust global-allocator / compiler-builtins runtime shims (`__rust_alloc`,
#      `__rust_dealloc`, `__rust_realloc`, `__rust_alloc_zeroed`,
#      `__rdl_alloc_error_handler`, `__rust_no_alloc_shim_is_unstable_v2`,
#      `__rust_probestack`). `nvx-crt0` only carries a *forwarding*
#      `#[global_allocator]` (built with the `forwarding-allocator` feature) — it
#      has no allocation sites of its own and exists solely to satisfy rustc's
#      requirement that every `alloc`-linking staticlib declare a global
#      allocator (see nvx-crt0/src/lib.rs). The real allocator (and the
#      compiler-builtins `__rust_probestack` intrinsic) live in `libc.a`, which
#      exports the SAME shim symbols.
#
#   2. The `sys`-crate kernel-call / fork / thread startup stubs (`__kcall_*`,
#      `fork_save_context`, `fork_trampoline`, `_do_start_thread`,
#      `_do_exit_thread`). `nvx-crt0` depends on `sys` with the `kcall` feature
#      because `_start` registers the signal restorer via `__kcall_sig_restorer()`,
#      so the WHOLE `sys` kernel-call backend is folded into `crt0.o`. `crt0`
#      reaches the restorer through its name-mangled Rust symbol, so the C-ABI
#      `__kcall_*` wrappers and the fork / thread assembly helpers are dead
#      duplicates here; `libc.a` embeds the same backend and is their single owner.
#
# Demoting crt0's dead copies to local keeps `libc.a` the single owner and
# realizes the "crt0 is a stateless startup shim" invariant (see
# nvx-crt0/src/lib.rs) — without a `-z muldefs` reliance. The `__kcall_*` pattern
# is anchored (no leading `*`) so it matches only the C-ABI wrappers and leaves
# crt0's own name-mangled `__kcall_sig_restorer` reference untouched.
$(NANVIX_CRT0_OBJECT): $(NANVIX_CRT0_ARCHIVE)
	@echo "[nanvix-libc] emitting crt0.o (nvx-crt0 startup object)"
	$(NANVIX_LIBC_LD) -r -m $(NANVIX_LIBC_ELF_EMULATION) \
		--whole-archive $(NANVIX_CRT0_ARCHIVE) --no-whole-archive \
		-o $@
	$(NANVIX_LIBC_OBJCOPY) --wildcard \
		-L '*__rust_alloc*' -L '*__rust_dealloc*' -L '*__rust_realloc*' \
		-L '*__rdl_alloc_error_handler*' -L '*__rust_no_alloc_shim_is_unstable*' \
		-L '*__rust_probestack*' \
		-L '__kcall_*' -L 'fork_save_context' -L 'fork_trampoline' \
		-L '_do_start_thread' -L '_do_exit_thread' \
		$@
	@for alias in $(NANVIX_CRT0_ALIAS_NAMES); do \
		$(CP_CMD) $(NANVIX_CRT0_OBJECT) $(LIBRARIES_DIR)/$$alias; \
	done

# Empty -ldl/-lpthread/-lrt stub archives. `ar -rcs` on no members produces a
# valid empty archive with an (empty) symbol index.
$(NANVIX_LIBC_STUB_ARCHIVES):
	@echo "[nanvix-libc] creating empty stub archive $(@F)"
	$(NANVIX_LIBC_AR) -rcs $@

# Convenience alias to build all artifacts (libc.a + libm.a + libc.so + libm.so +
# crt0.o [+ aliases] + the empty -ldl/-lpthread/-lrt stub archives).
nanvix-libc-bundle: $(NANVIX_LIBC_BUNDLE_AR) $(NANVIX_LIBM_BUNDLE_AR) \
	$(NANVIX_LIBC_BUNDLE_SO) $(NANVIX_LIBM_BUNDLE_SO) $(NANVIX_CRT0_OBJECT) $(NANVIX_LIBC_STUB_ARCHIVES)

clean-nanvix-libc-bundle:
	$(RM_CMD) $(NANVIX_LIBC_BUNDLE_AR) $(NANVIX_LIBM_BUNDLE_AR) $(NANVIX_LIBC_BUNDLE_SO) $(NANVIX_LIBM_BUNDLE_SO)
	$(RM_CMD) $(NANVIX_CRT0_OBJECT) $(NANVIX_LIBC_STUB_ARCHIVES)
	$(RM_CMD) $(NANVIX_CRT0_ALIAS_OBJECTS)

# Produce the bundle as part of a full build.
all-nanvix: nanvix-libc-bundle
