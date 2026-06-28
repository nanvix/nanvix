# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

GUEST_STATICLIB_FEATURES := $(LOG_LEVEL)
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
# `$(LOG_LEVEL) [+ standalone]` combination above. Define the overrides here
# as `GUEST_STATICLIB_FEATURES_<package>` and the lookup helper below picks
# them up.
#
# nvx-crt0 is the C-executable startup crate.  It is built once with the
# `c-main` feature (so `libnvx_crt0.a` contains the C trampoline used by
# `python.elf`, `hello-c`, `smoke`, ...).  The crate is deliberately
# stateless — it does not depend on `sysalloc` and has no
# `#[global_allocator]` — so `libnvx_crt0.a` carries no `sysalloc`
# objects.  All heap state (`VADDR_NEXT`, ...) lives exclusively in
# `libposix.a` via `__nanvix_libc_start_main`.  See
# `nanvix-todo/dlopen-load-address-conflict.md` for the bug this
# structurally prevents.
#
# Rust no_std binaries that need the `rust-main` flavour pull `nvx-crt0`
# in directly via cargo as a normal dependency, with the appropriate
# feature toggle in their own `Cargo.toml`.  Those binaries depend on
# `sysalloc` explicitly (and on `nvx-crt0` with `provides-allocator`),
# because the workspace `nvx` dependency is `default-features = false`
# and therefore does NOT enable `nvx`'s `runtime` feature or pull
# `sysalloc` in transitively.  Cargo unifies all of these into a single
# compilation per binary, so they do not consume the sysroot copy and
# do not hit the duplicate-`sysalloc` scenario.
# The `init-array` feature switches the `c-main` trampoline from the legacy
# `_init` / `_fini` hooks to relying on `posix/init-array`
# (`__nanvix_libc_start_main` walking `.init_array` / `.fini_array`).  Both
# halves are enabled together for the bundle: `crt0.o` here and `libc.a` via
# `nanvix_libc`'s `posix` dependency.
GUEST_STATICLIB_FEATURES_nvx-crt0 := c-main forwarding-allocator init-array $(LOG_LEVEL)
GUEST_STATICLIB_FEATURES_nvx-crt0 := $(strip $(GUEST_STATICLIB_FEATURES_nvx-crt0))

# nanvix_libc is the C library aggregator that produces the deliverable libc.a.
# It now ALSO pulls the POSIX syscall backend (`posix`) in via its
# `backend-nanvix` feature, so a SINGLE `cargo build -p nanvix_libc` compiles the
# complete C library + backend together. Cargo unifies every shared transitive
# dependency (sysalloc, libc_stdlib, sys, nvx, syslog) to ONE instance, which
# structurally prevents the duplicate-`sysalloc`/duplicate-`HEAP` bug the old
# separate-build + `ar`-merge produced. It therefore takes the DEFAULT feature
# set ($(LOG_LEVEL) [+ standalone]); its `standalone` feature forwards to
# `posix/standalone`.
#
# (No `GUEST_STATICLIB_FEATURES_nanvix_libc` override: it must use the default
# bundle so `standalone` reaches the embedded `posix` backend.)

# nanvix_libm produces libm.a (libc_math + the nvx panic handler). It has no
# standalone-specific behaviour and pulls no syscall backend, so it takes ONLY
# the log level (never the `standalone` feature, which it does not define).
GUEST_STATICLIB_FEATURES_nanvix_libm := $(LOG_LEVEL)
GUEST_STATICLIB_FEATURES_nanvix_libm := $(strip $(GUEST_STATICLIB_FEATURES_nanvix_libm))

# posix is the standalone Nanvix syscall backend (libposix.a), shipped for
# out-of-tree C consumers that link it alongside their own libc. It takes an
# EXPLICIT feature override — the default `$(LOG_LEVEL) [+ standalone]` set (KEEPING
# posix's defaults `syscall allocator c-main`; no `--no-default-features`) plus
# `newlib-compat` — for two reasons:
#
#   1. To force posix into the per-package build group so its `libposix.a` is
#      compiled on its OWN, separately from `nanvix_libc`. `nanvix_libc` depends
#      on `posix` with `init-array`; a combined `cargo build -p posix -p
#      nanvix_libc` would unify features and bake `init-array` into `libposix.a`,
#      imposing a `.preinit/.init/.fini_array` linker-script contract on those
#      out-of-tree consumers. Building posix alone keeps `libposix.a` on the
#      legacy `_init`/`_fini` contract, exactly as before.
#   2. `newlib-compat` surfaces the syscall-backed `sigaction` / `sigprocmask`
#      that Newlib-linked ports (e.g. xz) need from `libposix.a`. It is scoped to
#      THIS standalone build on purpose: the `nanvix_libc` bundle (`libc.a`)
#      already defines those symbols (via `nanvix_libc` / `libc_signal`), so
#      enabling them in the bundle's embedded posix too would duplicate them.
GUEST_STATICLIB_FEATURES_posix := $(GUEST_STATICLIB_FEATURES) newlib-compat
GUEST_STATICLIB_FEATURES_posix := $(strip $(GUEST_STATICLIB_FEATURES_posix))

# Returns the cargo `--features "..."` arg for the given package, falling
# back to the default $(GUEST_STATICLIB_CARGO_FEATURES) when no override is
# defined.
GUEST_STATICLIB_PKG_FEATURES = $(if $(GUEST_STATICLIB_FEATURES_$(1)),--features "$(GUEST_STATICLIB_FEATURES_$(1))",$(GUEST_STATICLIB_CARGO_FEATURES))

# Cargo crate names with hyphens produce artifacts with underscores (e.g.
# `nvx-crt0` -> `libnvx_crt0.a`). This helper normalises the package name
# for `cp` / `rm` paths against the cargo target dir.
guest_staticlib_artifact = lib$(subst -,_,$(1)).a

# Destination (staged) name in LIBRARIES_DIR. Defaults to the cargo output name,
# EXCEPT the two aggregator staticlibs are staged under their conventional `-l`
# names: `nanvix_libc` -> `libc.a` (embeds the POSIX backend) and `nanvix_libm`
# -> `libm.a` (the math archive). There is no separate `libnanvix_libc.a` or
# `libnanvix_libm.a`; `posix` still produces the standalone `libposix.a` for
# out-of-tree consumers that link it with their own libc.
guest_staticlib_staged = $(if $(filter nanvix_libc,$(1)),libc.a,$(if $(filter nanvix_libm,$(1)),libm.a,$(call guest_staticlib_artifact,$(1))))

# Per-package crate-type override. Some packages keep `crate-type = ["lib"]`
# only in `Cargo.toml` (so they can be used as a normal cargo dep without
# rustc's staticlib-time `#[global_allocator]` requirement) and rely on the
# Makefile to ask `cargo rustc` to emit the `staticlib` crate type when
# producing the sysroot artifact.  See `nvx-crt0`'s `Cargo.toml` for the
# rationale.
GUEST_STATICLIB_CRATE_TYPE_nvx-crt0 := staticlib

# Macro returning the appropriate per-package `cargo build`/`cargo rustc`
# command.  Packages with a `GUEST_STATICLIB_CRATE_TYPE_<pkg>` override use
# `cargo rustc --crate-type <type>` (which can override `Cargo.toml`'s
# `crate-type`); the rest use the regular `cargo build`.
GUEST_STATICLIB_CARGO_BUILD = $(if $(GUEST_STATICLIB_CRATE_TYPE_$(1)),$(subst cargo build,cargo rustc,$(GUEST_CARGO_BUILD_CMD)) --lib --crate-type $(GUEST_STATICLIB_CRATE_TYPE_$(1)),$(GUEST_CARGO_BUILD_CMD))

# Per-package rules retained for direct invocation (e.g., make all-guest-staticlib-<pkg>).
define GUEST_STATICLIB_RULES
all-guest-staticlib-$(1): init
	$(call GUEST_STATICLIB_CARGO_BUILD,$(1)) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$(call guest_staticlib_artifact,$(1)) $(LIBRARIES_DIR)/$(call guest_staticlib_staged,$(1))

check-guest-staticlib-$(1):
	@$(GUEST_CARGO_CHECK_CMD) -p $(1)
	@$(GUEST_CARGO_CHECK_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))

format-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-staticlib-$(1):
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(LIBRARIES_DIR)/$(call guest_staticlib_staged,$(1))

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
	$(call GUEST_STATICLIB_CARGO_BUILD,$(1)) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))
endef

define _OVERRIDE_CHECK_CMDS
$(GUEST_CARGO_CHECK_CMD) -p $(1) && $(GUEST_CARGO_CHECK_CMD) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))
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
		src=lib$$(echo $$pkg | sed 's/-/_/g').a; \
		case "$$pkg" in \
			nanvix_libc) dst=libc.a;; \
			nanvix_libm) dst=libm.a;; \
			*) dst=$$src;; \
		esac; \
		$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$$src $(LIBRARIES_DIR)/$$dst; \
	done

check-guest-staticlibs:
	$(if $(_GUEST_STATICLIB_PKGS_DEFAULT),@$(GUEST_CARGO_CHECK_CMD) $(_GUEST_STATICLIB_PKGS_DEFAULT))
	$(if $(_GUEST_STATICLIB_PKGS_DEFAULT),@$(GUEST_CARGO_CHECK_CMD) $(_GUEST_STATICLIB_PKGS_DEFAULT) $(GUEST_STATICLIB_CARGO_FEATURES))
	@$(foreach pkg,$(_GUEST_STATICLIB_PKGS_OVERRIDE),$(call _OVERRIDE_CHECK_CMDS,$(pkg)) &&) true

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
