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
	# Only give nanvixd CAP_SYS_ADMIN and CAP_NET_ADMIN if we need to manage
	# network namespaces. This is only the case in L2 (multi-process) deployments.
ifeq ($(SINGLE_PROCESS),no)
ifeq ($(L2_VM),yes)
	$(SUDO_CMD) $(SETCAP_CMD) cap_sys_admin,cap_net_admin+ep $(BINARIES_DIR)/nanvixd.elf
endif
endif

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
