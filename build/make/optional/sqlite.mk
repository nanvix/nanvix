# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

SQLITE_LIB := $(SYSROOT_DIR)/lib/libsqlite3.a

all-sqlite: $(SQLITE_LIB)

$(SQLITE_LIB): init-repo install all-zlib
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@if [ ! -f $@ ]; then \
		echo "Building SQLite (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(SQLITE_REPOSITORY) $(SQLITE_COMMIT) sqlite; \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building SQLite (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(SQLITE_REPOSITORY) $(SQLITE_COMMIT) sqlite; \
	else \
		echo "SQLite up-to-date!"; \
	fi
endif

clean-sqlite: clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(SQLITE_REPOSITORY) $(SQLITE_COMMIT) sqlite
	$(RM_CMD) $(SQLITE_LIB)
endif

init-sqlite: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(SQLITE_REPOSITORY) $(SQLITE_COMMIT) sqlite
endif
