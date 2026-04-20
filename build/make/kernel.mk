# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

KERNEL_FEATURES := $(MACHINE) $(LOG_LEVEL)
KERNEL_FEATURES += $(if $(filter yes,$(WHP)),whp,)
KERNEL_FEATURES += $(if $(filter yes,$(RELEASE)),nightly-performance-optimizations,)
KERNEL_FEATURES := $(strip $(KERNEL_FEATURES))
KERNEL_CARGO_FEATURES := $(if $(KERNEL_FEATURES),--features "$(KERNEL_FEATURES)")

all-kernel: init
	$(KERNEL_CARGO_BUILD_CMD) $(KERNEL_CARGO_FEATURES) -p kernel
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-kernel/$(BUILD_MODE)/kernel.elf $(BINARIES_DIR)/kernel.elf

check-kernel:
	$(KERNEL_CARGO_CHECK_CMD) $(KERNEL_CARGO_FEATURES) -p kernel

format-kernel:
	$(KERNEL_CARGO_FMT_CMD) -p kernel

format-check-kernel:
	$(KERNEL_CARGO_FMT_CMD) -p kernel --check

clean-kernel:
	$(KERNEL_CARGO_CLEAN_CMD) -p kernel
	$(RM_CMD) $(BINARIES_DIR)/kernel.elf

rust-lint-kernel:
	$(KERNEL_CARGO_CLIPPY_CMD) $(KERNEL_CARGO_FEATURES) -p kernel --fix --allow-dirty --allow-no-vcs

rust-lint-check-kernel:
	$(KERNEL_CARGO_CLIPPY_CMD) $(KERNEL_CARGO_FEATURES) -p kernel -- -D warnings

# Features used when building the kernel with in-kernel tests enabled.
KERNEL_TEST_FEATURES := $(KERNEL_FEATURES) test
KERNEL_TEST_FEATURES := $(strip $(KERNEL_TEST_FEATURES))
KERNEL_TEST_CARGO_FEATURES := $(if $(KERNEL_TEST_FEATURES),--features "$(KERNEL_TEST_FEATURES)")

# Build the kernel with the `test` feature flag.
all-test-kernel: init
	$(KERNEL_CARGO_BUILD_CMD) $(KERNEL_TEST_CARGO_FEATURES) -p kernel
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-kernel/$(BUILD_MODE)/kernel.elf $(BINARIES_DIR)/kernel-test.elf

# Run in-kernel integration tests via the standalone UserVM. Boots the test-enabled kernel via
# uservm and waits for the kernel magic string ("hello, world!") to confirm tests passed and boot
# completed.
KERNEL_TEST_MAGIC_STRING := hello, world!
KERNEL_TEST_TIMEOUT := 120

run-kernel-tests: all-test-kernel all-uservm
	@echo "Running in-kernel integration tests..."
	$(PYTHON) scripts/run-uservm.py $(BINARIES_DIR)/kernel-test.elf $(KERNEL_TEST_TIMEOUT) \
		--wait-for-string "$(KERNEL_TEST_MAGIC_STRING)"

check-test-kernel:
	$(KERNEL_CARGO_CHECK_CMD) $(KERNEL_TEST_CARGO_FEATURES) -p kernel

clean-test-kernel:
	$(RM_CMD) $(BINARIES_DIR)/kernel-test.elf
