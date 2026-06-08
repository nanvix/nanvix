# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

GUEST_STATICLIB_FEATURES := staticlib $(LOG_LEVEL)
# Enable standalone mode: routes stdout/stderr to debug kcall,
# file I/O to in-memory VFS, and disables IPC-based syscalls (no linuxd).
ifeq ($(DEPLOYMENT_MODE),standalone)
GUEST_STATICLIB_FEATURES += standalone
endif
GUEST_STATICLIB_FEATURES := $(strip $(GUEST_STATICLIB_FEATURES))
GUEST_STATICLIB_CARGO_FEATURES := $(if $(GUEST_STATICLIB_FEATURES),--features "$(GUEST_STATICLIB_FEATURES)")

#===================================================================================================
# Per-package feature overrides
#===================================================================================================
# Some staticlib packages take a different feature set than the default
# `staticlib + LOG_LEVEL [+ standalone]` combination above. Define the
# overrides here as `GUEST_STATICLIB_FEATURES_<package>` and the lookup
# helper below picks them up.
#
# nvx-crt0 is the C-executable startup crate. It is built once with the
# `c-main` feature (so the sysroot copy of `libnvx_crt0.a` is the variant a
# C executable like `python.elf` links against). Rust no_std binaries that
# need the `rust-main` flavour pull `nvx-crt0` in directly via cargo as a
# normal dependency, with the appropriate feature toggle in their own
# `Cargo.toml`, so they do not consume the sysroot copy.
GUEST_STATICLIB_FEATURES_nvx-crt0 := c-main $(LOG_LEVEL)
GUEST_STATICLIB_FEATURES_nvx-crt0 := $(strip $(GUEST_STATICLIB_FEATURES_nvx-crt0))

# Returns the cargo `--features "..."` arg for the given package, falling
# back to the default $(GUEST_STATICLIB_CARGO_FEATURES) when no override is
# defined.
GUEST_STATICLIB_PKG_FEATURES = $(if $(GUEST_STATICLIB_FEATURES_$(1)),--features "$(GUEST_STATICLIB_FEATURES_$(1))",$(GUEST_STATICLIB_CARGO_FEATURES))

# Cargo crate names with hyphens produce artifacts with underscores (e.g.
# `nvx-crt0` -> `libnvx_crt0.a`). This helper normalises the package name
# for `cp` / `rm` paths against the cargo target dir.
guest_staticlib_artifact = lib$(subst -,_,$(1)).a

#===================================================================================================
# Guest staticlib libm visibility fix
#===================================================================================================
# Rust's `compiler_builtins` crate (pulled in transitively by `core` for every
# guest staticlib) emits ~28 libm wrapper symbols as `STB_WEAK + STV_HIDDEN`
# on every non-Windows, non-Apple target. Source: `compiler-builtins`
# `src/macros.rs` (linkage = "weak" for non-Windows/Apple) and
# `src/math/mod.rs` (the `full_availability` block).
#
# Most of compiler_builtins is integer-arithmetic / soft-float helpers
# (`__adddf3`, `__divdi3`, `__bswapdi2`, ...) that newlib's libc / libm do
# NOT provide. They MUST stay WEAK HIDDEN where they are — they are the
# canonical providers for any C or Rust code that needs them.
#
# The libm wrappers (the 28 names below) are different. Newlib's libm
# defines all of them as `STB_GLOBAL + STV_DEFAULT`. Under normal archive
# linking this is harmless: WEAK loses to STRONG. But Nanvix guest
# executables that dlopen extension modules (CPython, future plugins) link
# with:
#
#   -Wl,--whole-archive libnvx_crt0.a libposix.a libc.a libm.a ...
#   -Wl,--allow-multiple-definition
#   -Wl,--export-dynamic
#
# so dlopen'd modules can resolve libm names at runtime against the main
# executable's `.dynsym`. With this combination GNU ld merges the two
# `sqrt` definitions body-from-strong-visibility-from-most-restrictive. The
# result is STRONG HIDDEN, demoted to LOCAL after link. `--export-dynamic`
# cannot put a LOCAL symbol into `.dynsym`, so dlopen'd modules fail at
# runtime with "symbol not found" on `sqrt`/`cbrt`/etc.
#
# The fix: after each guest staticlib is built, run `rust-objcopy
# --localize-symbol=<sym>` on each libm wrapper name it defines. LOCAL
# symbols don't participate in cross-object resolution, so libm.a's STRONG
# DEFAULT definitions become the only visible-to-the-linker definitions.
# `--export-dynamic` works as intended. Symbols not in the list are
# untouched, so integer/soft-float intrinsics keep their WEAK HIDDEN
# semantics.
#
# Why a hardcoded list rather than autodiscovery: the 28 names below
# are the C99 math.h public API. That contract is frozen — newlib's
# libm has all of them, every other libm (glibc, musl, picolibc, ...)
# has all of them, and the set has not changed in decades. Hardcoding
# against a stable specification is the most robust option here: the
# list will not drift under us, and the localization step has no
# build-time dependency on any other artifact.
#
# A "diff against libm.a" alternative was considered and rejected for
# two reasons:
#
#   1. libm.a is not available at libposix.a build time in any of the
#      build paths nanvix itself uses:
#
#        - Windows native (`z.ps1`, dev + CI `windows-latest`): no C
#          cross-toolchain installed, libm.a does not exist on the host.
#        - Linux native without `./z setup --nanvix-sdk`: same — newlib
#          is only built when the SDK target is requested.
#        - Linux CI (`ghcr.io/nanvix/ci:ubuntu-24.04`): the nanvix CI
#          image carries the Rust toolchain only; no newlib, no libm.a.
#
#      libm.a only exists in the separate full-SDK Docker image
#      (`nanvix/toolchain` / `ghcr.io/nanvix/toolchain-*`) that is
#      consumed by downstream projects like cpython. Those projects
#      link against the libposix.a we ship, but they build *after*
#      libposix.a, so they cannot influence its localization step.
#
#   2. Even on a setup that does have libm.a available, introspecting
#      it at libposix.a build time would couple two otherwise
#      independent build stages and add machinery (locate libm.a, run
#      `nm`, filter visibility — `nm`'s single-letter format does not
#      distinguish HIDDEN from DEFAULT, so this needs care) without
#      removing the need for a stable list as fallback. The
#      cost/benefit does not justify the complexity.
#
# A proper long-term fix lives upstream in `compiler-builtins`: add a
# target-spec field like `has-system-libm` to `i686-unknown-nanvix` so
# the `full_availability` block in `src/math/mod.rs` is skipped at the
# source. No such field exists today and no upstream PR is in flight;
# until one lands, this post-build localization is the canonical
# nanvix fix.
#
# Safety of localization: no Nanvix Rust no_std guest code calls these
# C math symbols. `core`'s `f64::sqrt()` etc. lower to LLVM intrinsics
# (`@llvm.sqrt.f64` -> hardware FSQRT/SQRTSD), bypassing extern "C"
# entirely. The `std` variants that DO call C math (`sin`, `cos`, `tan`,
# ...) are absent from `-Zbuild-std=core,alloc`. So the WEAK HIDDEN math
# symbols have no Rust caller; localizing them affects only the
# cross-archive visibility merge.
#
# Note on the 11 newlib-internal `__math_*` helpers (`__math_invalid`,
# `__math_oflow`, ...): those are GLOBAL HIDDEN at source in
# `newlib/libm/common/math_config.h` (ported from ARM optimized-routines,
# same as glibc and musl). They are deliberately library-private and
# called only from inside libm's own internals (e.g. `__ieee754_sqrt` ->
# `__math_invalid`). In a Nanvix executable, those internals live inside
# the executable alongside the helpers themselves; the PC-relative call
# between them is resolved at static-link time and never touches
# `.dynsym`. dlopen'd modules don't need them — they call public names
# like `sqrt`, which dispatches to the executable's `sqrt`, which
# internally calls `__math_invalid` if needed. This is identical to how
# Linux ships libm.so.6 with the same HIDDEN attribute on the same
# helpers, and dlopen'd code on Linux never needs them either. NOT
# exporting them is the correct behaviour, not a bug.
#
# Tooling: rust-objcopy comes from cargo-binutils, a build prerequisite
# (see build/make/kernel.mk for an identical fallback pattern). The
# command is a no-op on staticlibs that don't define these symbols.
GUEST_STATICLIB_OBJCOPY := $(shell command -v rust-objcopy 2>/dev/null || command -v objcopy 2>/dev/null)

# C99 libm wrapper names that newlib's libm.a defines as STRONG DEFAULT and
# that compiler_builtins shadows as WEAK HIDDEN. Stable; effectively pinned
# by the C99 math.h public API.
GUEST_STATICLIB_LIBM_WRAPPERS := \
	cbrt cbrtf ceil ceilf copysign copysignf fabs fabsf \
	fdim fdimf floor floorf fma fmaf fmax fmaxf fmin fminf \
	fmod fmodf rint rintf round roundf sqrt sqrtf trunc truncf

GUEST_STATICLIB_LIBM_LOCALIZE_ARGS := \
	$(addprefix --localize-symbol=,$(GUEST_STATICLIB_LIBM_WRAPPERS))

define GUEST_STATICLIB_LIBM_FIX_CMD
	if [ -n "$(GUEST_STATICLIB_OBJCOPY)" ]; then \
	    $(GUEST_STATICLIB_OBJCOPY) $(GUEST_STATICLIB_LIBM_LOCALIZE_ARGS) $(1); \
	else \
	    echo "WARNING: rust-objcopy/objcopy not found; skipping libm visibility fix on $(1)"; \
	fi;
endef

# Per-package rules retained for direct invocation (e.g., make all-guest-staticlib-<pkg>).
define GUEST_STATICLIB_RULES
all-guest-staticlib-$(1): init
	$(GUEST_CARGO_BUILD_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$(call guest_staticlib_artifact,$(1)) $(LIBRARIES_DIR)/$(call guest_staticlib_artifact,$(1))
	@$(call GUEST_STATICLIB_LIBM_FIX_CMD,$(LIBRARIES_DIR)/$(call guest_staticlib_artifact,$(1)))

check-guest-staticlib-$(1):
	@$(GUEST_CARGO_CHECK_CMD) -p $(1)
	@$(GUEST_CARGO_CHECK_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))

format-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-staticlib-$(1):
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(LIBRARIES_DIR)/$(call guest_staticlib_artifact,$(1))

rust-lint-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1)) --fix --allow-dirty --allow-no-vcs

rust-lint-check-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1)) -- -D warnings
endef

$(foreach target,$(ALL_GUEST_STATIC_LIBS),$(eval $(call GUEST_STATICLIB_RULES,$(target))))

# Batched targets: invoke cargo once per distinct feature set so that packages
# with overrides (e.g. nvx-crt0) get their own feature flags rather than the
# default $(GUEST_STATICLIB_CARGO_FEATURES) bundle. Per-package override
# invocations are unrolled at make-time (one cargo call per override package)
# instead of looping in shell, so that the per-package feature lookup happens
# at make-time when the package name is statically known.
_GUEST_STATICLIB_PKGS_DEFAULT := $(foreach pkg,$(ALL_GUEST_STATIC_LIBS),$(if $(GUEST_STATICLIB_FEATURES_$(pkg)),,-p $(pkg)))
_GUEST_STATICLIB_PKGS_OVERRIDE := $(foreach pkg,$(ALL_GUEST_STATIC_LIBS),$(if $(GUEST_STATICLIB_FEATURES_$(pkg)),$(pkg),))

# Macros that emit the per-override commands for the batched targets.
define _OVERRIDE_BUILD_CMD
	$(GUEST_CARGO_BUILD_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))
endef

define _OVERRIDE_CHECK_CMDS
	@$(GUEST_CARGO_CHECK_CMD) -p $(1)
	@$(GUEST_CARGO_CHECK_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))
endef

define _OVERRIDE_CLIPPY_CMD
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1)) --fix --allow-dirty --allow-no-vcs
endef

define _OVERRIDE_CLIPPY_CHECK_CMD
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1)) -- -D warnings
endef

all-guest-staticlibs: init
	$(if $(_GUEST_STATICLIB_PKGS_DEFAULT),$(GUEST_CARGO_BUILD_CMD) $(_GUEST_STATICLIB_PKGS_DEFAULT) $(GUEST_STATICLIB_CARGO_FEATURES))
	$(foreach pkg,$(_GUEST_STATICLIB_PKGS_OVERRIDE),$(call _OVERRIDE_BUILD_CMD,$(pkg)) &&) true
	@for pkg in $(ALL_GUEST_STATIC_LIBS); do \
		artifact=lib$$(echo $$pkg | sed 's/-/_/g').a; \
		$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$$artifact $(LIBRARIES_DIR)/$$artifact; \
		$(call GUEST_STATICLIB_LIBM_FIX_CMD,$(LIBRARIES_DIR)/$$artifact) \
	done

check-guest-staticlibs:
	$(if $(_GUEST_STATICLIB_PKGS_DEFAULT),@$(GUEST_CARGO_CHECK_CMD) $(_GUEST_STATICLIB_PKGS_DEFAULT))
	$(if $(_GUEST_STATICLIB_PKGS_DEFAULT),@$(GUEST_CARGO_CHECK_CMD) $(_GUEST_STATICLIB_PKGS_DEFAULT) $(GUEST_STATICLIB_CARGO_FEATURES))
	$(foreach pkg,$(_GUEST_STATICLIB_PKGS_OVERRIDE),$(call _OVERRIDE_CHECK_CMDS,$(pkg)))

format-guest-staticlibs:
	$(GUEST_CARGO_FMT_CMD) $(foreach pkg,$(ALL_GUEST_STATIC_LIBS),-p $(pkg))

format-check-guest-staticlibs:
	$(GUEST_CARGO_FMT_CMD) $(foreach pkg,$(ALL_GUEST_STATIC_LIBS),-p $(pkg)) --check

clean-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),clean-guest-staticlib-$(target))

rust-lint-guest-staticlibs:
	$(if $(_GUEST_STATICLIB_PKGS_DEFAULT),$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_STATICLIB_PKGS_DEFAULT) $(GUEST_STATICLIB_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs)
	$(foreach pkg,$(_GUEST_STATICLIB_PKGS_OVERRIDE),$(call _OVERRIDE_CLIPPY_CMD,$(pkg)) &&) true

rust-lint-check-guest-staticlibs:
	$(if $(_GUEST_STATICLIB_PKGS_DEFAULT),$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_STATICLIB_PKGS_DEFAULT) $(GUEST_STATICLIB_CARGO_FEATURES) -- -D warnings)
	$(foreach pkg,$(_GUEST_STATICLIB_PKGS_OVERRIDE),$(call _OVERRIDE_CLIPPY_CHECK_CMD,$(pkg)) &&) true
