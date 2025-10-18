# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

PYTHON_LIB := $(SYSROOT_DIR)/lib/libpython3.12.a

all-python: $(PYTHON_LIB)

$(PYTHON_LIB): init-repo install all-openssl all-sqlite all-zlib
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@if [ ! -f $@ ]; then \
		echo "Building Python (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(PYTHON_REPOSITORY) $(PYTHON_COMMIT) cpython; \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building Python (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(PYTHON_REPOSITORY) $(PYTHON_COMMIT) cpython; \
	else \
		echo "Python up-to-date!"; \
	fi
endif

clean-python: clean-sqlite clean-openssl clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(PYTHON_REPOSITORY) $(PYTHON_COMMIT) cpython
	$(RM_CMD) $(PYTHON_LIB)
endif

init-python: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(PYTHON_REPOSITORY) $(PYTHON_COMMIT) cpython
endif
