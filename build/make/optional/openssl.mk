# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

CRYPTO_LIB := $(SYSROOT_DIR)/lib/libcrypto.a
OPENSSL_LIB := $(SYSROOT_DIR)/lib/libssl.a

all-openssl: $(OPENSSL_LIB)

$(OPENSSL_LIB): init-repo install
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@if [ ! -f $@ ]; then \
		echo "Building OpenSSL (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(OPENSSL_REPOSITORY) $(OPENSSL_COMMIT) openssl; \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building OpenSSL (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-opt.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(OPENSSL_REPOSITORY) $(OPENSSL_COMMIT) openssl; \
	else \
		echo "OpenSSL up-to-date!"; \
	fi
endif

clean-openssl:
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(OPENSSL_REPOSITORY) $(OPENSSL_COMMIT) openssl
	$(RM_CMD) $(OPENSSL_LIB) $(CRYPTO_LIB)
endif

init-openssl: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/build-opt.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR) $(OPENSSL_REPOSITORY) $(OPENSSL_COMMIT) openssl
endif
