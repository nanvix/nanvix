# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

OPENBLAS_LIB := $(SYSROOT_DIR)/lib/libopenblas.a

all-openblas: $(OPENBLAS_LIB)

$(OPENBLAS_LIB): init-repo install
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@if [ ! -f $@ ]; then \
		echo "Building OpenBLAS (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(OPENBLAS_REPOSITORY) $(OPENBLAS_COMMIT) openblas; \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building OpenBLAS (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(OPENBLAS_REPOSITORY) $(OPENBLAS_COMMIT) openblas; \
	else \
		echo "OpenBLAS up-to-date!"; \
	fi
endif

clean-openblas: clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(OPENBLAS_REPOSITORY) $(OPENBLAS_COMMIT) openblas
	$(RM_CMD) $(OPENBLAS_LIB)
endif

init-openblas: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(OPENBLAS_REPOSITORY) $(OPENBLAS_COMMIT) openblas
endif
