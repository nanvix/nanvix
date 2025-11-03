# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

define GUEST_RLIB_RULES
check-guest-rlib-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1)

format-guest-rlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-rlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

rust-lint-guest-rlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty

rust-lint-check-guest-rlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) -- -D warnings
endef

$(foreach target,$(ALL_GUEST_RUST_LIBS),$(eval $(call GUEST_RLIB_RULES,$(target))))

check-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),check-guest-rlib-$(target))

format-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),format-guest-rlib-$(target))

format-check-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),format-check-guest-rlib-$(target))

rust-lint-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),rust-lint-guest-rlib-$(target))

rust-lint-check-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),rust-lint-check-guest-rlib-$(target))

define GUEST_RLIB_TEST_RULES
test-guest-rlib-$(1):
	$(HOST_CARGO_TEST_CMD) --features=std -p $(1)
endef

$(foreach target,$(ALL_GUEST_RUST_LIBS_TEST_LIST),$(eval $(call GUEST_RLIB_TEST_RULES,$(target))))

test-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS_TEST_LIST),test-guest-rlib-$(target))
