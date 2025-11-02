# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

KERNEL_FEATURES := $(MACHINE) $(LOG_LEVEL)
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
	$(KERNEL_CARGO_CLIPPY_CMD) $(KERNEL_CARGO_FEATURES) -p kernel --fix --allow-dirty

rust-lint-check-kernel:
	$(KERNEL_CARGO_CLIPPY_CMD) $(KERNEL_CARGO_FEATURES) -p kernel -- -D warnings
