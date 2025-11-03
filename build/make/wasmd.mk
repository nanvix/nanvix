# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all-wasmd: all-wasm-binaries all-guest-binaries
	@echo "WASM_BINARY=$(WASM_BINARY)"
ifneq ($(WASM_BINARY),)
	$(eval export NANVIX_WASM_BINARY := $(realpath $(WASM_BINARY)))
	$(eval export NANVIX_WASM_BINARY_BASENAME := $(shell basename $(NANVIX_WASM_BINARY)))
	$(eval export NANVIX_WASM_BINARY_ARGS := =$(WASM_BINARY_ARGS))
	$(eval export WASMD_CARGO_FEATURES := --features wasm_binary)
endif
	@echo "NANVIX_WASM_BINARY=$(NANVIX_WASM_BINARY)"
	@echo "NANVIX_WASM_BINARY_BASENAME=$(NANVIX_WASM_BINARY_BASENAME)"
	@echo "NANVIX_WASM_BINARY_ARGS=$(NANVIX_WASM_BINARY_ARGS)"
	$(GUEST_CARGO_BUILD_CMD) $(WASMD_CARGO_FEATURES) -p wasmd
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/wasmd.elf $(BINARIES_DIR)/wasmd.elf

check-wasmd:
	$(GUEST_CARGO_CHECK_CMD) -p wasmd

format-wasmd:
	$(GUEST_CARGO_FMT_CMD) -p wasmd

format-check-wasmd:
	$(GUEST_CARGO_FMT_CMD) -p wasmd --check

clean-wasmd: clean-wasm-binaries clean-guest-binaries
	$(GUEST_CARGO_CLEAN_CMD) -p wasmd
	$(RM_CMD) $(BINARIES_DIR)/wasmd.elf

rust-lint-wasmd:
	$(GUEST_CARGO_CLIPPY_CMD) -p wasmd --fix --allow-dirty

rust-lint-check-wasmd:
	$(GUEST_CARGO_CLIPPY_CMD) -p wasmd -- -D warnings
