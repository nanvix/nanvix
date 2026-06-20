# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

GUEST_BINARY_FEATURES := $(LOG_LEVEL)
GUEST_BINARY_FEATURES := $(strip $(GUEST_BINARY_FEATURES))
GUEST_BINARY_CARGO_FEATURES := $(if $(GUEST_BINARY_FEATURES),--features "$(GUEST_BINARY_FEATURES)")

# Package-specific features for test-kernel program.
TEST_KERNEL_FEATURES := $(GUEST_BINARY_FEATURES)
TEST_KERNEL_FEATURES := $(strip $(TEST_KERNEL_FEATURES))
TEST_KERNEL_CARGO_FEATURES := $(if $(TEST_KERNEL_FEATURES),--features "$(TEST_KERNEL_FEATURES)")

# Package-specific features for misc-rust program.
MISC_RUST_FEATURES := $(GUEST_BINARY_FEATURES)
MISC_RUST_FEATURES := $(strip $(MISC_RUST_FEATURES))
MISC_RUST_CARGO_FEATURES := $(if $(MISC_RUST_FEATURES),--features "$(MISC_RUST_FEATURES)")

# Guest binaries that support standalone deployment mode.
STANDALONE_GUEST_BINARIES := file-rust test-fork-guestfs test-fork-hostfs test-fork-kcall waitpid-rust setenv-rust linux-app thread-rust stress-rust arch-rust mount-test mount-multipart-test mount-bench-nostd cmdline-len-rust network-rust execv-test execv-target execv-big-target pipe-dup2-rust

# Computes the cargo features string for a guest binary package.
# test-kernel has its own overrides. When DEPLOYMENT_MODE=standalone, packages
# listed in STANDALONE_GUEST_BINARIES also get the 'standalone' cargo feature.
_STANDALONE_FEATURE := standalone
_standalone_feature = $(if $(and $(filter standalone,$(DEPLOYMENT_MODE)),$(filter $(STANDALONE_GUEST_BINARIES),$(1))),$(_STANDALONE_FEATURE))
_pkg_features = $(strip $(GUEST_BINARY_FEATURES) $(call _standalone_feature,$(1)))

# Returns package-specific cargo features, falling back to generic features.
GUEST_BINARY_PKG_FEATURES = $(if $(filter test-kernel,$(1)),$(TEST_KERNEL_CARGO_FEATURES),$(if $(filter misc-rust,$(1)),$(MISC_RUST_CARGO_FEATURES),$(if $(call _pkg_features,$(1)),--features "$(call _pkg_features,$(1))")))

# Per-package rules retained for direct invocation (e.g., make all-guest-binaries-<pkg>).
define GUEST_BINARY_RULES
all-guest-binaries-$(1): init all-guest-staticlibs
	$(GUEST_CARGO_BUILD_CMD) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1))
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$(1).elf $(BINARIES_DIR)/$(1).elf

check-guest-binaries-$(1):
	@$(GUEST_CARGO_CHECK_CMD) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1))

format-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-binaries-$(1): clean-guest-staticlibs
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

rust-lint-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1)) --fix --allow-dirty --allow-no-vcs

rust-lint-check-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1)) -- -D warnings
endef

$(foreach target,$(ALL_GUEST_BINARIES),$(eval $(call GUEST_BINARY_RULES,$(target))))

# Batched build/check/lint grouping: split guest binaries by feature set.
# - Regular: all except test-kernel and c-bindings-rust (and standalone-capable in standalone mode).
# - Standalone: standalone-capable binaries (only in standalone mode).
# - test-kernel: always separate (unique features).
# - c-bindings-rust: built separately to avoid Cargo feature unification masking
#   missing symbols (it validates that all expected C symbols link without
#   features contributed by sibling crates like network-rust).
_GUEST_BINS_COMMON := $(filter-out test-kernel c-bindings-rust,$(ALL_GUEST_BINARIES))

ifeq ($(DEPLOYMENT_MODE),standalone)
_GUEST_BINS_STANDALONE := $(filter $(STANDALONE_GUEST_BINARIES),$(_GUEST_BINS_COMMON))
_GUEST_BINS_REGULAR := $(filter-out $(STANDALONE_GUEST_BINARIES),$(_GUEST_BINS_COMMON))
_GUEST_BINS_STANDALONE_FEATURES := $(strip $(GUEST_BINARY_FEATURES) $(_STANDALONE_FEATURE))
_GUEST_BINS_STANDALONE_CARGO_FEATURES := $(if $(_GUEST_BINS_STANDALONE_FEATURES),--features "$(_GUEST_BINS_STANDALONE_FEATURES)")
else
_GUEST_BINS_REGULAR := $(_GUEST_BINS_COMMON)
_GUEST_BINS_STANDALONE :=
endif

_GUEST_BINS_REGULAR_PKGS := $(foreach pkg,$(_GUEST_BINS_REGULAR),-p $(pkg))
_GUEST_BINS_STANDALONE_PKGS := $(foreach pkg,$(_GUEST_BINS_STANDALONE),-p $(pkg))

# Batched build: group guest binaries by feature set, then copy all artifacts.
all-guest-binaries: init all-guest-staticlibs
ifneq ($(_GUEST_BINS_REGULAR_PKGS),)
	$(GUEST_CARGO_BUILD_CMD) $(_GUEST_BINS_REGULAR_PKGS) $(GUEST_BINARY_CARGO_FEATURES)
endif
ifneq ($(_GUEST_BINS_STANDALONE_PKGS),)
	$(GUEST_CARGO_BUILD_CMD) $(_GUEST_BINS_STANDALONE_PKGS) $(_GUEST_BINS_STANDALONE_CARGO_FEATURES)
endif
ifneq ($(filter test-kernel,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_BUILD_CMD) -p test-kernel $(TEST_KERNEL_CARGO_FEATURES)
endif
ifneq ($(filter c-bindings-rust,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_BUILD_CMD) -p c-bindings-rust $(GUEST_BINARY_CARGO_FEATURES)
endif
	@for pkg in $(ALL_GUEST_BINARIES); do \
		$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$$pkg.elf $(BINARIES_DIR)/$$pkg.elf; \
	done
# Copy side-artifact images produced by guest build scripts (e.g., vfs-test.img).
# The build script may be cached, so the copy it performs at build time is
# unreliable after a bin/ clean. Re-copy from the build output directory.
# Multiple stale build hash directories may exist; pick the most recently
# modified image to avoid copying an outdated artifact.
	@newest=$$(ls -t $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/build/vfs-test-*/out/test.img 2>/dev/null | head -n1); \
		if [ -n "$$newest" ]; then $(CP_CMD) "$$newest" $(BINARIES_DIR)/vfs-test.img; fi
	@newest=$$(ls -t $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/build/vfs-bench-nostd-*/out/$(VFS_BENCH_IMG) 2>/dev/null | head -n1); \
		if [ -n "$$newest" ]; then $(CP_CMD) "$$newest" $(BINARIES_DIR)/$(VFS_BENCH_IMG); fi
# Reset mount-test-data so that subsequent test runs always start with pristine input.
# A previous test execution may have modified files directly via hostfsd.
	@$(RM_CMD) -rf $(BINARIES_DIR)/mount-test-data
	@mkdir -p $(BINARIES_DIR)/mount-test-data/subdir
	@printf 'mount-test-input\n' > $(BINARIES_DIR)/mount-test-data/input.txt
	@printf 'nested-content\n' > $(BINARIES_DIR)/mount-test-data/subdir/nested.txt
# Reset mount-bench-data similarly.
	@$(RM_CMD) -rf $(BINARIES_DIR)/mount-bench-data
	@mkdir -p $(BINARIES_DIR)/mount-bench-data
	@dd if=/dev/zero bs=4096 count=1 2>/dev/null | tr '\0' '\253' > $(BINARIES_DIR)/mount-bench-data/bench-4k.bin

check-guest-binaries:
ifneq ($(_GUEST_BINS_REGULAR_PKGS),)
	@$(GUEST_CARGO_CHECK_CMD) $(_GUEST_BINS_REGULAR_PKGS) $(GUEST_BINARY_CARGO_FEATURES)
endif
ifneq ($(_GUEST_BINS_STANDALONE_PKGS),)
	@$(GUEST_CARGO_CHECK_CMD) $(_GUEST_BINS_STANDALONE_PKGS) $(_GUEST_BINS_STANDALONE_CARGO_FEATURES)
endif
ifneq ($(filter test-kernel,$(ALL_GUEST_BINARIES)),)
	@$(GUEST_CARGO_CHECK_CMD) -p test-kernel $(TEST_KERNEL_CARGO_FEATURES)
endif
ifneq ($(filter c-bindings-rust,$(ALL_GUEST_BINARIES)),)
	@$(GUEST_CARGO_CHECK_CMD) -p c-bindings-rust $(GUEST_BINARY_CARGO_FEATURES)
endif

# Batched format: single cargo invocation for all guest binaries.
_GUEST_BINS_FMT_PKGS := $(foreach pkg,$(ALL_GUEST_BINARIES),-p $(pkg))
format-guest-binaries:
	$(GUEST_CARGO_FMT_CMD) $(_GUEST_BINS_FMT_PKGS)

format-check-guest-binaries:
	$(GUEST_CARGO_FMT_CMD) $(_GUEST_BINS_FMT_PKGS) --check

clean-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),clean-guest-binaries-$(target))

# Batched lint: group guest binaries by feature set (same as check).
rust-lint-guest-binaries:
ifneq ($(_GUEST_BINS_REGULAR_PKGS),)
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_BINS_REGULAR_PKGS) $(GUEST_BINARY_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs
endif
ifneq ($(_GUEST_BINS_STANDALONE_PKGS),)
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_BINS_STANDALONE_PKGS) $(_GUEST_BINS_STANDALONE_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs
endif
ifneq ($(filter test-kernel,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_CLIPPY_CMD) -p test-kernel $(TEST_KERNEL_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs
endif
ifneq ($(filter c-bindings-rust,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_CLIPPY_CMD) -p c-bindings-rust $(GUEST_BINARY_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs
endif

rust-lint-check-guest-binaries:
ifneq ($(_GUEST_BINS_REGULAR_PKGS),)
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_BINS_REGULAR_PKGS) $(GUEST_BINARY_CARGO_FEATURES) -- -D warnings
endif
ifneq ($(_GUEST_BINS_STANDALONE_PKGS),)
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_BINS_STANDALONE_PKGS) $(_GUEST_BINS_STANDALONE_CARGO_FEATURES) -- -D warnings
endif
ifneq ($(filter test-kernel,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_CLIPPY_CMD) -p test-kernel $(TEST_KERNEL_CARGO_FEATURES) -- -D warnings
endif
ifneq ($(filter c-bindings-rust,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_CLIPPY_CMD) -p c-bindings-rust $(GUEST_BINARY_CARGO_FEATURES) -- -D warnings
endif
