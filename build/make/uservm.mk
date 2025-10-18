# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all-uservm: init
	$(HOST_CARGO_BUILD_CMD) $(USERVM_CARGO_FEATURES) -p uservm
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/uservm $(BINARIES_DIR)/uservm.elf

check-uservm:
	$(HOST_CARGO_CHECK_CMD) $(USERVM_CARGO_FEATURES) -p uservm

format-uservm:
	$(HOST_CARGO_FMT_CMD) -p uservm

format-check-uservm:
	$(HOST_CARGO_FMT_CMD) -p uservm --check

clean-uservm:
	$(HOST_CARGO_CLEAN_CMD) -p uservm
	$(RM_CMD) $(BINARIES_DIR)/uservm.elf

rust-lint-uservm:
	$(HOST_CARGO_CLIPPY_CMD) $(USERVM_CARGO_FEATURES) -p uservm --fix --allow-dirty

rust-lint-check-uservm:
	$(HOST_CARGO_CLIPPY_CMD) $(USERVM_CARGO_FEATURES) -p uservm

test-uservm:
	$(USERVM_CARGO_TEST_CMD) -p uservm
