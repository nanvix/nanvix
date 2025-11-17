# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Host rlibs that depend on uservm and require machine-specific features.
USERVM_DEPENDENT_RLIBS := nanvix nanvix-sandbox nanvix-http nanvix-terminal nanvix-sandbox-cache

# Machine-specific feature flags for host rlibs that depend on uservm.
MACHINE_FEATURES :=
MACHINE_FEATURES += $(if $(filter hyperlight,$(MACHINE)),hyperlight,)
MACHINE_FEATURES += $(if $(filter microvm,$(MACHINE)),microvm,)
MACHINE_FEATURES := $(strip $(MACHINE_FEATURES))
MACHINE_CARGO_FEATURES := $(if $(MACHINE_FEATURES),--features "$(MACHINE_FEATURES)",)
HOST_RLIBS_CARGO_FEATURES = $(if $(filter $(1),$(USERVM_DEPENDENT_RLIBS)),$(MACHINE_CARGO_FEATURES),)

define HOST_RLIB_RULES
check-host-rlib-$(1):
	$(HOST_CARGO_CHECK_CMD) $(call HOST_RLIBS_CARGO_FEATURES,$(1)) -p $(1)

format-host-rlib-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1)

format-check-host-rlib-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1) --check

rust-lint-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) $(call HOST_RLIBS_CARGO_FEATURES,$(1)) -p $(1) --fix --allow-dirty

rust-lint-check-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) $(call HOST_RLIBS_CARGO_FEATURES,$(1)) -p $(1) -- -D warnings

test-host-rlib-$(1):
	$(HOST_CARGO_TEST_CMD) $(call HOST_RLIBS_CARGO_FEATURES,$(1)) -p $(1)
endef

$(foreach target,$(ALL_HOST_RUST_LIBS),$(eval $(call HOST_RLIB_RULES,$(target))))

check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),check-host-rlib-$(target))

format-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),format-host-rlib-$(target))

format-check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),format-check-host-rlib-$(target))

rust-lint-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),rust-lint-host-rlib-$(target))

rust-lint-check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),rust-lint-check-host-rlib-$(target))

test-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),test-host-rlib-$(target))
