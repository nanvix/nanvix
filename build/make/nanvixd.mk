# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

NANVIXD_FEATURES :=
NANVIXD_FEATURES += $(if $(filter standalone,$(DEPLOYMENT_MODE)),standalone,)
NANVIXD_FEATURES += $(if $(filter single-process,$(DEPLOYMENT_MODE)),single-process,)
NANVIXD_FEATURES += $(if $(filter multi-process l2,$(DEPLOYMENT_MODE)),multi-process,)
NANVIXD_FEATURES += $(if $(filter hyperlight,$(MACHINE)),hyperlight,)
NANVIXD_FEATURES += $(if $(filter microvm,$(MACHINE)),microvm,)
NANVIXD_FEATURES := $(strip $(NANVIXD_FEATURES))
NANVIXD_CARGO_FEATURES := $(if $(NANVIXD_FEATURES),--features "$(NANVIXD_FEATURES)")

# In standalone mode, nanvixd needs mkramfs to produce the rootfs image.
ifeq ($(DEPLOYMENT_MODE),standalone)
all-nanvixd: all-host-binaries-mkramfs
endif

all-nanvixd: init
	$(HOST_CARGO_BUILD_CMD) $(NANVIXD_CARGO_FEATURES) -p nanvixd
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/nanvixd $(BINARIES_DIR)/nanvixd.elf
	# Build the standalone rootfs image from a seed directory using mkramfs.
ifeq ($(DEPLOYMENT_MODE),standalone)
	@mkdir -p $(BINARIES_DIR)/standalone-rootfs-seed/lib
	@mkdir -p $(BINARIES_DIR)/standalone-rootfs-seed/src
	@cp -f $(ROOT_DIR)/README.md $(BINARIES_DIR)/standalone-rootfs-seed/
	@if [ -f $(LIBRARIES_DIR)/libmul.so ]; then \
		cp -f $(LIBRARIES_DIR)/libmul.so $(BINARIES_DIR)/standalone-rootfs-seed/lib/; \
	fi
	$(BINARIES_DIR)/mkramfs.elf -o $(BINARIES_DIR)/standalone-rootfs.img $(BINARIES_DIR)/standalone-rootfs-seed/
endif
	# Only give nanvixd CAP_SYS_ADMIN and CAP_NET_ADMIN if we need to manage
	# network namespaces. This is only the case in L2 deployments.
ifeq ($(DEPLOYMENT_MODE),l2)
	$(SUDO_CMD) $(SETCAP_CMD) cap_sys_admin,cap_net_admin+ep $(BINARIES_DIR)/nanvixd.elf
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
	$(HOST_CARGO_CLIPPY_CMD) --tests $(NANVIXD_CARGO_FEATURES) -p nanvixd --fix --allow-dirty --allow-no-vcs

rust-lint-check-nanvixd:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(NANVIXD_CARGO_FEATURES) -p nanvixd -- -D warnings
