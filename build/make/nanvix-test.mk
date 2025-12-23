# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

NANVIX_TEST_FEATURES :=
NANVIX_TEST_FEATURES += $(if $(filter yes,$(SINGLE_PROCESS)),single-process,)
NANVIX_TEST_FEATURES += $(if $(filter hyperlight,$(MACHINE)),hyperlight,)
NANVIX_TEST_FEATURES += $(if $(filter microvm,$(MACHINE)),microvm,)
NANVIX_TEST_FEATURES := $(strip $(NANVIX_TEST_FEATURES))
NANVIX_TEST_CARGO_FEATURES := $(if $(NANVIX_TEST_FEATURES),--features "$(NANVIX_TEST_FEATURES)")

all-nanvix-test: init
	$(HOST_CARGO_BUILD_CMD) $(NANVIX_TEST_CARGO_FEATURES) -p nanvix-test
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/nanvix-test $(BINARIES_DIR)/nanvix-test.elf

check-nanvix-test:
	$(HOST_CARGO_CHECK_CMD) $(NANVIX_TEST_CARGO_FEATURES) -p nanvix-test

format-nanvix-test:
	$(HOST_CARGO_FMT_CMD) -p nanvix-test

format-check-nanvix-test:
	$(HOST_CARGO_FMT_CMD) -p nanvix-test --check

clean-nanvix-test:
	$(HOST_CARGO_CLEAN_CMD) -p nanvix-test
	$(RM_CMD) $(BINARIES_DIR)/nanvix-test.elf

rust-lint-nanvix-test:
	$(HOST_CARGO_CLIPPY_CMD) $(NANVIX_TEST_CARGO_FEATURES) -p nanvix-test --fix --allow-dirty

rust-lint-check-nanvix-test:
	$(HOST_CARGO_CLIPPY_CMD) $(NANVIX_TEST_CARGO_FEATURES) -p nanvix-test -- -D warnings
