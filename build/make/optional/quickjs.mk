# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

QUICKJS_LIB := $(SYSROOT_DIR)/lib/libquickjs.a

all-quickjs: $(QUICKJS_LIB)

$(QUICKJS_LIB): init-repo install
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@if [ ! -f $@ ]; then \
		echo "Building QuickJS (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(QUICKJS_REPOSITORY) $(QUICKJS_COMMIT) quickjs; \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building QuickJS (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(QUICKJS_REPOSITORY) $(QUICKJS_COMMIT) quickjs; \
	else \
		echo "QuickJS up-to-date!"; \
	fi
endif

clean-quickjs:
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(QUICKJS_REPOSITORY) $(QUICKJS_COMMIT) quickjs
	$(RM_CMD) $(QUICKJS_LIB)
endif

init-quickjs: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(QUICKJS_REPOSITORY) $(QUICKJS_COMMIT) quickjs
endif
