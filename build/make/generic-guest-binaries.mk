# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

define GUEST_BINARY_RULES
all-guest-binaries-$(1): init all-guest-staticlibs
	$(GUEST_CARGO_BUILD_CMD) -p $(1) --features=$(LOG_LEVEL)
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$(1).elf $(BINARIES_DIR)/$(1).elf

check-guest-binaries-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1) --features=$(LOG_LEVEL)

format-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-binaries-$(1): clean-guest-staticlibs
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

rust-lint-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=$(LOG_LEVEL) --fix --allow-dirty

rust-lint-check-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=$(LOG_LEVEL)
endef

$(foreach target,$(ALL_GUEST_BINARIES),$(eval $(call GUEST_BINARY_RULES,$(target))))

all-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),all-guest-binaries-$(target))
	$(MAKE) -C $(SOURCES_DIR)/benchmarks all
	$(MAKE) -C $(SOURCES_DIR)/user all
	$(MAKE) -C $(SOURCES_DIR)/tests all

check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),check-guest-binaries-$(target))

format-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),format-guest-binaries-$(target))

format-check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),format-check-guest-binaries-$(target))

clean-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),clean-guest-binaries-$(target))
	$(MAKE) -C $(SOURCES_DIR)/benchmarks clean
	$(MAKE) -C $(SOURCES_DIR)/user clean
	$(MAKE) -C $(SOURCES_DIR)/tests clean

rust-lint-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),rust-lint-guest-binaries-$(target))

rust-lint-check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),rust-lint-check-guest-binaries-$(target))
