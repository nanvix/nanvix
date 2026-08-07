# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

GUEST_BINARY_FEATURES := $(LOG_LEVEL)
GUEST_BINARY_FEATURES := $(strip $(GUEST_BINARY_FEATURES))
GUEST_BINARY_CARGO_FEATURES := $(if $(GUEST_BINARY_FEATURES),--features "$(GUEST_BINARY_FEATURES)")

# Package-specific features for test-kernel program.
TEST_KERNEL_FEATURES := $(GUEST_BINARY_FEATURES)
TEST_KERNEL_FEATURES := $(strip $(TEST_KERNEL_FEATURES))
TEST_KERNEL_CARGO_FEATURES := $(if $(TEST_KERNEL_FEATURES),--features "$(TEST_KERNEL_FEATURES)")

# Package-specific features for test-rust-misc program.
MISC_RUST_FEATURES := $(GUEST_BINARY_FEATURES)
MISC_RUST_FEATURES := $(strip $(MISC_RUST_FEATURES))
MISC_RUST_CARGO_FEATURES := $(if $(MISC_RUST_FEATURES),--features "$(MISC_RUST_FEATURES)")

# Returns package-specific cargo features, falling back to generic features.
GUEST_BINARY_PKG_FEATURES = $(if $(filter test-rust-kernel,$(1)),$(TEST_KERNEL_CARGO_FEATURES),$(if $(filter test-rust-misc,$(1)),$(MISC_RUST_CARGO_FEATURES),$(GUEST_BINARY_CARGO_FEATURES)))

# test-rust-dlfcn must be a real PIE so its exported symbols populate the
# loader's global scope. On 64-bit targets, build it with the dedicated PIC
# target; other guest binaries retain the regular static relocation model.
GUEST_BINARY_USES_PIC = $(and $(filter x86_64 aarch64,$(TARGET)),$(filter test-rust-dlfcn,$(1)))
GUEST_BINARY_CARGO_BUILD = $(if $(call GUEST_BINARY_USES_PIC,$(1)),$(NANVIX_LIBC_PIC_CARGO_BUILD),$(GUEST_CARGO_BUILD_CMD))
GUEST_BINARY_CARGO_CHECK = $(if $(call GUEST_BINARY_USES_PIC,$(1)),$(GUEST_PIC_CARGO_CHECK_CMD),$(GUEST_CARGO_CHECK_CMD))
GUEST_BINARY_CARGO_CLEAN = $(if $(call GUEST_BINARY_USES_PIC,$(1)),$(GUEST_PIC_CARGO_CLEAN_CMD),$(GUEST_CARGO_CLEAN_CMD))
GUEST_BINARY_CARGO_CLIPPY = $(if $(call GUEST_BINARY_USES_PIC,$(1)),$(GUEST_PIC_CARGO_CLIPPY_CMD),$(GUEST_CARGO_CLIPPY_CMD))
GUEST_BINARY_OBJDIR = $(if $(call GUEST_BINARY_USES_PIC,$(1)),$(NANVIX_LIBC_PIC_OBJDIR),$(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE))

ifneq ($(filter $(TARGET),x86_64 aarch64),)
GUEST_PIC_CARGO_CHECK_CMD := RUSTFLAGS=$(NANVIX_LIBC_PIC_RUSTFLAGS) $(CARGO) check --locked \
	$(GUEST_CARGO_FLAGS) --target $(NANVIX_LIBC_PIC_TARGET) --message-format=json
GUEST_PIC_CARGO_CLEAN_CMD := RUSTFLAGS=$(NANVIX_LIBC_PIC_RUSTFLAGS) $(CARGO) clean \
	$(GUEST_CARGO_FLAGS) --target $(NANVIX_LIBC_PIC_TARGET)
GUEST_PIC_CARGO_CLIPPY_CMD := RUSTFLAGS=$(NANVIX_LIBC_PIC_RUSTFLAGS) $(CARGO) clippy --locked \
	$(GUEST_CARGO_FLAGS) --target $(NANVIX_LIBC_PIC_TARGET)
endif

# Per-package rules retained for direct invocation (e.g., make all-guest-binaries-<pkg>).
define GUEST_BINARY_RULES
all-guest-binaries-$(1): init all-guest-staticlibs
	$(call GUEST_BINARY_CARGO_BUILD,$(1)) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1))
	$(CP_CMD) $(call GUEST_BINARY_OBJDIR,$(1))/$(1).elf $(BINARIES_DIR)/$(1).elf

check-guest-binaries-$(1):
	@$(call GUEST_BINARY_CARGO_CHECK,$(1)) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1))

format-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-binaries-$(1): clean-guest-staticlibs
	$(call GUEST_BINARY_CARGO_CLEAN,$(1)) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

rust-lint-guest-binaries-$(1):
	$(call GUEST_BINARY_CARGO_CLIPPY,$(1)) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1)) --fix --allow-dirty --allow-no-vcs

rust-lint-check-guest-binaries-$(1):
	$(call GUEST_BINARY_CARGO_CLIPPY,$(1)) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1)) -- -D warnings
endef

$(foreach target,$(ALL_GUEST_BINARIES),$(eval $(call GUEST_BINARY_RULES,$(target))))

# Batched build/check/lint grouping.
# - Common: all except test-kernel, c-bindings-rust, and PIC dlfcn-rust.
# - test-kernel: always separate (unique features).
# - c-bindings-rust: built separately to avoid Cargo feature unification masking
#   missing symbols (it validates that all expected C symbols link without
#   features contributed by sibling crates like network-rust).
_GUEST_BINS_PIC := $(if $(filter x86_64 aarch64,$(TARGET)),$(filter test-rust-dlfcn,$(ALL_GUEST_BINARIES)))
_GUEST_BINS_STATIC := $(filter-out $(_GUEST_BINS_PIC),$(ALL_GUEST_BINARIES))
_GUEST_BINS_COMMON := $(filter-out test-rust-kernel test-rust-c-bindings,$(_GUEST_BINS_STATIC))
_GUEST_BINS_COMMON_PKGS := $(foreach pkg,$(_GUEST_BINS_COMMON),-p $(pkg))

# Batched build: group guest binaries by feature set, then copy all artifacts.
all-guest-binaries: init all-guest-staticlibs
ifneq ($(_GUEST_BINS_COMMON_PKGS),)
	$(GUEST_CARGO_BUILD_CMD) $(_GUEST_BINS_COMMON_PKGS) $(GUEST_BINARY_CARGO_FEATURES)
endif
ifneq ($(filter test-rust-kernel,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_BUILD_CMD) -p test-rust-kernel $(TEST_KERNEL_CARGO_FEATURES)
endif
ifneq ($(filter test-rust-c-bindings,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_BUILD_CMD) -p test-rust-c-bindings $(GUEST_BINARY_CARGO_FEATURES)
endif
ifneq ($(_GUEST_BINS_PIC),)
	$(NANVIX_LIBC_PIC_CARGO_BUILD) -p test-rust-dlfcn $(GUEST_BINARY_CARGO_FEATURES)
endif
	@for pkg in $(_GUEST_BINS_STATIC); do \
		$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$$pkg.elf $(BINARIES_DIR)/$$pkg.elf; \
	done
ifneq ($(_GUEST_BINS_PIC),)
	$(CP_CMD) $(NANVIX_LIBC_PIC_OBJDIR)/test-rust-dlfcn.elf $(BINARIES_DIR)/test-rust-dlfcn.elf
endif
# Copy side-artifact images produced by guest build scripts (e.g., vfs-test.img).
# The build script may be cached, so the copy it performs at build time is
# unreliable after a bin/ clean. Re-copy from the build output directory.
# Multiple stale build hash directories may exist; pick the most recently
# modified image to avoid copying an outdated artifact.
	@newest=$$(ls -t $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/build/test-rust-vfs-test-*/out/test.img 2>/dev/null | head -n1); \
		if [ -n "$$newest" ]; then $(CP_CMD) "$$newest" $(BINARIES_DIR)/test-rust-vfs-test.img; fi
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
ifneq ($(_GUEST_BINS_COMMON_PKGS),)
	@$(GUEST_CARGO_CHECK_CMD) $(_GUEST_BINS_COMMON_PKGS) $(GUEST_BINARY_CARGO_FEATURES)
endif
ifneq ($(filter test-rust-kernel,$(ALL_GUEST_BINARIES)),)
	@$(GUEST_CARGO_CHECK_CMD) -p test-rust-kernel $(TEST_KERNEL_CARGO_FEATURES)
endif
ifneq ($(filter test-rust-c-bindings,$(ALL_GUEST_BINARIES)),)
	@$(GUEST_CARGO_CHECK_CMD) -p test-rust-c-bindings $(GUEST_BINARY_CARGO_FEATURES)
endif
ifneq ($(_GUEST_BINS_PIC),)
	@$(GUEST_PIC_CARGO_CHECK_CMD) -p test-rust-dlfcn $(GUEST_BINARY_CARGO_FEATURES)
endif

# Batched format: single cargo invocation for all guest binaries.
_GUEST_BINS_FMT_PKGS := $(foreach pkg,$(ALL_GUEST_BINARIES),-p $(pkg))
format-guest-binaries:
	$(GUEST_CARGO_FMT_CMD) $(_GUEST_BINS_FMT_PKGS)

format-check-guest-binaries:
	$(GUEST_CARGO_FMT_CMD) $(_GUEST_BINS_FMT_PKGS) --check

clean-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),clean-guest-binaries-$(target))

# Batched lint for guest binaries.
rust-lint-guest-binaries:
ifneq ($(_GUEST_BINS_COMMON_PKGS),)
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_BINS_COMMON_PKGS) $(GUEST_BINARY_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs
endif
ifneq ($(filter test-rust-kernel,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_CLIPPY_CMD) -p test-rust-kernel $(TEST_KERNEL_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs
endif
ifneq ($(filter test-rust-c-bindings,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_CLIPPY_CMD) -p test-rust-c-bindings $(GUEST_BINARY_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs
endif
ifneq ($(_GUEST_BINS_PIC),)
	$(GUEST_PIC_CARGO_CLIPPY_CMD) -p test-rust-dlfcn $(GUEST_BINARY_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs
endif

rust-lint-check-guest-binaries:
ifneq ($(_GUEST_BINS_COMMON_PKGS),)
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_BINS_COMMON_PKGS) $(GUEST_BINARY_CARGO_FEATURES) -- -D warnings
endif
ifneq ($(filter test-rust-kernel,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_CLIPPY_CMD) -p test-rust-kernel $(TEST_KERNEL_CARGO_FEATURES) -- -D warnings
endif
ifneq ($(filter test-rust-c-bindings,$(ALL_GUEST_BINARIES)),)
	$(GUEST_CARGO_CLIPPY_CMD) -p test-rust-c-bindings $(GUEST_BINARY_CARGO_FEATURES) -- -D warnings
endif
ifneq ($(_GUEST_BINS_PIC),)
	$(GUEST_PIC_CARGO_CLIPPY_CMD) -p test-rust-dlfcn $(GUEST_BINARY_CARGO_FEATURES) -- -D warnings
endif
