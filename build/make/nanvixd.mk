# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

NANVIXD_FEATURES :=
NANVIXD_FEATURES += $(if $(filter yes,$(SINGLE_PROCESS)),single-process,)
NANVIXD_FEATURES += $(if $(filter hyperlight,$(MACHINE)),hyperlight,)
NANVIXD_FEATURES += $(if $(filter microvm,$(MACHINE)),microvm,)
NANVIXD_FEATURES := $(strip $(NANVIXD_FEATURES))
NANVIXD_CARGO_FEATURES := $(if $(NANVIXD_FEATURES),--features "$(NANVIXD_FEATURES)")

all-nanvixd: init
	$(HOST_CARGO_BUILD_CMD) $(NANVIXD_CARGO_FEATURES) -p nanvixd
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/nanvixd $(BINARIES_DIR)/nanvixd.elf

check-nanvixd:
	$(HOST_CARGO_CHECK_CMD) $(NANVIXD_CARGO_FEATURES) -p nanvixd

format-nanvixd:
	$(HOST_CARGO_FMT_CMD) -p nanvixd

format-check-nanvixd:
	$(HOST_CARGO_FMT_CMD) -p nanvixd --check

clean-nanvixd:
	$(HOST_CARGO_CLEAN_CMD) -p nanvixd
	$(RM_CMD) $(BINARIES_DIR)/nanvixd.elf

rust-lint-nanvixd:
	$(HOST_CARGO_CLIPPY_CMD) $(NANVIXD_CARGO_FEATURES) -p nanvixd --fix --allow-dirty

rust-lint-check-nanvixd:
	$(HOST_CARGO_CLIPPY_CMD) $(NANVIXD_CARGO_FEATURES) -p nanvixd -- -D warnings
