# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

ZLIB_LIB := $(SYSROOT_DIR)/lib/libz.a

all-zlib: $(ZLIB_LIB)

$(ZLIB_LIB): init-repo install
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@if [ ! -f $@ ]; then \
		echo "Building ZLib (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(ZLIB_REPOSITORY) $(ZLIB_COMMIT) zlib; \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building ZLib (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(ZLIB_REPOSITORY) $(ZLIB_COMMIT) zlib; \
	else \
		echo "ZLib up-to-date!"; \
	fi
endif

clean-zlib:
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(ZLIB_REPOSITORY) $(ZLIB_COMMIT) zlib
	$(RM_CMD) $(ZLIB_LIB)
endif

init-zlib: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(ZLIB_REPOSITORY) $(ZLIB_COMMIT) zlib
endif
