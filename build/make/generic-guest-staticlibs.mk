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
GUEST_STATICLIB_FEATURES_nvx-crt0 := c-main forwarding-allocator $(LOG_LEVEL)
GUEST_STATICLIB_FEATURES_nvx-crt0 := $(strip $(GUEST_STATICLIB_FEATURES_nvx-crt0))

# Returns the cargo `--features "..."` arg for the given package, falling
# back to the default $(GUEST_STATICLIB_CARGO_FEATURES) when no override is
# defined.
GUEST_STATICLIB_PKG_FEATURES = $(if $(GUEST_STATICLIB_FEATURES_$(1)),--features "$(GUEST_STATICLIB_FEATURES_$(1))",$(GUEST_STATICLIB_CARGO_FEATURES))

# Cargo crate names with hyphens produce artifacts with underscores (e.g.
# `nvx-crt0` -> `libnvx_crt0.a`). This helper normalises the package name
# for `cp` / `rm` paths against the cargo target dir.
guest_staticlib_artifact = lib$(subst -,_,$(1)).a

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
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$(call guest_staticlib_artifact,$(1)) $(LIBRARIES_DIR)/$(call guest_staticlib_artifact,$(1))

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
	$(call GUEST_STATICLIB_CARGO_BUILD,$(1)) -p $(1) $(call GUEST_STATICLIB_PKG_FEATURES,$(1))
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
		$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/lib$$(echo $$pkg | sed 's/-/_/g').a $(LIBRARIES_DIR)/lib$$(echo $$pkg | sed 's/-/_/g').a; \
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
