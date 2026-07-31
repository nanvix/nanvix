# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Dynamic-Linking Test Libraries
#===================================================================================================
#
# Builds the libmul shared-library fixtures used by the Rust and POSIX dlfcn
# integration suites. The fixtures follow the active guest ABI and are staged
# in lib/ for inclusion in the standalone and POSIX RAMFS images.

DLFCN_TEST_LIB_SOURCE := $(SOURCES_DIR)/tests/integration/test-rust-dlfcn/libs/mul.c
DLFCN_TEST_LIB_OBJDIR := $(OBJECTS_DIR)/dlfcn-test-libs/$(TARGET)
DLFCN_TEST_LIB_OBJECT := $(DLFCN_TEST_LIB_OBJDIR)/mul.o
DLFCN_TEST_LIB_SO := $(DLFCN_TEST_LIB_OBJDIR)/libmul.so
DLFCN_TEST_LIB_PIE_SO := $(DLFCN_TEST_LIB_OBJDIR)/libmul-pie.so

ifeq ($(TARGET),x86_64)
DLFCN_TEST_LIB_CFLAGS := -m64 -march=x86-64 -mno-red-zone
else ifeq ($(TARGET),aarch64)
DLFCN_TEST_LIB_CFLAGS := -march=armv8-a
else
DLFCN_TEST_LIB_CFLAGS := -m32 -march=pentiumpro
endif
DLFCN_TEST_LIB_CFLAGS += -nostdlib -ffreestanding -fPIC -fno-builtin \
	-fno-stack-protector -O0 -std=c17
DLFCN_TEST_LIB_LDFLAGS := -shared -m$(NANVIX_LIBC_ELF_EMULATION) -z notext \
	--hash-style=sysv

$(DLFCN_TEST_LIB_OBJECT): $(DLFCN_TEST_LIB_SOURCE)
	@command -v $(firstword $(GUEST_C_APP_CC)) >/dev/null 2>&1 || { \
		echo "ERROR: dlfcn tests need '$(firstword $(GUEST_C_APP_CC))' on PATH to build libmul."; \
		exit 1; \
	}
	@$(MKDIR_CMD) $(dir $@)
	@echo "[dlfcn-test] compiling libmul for $(TARGET)"
	$(GUEST_C_APP_CC) $(DLFCN_TEST_LIB_CFLAGS) -c $< -o $@

$(DLFCN_TEST_LIB_SO): $(DLFCN_TEST_LIB_OBJECT)
	@echo "[dlfcn-test] linking libmul.so for $(TARGET)"
	$(NANVIX_LIBC_LD) $(DLFCN_TEST_LIB_LDFLAGS) $< -o $@

# Both test paths intentionally load the same shared object under different
# names. The second name distinguishes the executable-mode test case.
$(DLFCN_TEST_LIB_PIE_SO): $(DLFCN_TEST_LIB_SO)
	$(CP_CMD) $< $@

.PHONY: all-dlfcn-test-libs clean-dlfcn-test-libs

all-dlfcn-test-libs: $(DLFCN_TEST_LIB_SO) $(DLFCN_TEST_LIB_PIE_SO)
	@$(MKDIR_CMD) $(LIBRARIES_DIR)
	$(CP_CMD) $(DLFCN_TEST_LIB_SO) $(LIBRARIES_DIR)/libmul.so
	$(CP_CMD) $(DLFCN_TEST_LIB_PIE_SO) $(LIBRARIES_DIR)/libmul-pie.so

clean-dlfcn-test-libs:
	$(FORCE_RM_CMD) $(OBJECTS_DIR)/dlfcn-test-libs
	$(RM_CMD) $(LIBRARIES_DIR)/libmul.so $(LIBRARIES_DIR)/libmul-pie.so

# Direct Rust dlfcn builds and the test RAMFS images consume the staged fixtures.
# Keep this dependency off all-guest-binaries: production and benchmark builds do
# not package the fixtures and should not require a guest C compiler.
all-guest-binaries-test-rust-dlfcn: all-dlfcn-test-libs
clean: clean-dlfcn-test-libs
