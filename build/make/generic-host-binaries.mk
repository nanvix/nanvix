# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

HOST_FEATURES :=
HOST_FEATURES += $(if $(filter yes,$(TIMESTAMP_MSG)),timestamp-messages,)
HOST_FEATURES := $(strip $(HOST_FEATURES))
HOST_CARGO_FEATURES := $(if $(HOST_FEATURES),--features "$(HOST_FEATURES)")

define HOST_BINARY_RULES
all-host-binaries-$(1): init
	$(HOST_CARGO_BUILD_CMD) $(HOST_CARGO_FEATURES) -p $(1)
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/$(1) $(BINARIES_DIR)/$(1).elf

check-host-binaries-$(1):
	$(HOST_CARGO_CHECK_CMD) $(HOST_CARGO_FEATURES) -p $(1)

format-host-binaries-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1)

format-check-host-binaries-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1) --check

clean-host-binaries-$(1):
	$(HOST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

rust-lint-host-binaries-$(1):
	$(HOST_CARGO_CLIPPY_CMD) $(HOST_CARGO_FEATURES) -p $(1) --fix --allow-dirty

rust-lint-check-host-binaries-$(1):
	$(HOST_CARGO_CLIPPY_CMD) $(HOST_CARGO_FEATURES) -p $(1)
endef

$(foreach target,$(ALL_HOST_BINARIES),$(eval $(call HOST_BINARY_RULES,$(target))))

all-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),all-host-binaries-$(target))

check-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),check-host-binaries-$(target))

format-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),format-host-binaries-$(target))

format-check-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),format-check-host-binaries-$(target))

clean-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),clean-host-binaries-$(target))

rust-lint-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),rust-lint-host-binaries-$(target))

rust-lint-check-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),rust-lint-check-host-binaries-$(target))
