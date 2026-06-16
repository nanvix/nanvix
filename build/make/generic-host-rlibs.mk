# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Host rlibs that depend on uservm and require machine + deployment features.
USERVM_DEPENDENT_RLIBS := nanvix nanvix-sandbox nanvix-http nanvix-terminal

# Host rlibs that only support multi-process deployment features.
MULTI_PROCESS_ONLY_RLIBS := nanvix-sandbox-cache

# Machine-specific feature flags for host rlibs that depend on uservm.
MACHINE_FEATURES :=
MACHINE_FEATURES += $(if $(filter microvm,$(MACHINE)),microvm,)
MACHINE_FEATURES += $(if $(filter yes,$(WHP)),whp,)
MACHINE_FEATURES := $(strip $(MACHINE_FEATURES))

# Deployment-mode feature flags for host rlibs that depend on uservm.
DEPLOYMENT_FEATURES :=
DEPLOYMENT_FEATURES += $(if $(filter standalone,$(DEPLOYMENT_MODE)),standalone,)
DEPLOYMENT_FEATURES += $(if $(filter single-process,$(DEPLOYMENT_MODE)),single-process,)
DEPLOYMENT_FEATURES += $(if $(filter multi-process l2,$(DEPLOYMENT_MODE)),multi-process,)
DEPLOYMENT_FEATURES := $(strip $(DEPLOYMENT_FEATURES))

# Multi-process-only deployment features (for crates that do not support single-process).
MULTI_PROCESS_DEPLOYMENT_FEATURES :=
MULTI_PROCESS_DEPLOYMENT_FEATURES += $(if $(filter multi-process l2,$(DEPLOYMENT_MODE)),multi-process,)
MULTI_PROCESS_DEPLOYMENT_FEATURES := $(strip $(MULTI_PROCESS_DEPLOYMENT_FEATURES))

ALL_HOST_RLIB_FEATURES = $(strip $(MACHINE_FEATURES) $(DEPLOYMENT_FEATURES))
ALL_HOST_RLIB_CARGO_FEATURES = $(if $(ALL_HOST_RLIB_FEATURES),--features "$(ALL_HOST_RLIB_FEATURES)",)

MULTI_PROCESS_RLIB_FEATURES = $(strip $(MACHINE_FEATURES) multi-process)
MULTI_PROCESS_RLIB_CARGO_FEATURES = $(if $(MULTI_PROCESS_RLIB_FEATURES),--features "$(MULTI_PROCESS_RLIB_FEATURES)",)

# Resolve Cargo features for a given host rlib:
#  - Crates in USERVM_DEPENDENT_RLIBS get all deployment + machine features.
#  - Crates in MULTI_PROCESS_ONLY_RLIBS get only multi-process + machine features.
#  - All other crates get no extra features.
HOST_RLIBS_CARGO_FEATURES = $(if $(filter $(1),$(USERVM_DEPENDENT_RLIBS)),$(ALL_HOST_RLIB_CARGO_FEATURES),$(if $(filter $(1),$(MULTI_PROCESS_ONLY_RLIBS)),$(MULTI_PROCESS_RLIB_CARGO_FEATURES),))

# Per-package rules retained for direct invocation (e.g., make check-host-rlib-<pkg>).
define HOST_RLIB_RULES
check-host-rlib-$(1):
	@$(HOST_CARGO_CHECK_CMD) $(call HOST_RLIBS_CARGO_FEATURES,$(1)) -p $(1)

format-host-rlib-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1)

format-check-host-rlib-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1) --check

rust-lint-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) --tests $(call HOST_RLIBS_CARGO_FEATURES,$(1)) -p $(1) --fix --allow-dirty --allow-no-vcs

rust-lint-check-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) --tests $(call HOST_RLIBS_CARGO_FEATURES,$(1)) -p $(1) -- -D warnings

test-host-rlib-$(1):
	$(HOST_CARGO_TEST_CMD) $(call HOST_RLIBS_CARGO_FEATURES,$(1)) -p $(1)
endef

$(foreach target,$(ALL_HOST_RUST_LIBS),$(eval $(call HOST_RLIB_RULES,$(target))))

# Batched check: group host rlibs by feature set.
# - Plain: no extra features.
# - Uservm-dependent: machine + deployment features.
# - Multi-process-only: machine + multi-process features.
_HOST_RLIBS_PLAIN := $(filter-out $(USERVM_DEPENDENT_RLIBS) $(MULTI_PROCESS_ONLY_RLIBS),$(ALL_HOST_RUST_LIBS))
_HOST_RLIBS_USERVM := $(filter $(USERVM_DEPENDENT_RLIBS),$(ALL_HOST_RUST_LIBS))
_HOST_RLIBS_MULTI := $(filter $(MULTI_PROCESS_ONLY_RLIBS),$(ALL_HOST_RUST_LIBS))

_HOST_RLIBS_PLAIN_PKGS := $(foreach pkg,$(_HOST_RLIBS_PLAIN),-p $(pkg))
_HOST_RLIBS_USERVM_PKGS := $(foreach pkg,$(_HOST_RLIBS_USERVM),-p $(pkg))
_HOST_RLIBS_MULTI_PKGS := $(foreach pkg,$(_HOST_RLIBS_MULTI),-p $(pkg))

check-host-rlibs:
ifneq ($(_HOST_RLIBS_PLAIN_PKGS),)
	@$(HOST_CARGO_CHECK_CMD) $(_HOST_RLIBS_PLAIN_PKGS)
endif
ifneq ($(_HOST_RLIBS_USERVM_PKGS),)
	@$(HOST_CARGO_CHECK_CMD) $(ALL_HOST_RLIB_CARGO_FEATURES) $(_HOST_RLIBS_USERVM_PKGS)
endif
ifneq ($(_HOST_RLIBS_MULTI_PKGS),)
	@$(HOST_CARGO_CHECK_CMD) $(MULTI_PROCESS_RLIB_CARGO_FEATURES) $(_HOST_RLIBS_MULTI_PKGS)
endif

# Batched format: single cargo invocation for all host rlibs.
_HOST_RLIB_FMT_PKGS := $(foreach pkg,$(ALL_HOST_RUST_LIBS),-p $(pkg))
format-host-rlibs:
	$(HOST_CARGO_FMT_CMD) $(_HOST_RLIB_FMT_PKGS)

format-check-host-rlibs:
	$(HOST_CARGO_FMT_CMD) $(_HOST_RLIB_FMT_PKGS) --check

# Batched lint: group host rlibs by feature set (same groups as check).
rust-lint-host-rlibs:
ifneq ($(_HOST_RLIBS_PLAIN_PKGS),)
	$(HOST_CARGO_CLIPPY_CMD) --tests $(_HOST_RLIBS_PLAIN_PKGS) --fix --allow-dirty --allow-no-vcs
endif
ifneq ($(_HOST_RLIBS_USERVM_PKGS),)
	$(HOST_CARGO_CLIPPY_CMD) --tests $(ALL_HOST_RLIB_CARGO_FEATURES) $(_HOST_RLIBS_USERVM_PKGS) --fix --allow-dirty --allow-no-vcs
endif
ifneq ($(_HOST_RLIBS_MULTI_PKGS),)
	$(HOST_CARGO_CLIPPY_CMD) --tests $(MULTI_PROCESS_RLIB_CARGO_FEATURES) $(_HOST_RLIBS_MULTI_PKGS) --fix --allow-dirty --allow-no-vcs
endif

rust-lint-check-host-rlibs:
ifneq ($(_HOST_RLIBS_PLAIN_PKGS),)
	$(HOST_CARGO_CLIPPY_CMD) --tests $(_HOST_RLIBS_PLAIN_PKGS) -- -D warnings
endif
ifneq ($(_HOST_RLIBS_USERVM_PKGS),)
	$(HOST_CARGO_CLIPPY_CMD) --tests $(ALL_HOST_RLIB_CARGO_FEATURES) $(_HOST_RLIBS_USERVM_PKGS) -- -D warnings
endif
ifneq ($(_HOST_RLIBS_MULTI_PKGS),)
	$(HOST_CARGO_CLIPPY_CMD) --tests $(MULTI_PROCESS_RLIB_CARGO_FEATURES) $(_HOST_RLIBS_MULTI_PKGS) -- -D warnings
endif

test-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),test-host-rlib-$(target))
