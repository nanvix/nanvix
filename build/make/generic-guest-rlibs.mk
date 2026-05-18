# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Per-package rules retained for direct invocation (e.g., make check-guest-rlib-<pkg>).
define GUEST_RLIB_RULES
check-guest-rlib-$(1):
	@$(GUEST_CARGO_CHECK_CMD) -p $(1)

format-guest-rlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-rlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

rust-lint-guest-rlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty --allow-no-vcs

rust-lint-check-guest-rlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) -- -D warnings
endef

$(foreach target,$(ALL_GUEST_RUST_LIBS),$(eval $(call GUEST_RLIB_RULES,$(target))))

# Guest rlib test code is compiled for the host target (not the custom guest
# target), so a separate host-side clippy pass with --tests is needed to lint
# #[cfg(test)] modules.
# Per-package rules retained for direct invocation.
define GUEST_RLIB_LINT_TEST_RULES
rust-lint-guest-rlib-tests-$(1):
	$(HOST_CARGO_CLIPPY_CMD) --tests --features=std -p $(1) --fix --allow-dirty --allow-no-vcs

rust-lint-check-guest-rlib-tests-$(1):
	$(HOST_CARGO_CLIPPY_CMD) --tests --features=std -p $(1) -- -D warnings
endef

$(foreach target,$(ALL_GUEST_RUST_LIBS_TEST_LIST),$(eval $(call GUEST_RLIB_LINT_TEST_RULES,$(target))))

# Batched targets: single cargo invocations for all guest rlibs.
_GUEST_RLIB_PKGS := $(foreach pkg,$(ALL_GUEST_RUST_LIBS),-p $(pkg))
_GUEST_RLIB_LINT_TEST_PKGS := $(foreach pkg,$(ALL_GUEST_RUST_LIBS_TEST_LIST),-p $(pkg))

check-guest-rlibs:
	@$(GUEST_CARGO_CHECK_CMD) $(_GUEST_RLIB_PKGS)

format-guest-rlibs:
	$(GUEST_CARGO_FMT_CMD) $(_GUEST_RLIB_PKGS)

format-check-guest-rlibs:
	$(GUEST_CARGO_FMT_CMD) $(_GUEST_RLIB_PKGS) --check

rust-lint-guest-rlibs:
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_RLIB_PKGS) --fix --allow-dirty --allow-no-vcs
	$(HOST_CARGO_CLIPPY_CMD) --tests --features=std $(_GUEST_RLIB_LINT_TEST_PKGS) --fix --allow-dirty --allow-no-vcs

rust-lint-check-guest-rlibs:
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_RLIB_PKGS) -- -D warnings
	$(HOST_CARGO_CLIPPY_CMD) --tests --features=std $(_GUEST_RLIB_LINT_TEST_PKGS) -- -D warnings

define GUEST_RLIB_TEST_RULES
test-guest-rlib-$(1):
	$(HOST_CARGO_TEST_CMD) --features=std -p $(1)
endef

$(foreach target,$(ALL_GUEST_RUST_LIBS_TEST_LIST),$(eval $(call GUEST_RLIB_TEST_RULES,$(target))))

test-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS_TEST_LIST),test-guest-rlib-$(target))
