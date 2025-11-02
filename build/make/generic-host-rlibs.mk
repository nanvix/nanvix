# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

define HOST_RLIB_RULES
check-host-rlib-$(1):
	$(HOST_CARGO_CHECK_CMD) -p $(1)

format-host-rlib-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1)

format-check-host-rlib-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1) --check

rust-lint-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty

rust-lint-check-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) -p $(1) -- -D warnings

test-host-rlib-$(1):
	$(HOST_CARGO_TEST_CMD) -p $(1)
endef

$(foreach target,$(ALL_HOST_RUST_LIBS),$(eval $(call HOST_RLIB_RULES,$(target))))

check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),check-host-rlib-$(target))

format-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),format-host-rlib-$(target))

format-check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),format-check-host-rlib-$(target))

rust-lint-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),rust-lint-host-rlib-$(target))

rust-lint-check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),rust-lint-check-host-rlib-$(target))

test-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),test-host-rlib-$(target))
