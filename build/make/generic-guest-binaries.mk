# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

GUEST_BINARY_FEATURES := $(LOG_LEVEL)
GUEST_BINARY_FEATURES := $(strip $(GUEST_BINARY_FEATURES))
GUEST_BINARY_CARGO_FEATURES := $(if $(GUEST_BINARY_FEATURES),--features "$(GUEST_BINARY_FEATURES)")

# Package-specific features for test-kernel program.
TEST_KERNEL_FEATURES := $(GUEST_BINARY_FEATURES)
TEST_KERNEL_FEATURES += $(if $(filter hyperlight,$(MACHINE)),hyperlight,)
TEST_KERNEL_FEATURES := $(strip $(TEST_KERNEL_FEATURES))
TEST_KERNEL_CARGO_FEATURES := $(if $(TEST_KERNEL_FEATURES),--features "$(TEST_KERNEL_FEATURES)")

# Returns package-specific cargo features, falling back to generic features.
GUEST_BINARY_PKG_FEATURES = $(if $(filter test-kernel,$(1)),$(TEST_KERNEL_CARGO_FEATURES),$(GUEST_BINARY_CARGO_FEATURES))

define GUEST_BINARY_RULES
all-guest-binaries-$(1): init all-guest-staticlibs
	$(GUEST_CARGO_BUILD_CMD) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1))
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$(1).elf $(BINARIES_DIR)/$(1).elf

check-guest-binaries-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1))

format-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-binaries-$(1): clean-guest-staticlibs
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

rust-lint-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1)) --fix --allow-dirty

rust-lint-check-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(call GUEST_BINARY_PKG_FEATURES,$(1)) -- -D warnings
endef

$(foreach target,$(ALL_GUEST_BINARIES),$(eval $(call GUEST_BINARY_RULES,$(target))))

all-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),all-guest-binaries-$(target))
	$(MAKE_QUIET) -C $(SOURCES_DIR)/benchmarks all
	$(MAKE_QUIET) -C $(SOURCES_DIR)/user all
	$(MAKE_QUIET) -C $(SOURCES_DIR)/tests all

check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),check-guest-binaries-$(target))

format-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),format-guest-binaries-$(target))

format-check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),format-check-guest-binaries-$(target))

clean-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),clean-guest-binaries-$(target))
	$(MAKE_QUIET) -C $(SOURCES_DIR)/benchmarks clean
	$(MAKE_QUIET) -C $(SOURCES_DIR)/user clean
	$(MAKE_QUIET) -C $(SOURCES_DIR)/tests clean

rust-lint-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),rust-lint-guest-binaries-$(target))

rust-lint-check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),rust-lint-check-guest-binaries-$(target))
