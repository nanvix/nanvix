# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

define GUEST_STATICLIB_RULES
all-guest-staticlib-$(1): init
	$(GUEST_CARGO_BUILD_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/lib$(1).a $(LIBRARIES_DIR)/lib$(1).a

check-guest-staticlib-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1)
	$(GUEST_CARGO_CHECK_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)

format-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-staticlib-$(1):
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(LIBRARIES_DIR)/lib$(1).a

rust-lint-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL) --fix --allow-dirty

rust-lint-check-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
endef

$(foreach target,$(ALL_GUEST_STATIC_LIBS),$(eval $(call GUEST_STATICLIB_RULES,$(target))))

all-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),all-guest-staticlib-$(target))

check-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),check-guest-staticlib-$(target))

format-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),format-guest-staticlib-$(target))

format-check-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),format-check-guest-staticlib-$(target))

clean-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),clean-guest-staticlib-$(target))

rust-lint-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),rust-lint-guest-staticlib-$(target))

rust-lint-check-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),rust-lint-check-guest-staticlib-$(target))
