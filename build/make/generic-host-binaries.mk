# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

HOST_COMMON_FEATURES :=
HOST_COMMON_FEATURES := $(strip $(HOST_COMMON_FEATURES))

HOST_BINARIES_FEATURES.nanvix-bench += $(if $(filter yes,$(TIMESTAMP_MSG)),timestamp-messages,)
HOST_BINARIES_FEATURES.linuxd += $(if $(filter yes,$(TIMESTAMP_MSG)),timestamp-messages,)

host_binary_features = $(strip $(HOST_COMMON_FEATURES) $(HOST_BINARIES_FEATURES.$1))
host_cargo_features = $(if $1,--features "$1")

# Per-package rules used by feature-specific prerequisite targets and direct invocation.
define HOST_BINARY_RULES
all-host-binaries-$(1): init
	$(HOST_CARGO_BUILD_CMD) $(call host_cargo_features,$(call host_binary_features,$(1))) -p $(1)
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/$(1)$(CARGO_EXE_SUFFIX) $(BINARIES_DIR)/$(1).$(HOST_BIN_EXT)

check-host-binaries-$(1):
	@$(HOST_CARGO_CHECK_CMD) $(call host_cargo_features,$(call host_binary_features,$(1))) -p $(1)

format-host-binaries-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1)

format-check-host-binaries-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1) --check

clean-host-binaries-$(1):
	$(HOST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).$(HOST_BIN_EXT)

rust-lint-host-binaries-$(1):
	$(HOST_CARGO_CLIPPY_CMD) --tests $(call host_cargo_features,$(call host_binary_features,$(1))) -p $(1) --fix --allow-dirty --allow-no-vcs

rust-lint-check-host-binaries-$(1):
	$(HOST_CARGO_CLIPPY_CMD) --tests $(call host_cargo_features,$(call host_binary_features,$(1))) -p $(1) -- -D warnings
endef

$(foreach target,$(ALL_HOST_BINARIES),$(eval $(call HOST_BINARY_RULES,$(target))))

# Batched build/check/lint grouping: split host binaries by feature set.
# Binaries with package-specific features are handled individually.
# All remaining binaries are batched into a single cargo invocation.
_HOST_BINS_WITH_FEATURES := $(strip $(foreach pkg,$(ALL_HOST_BINARIES),$(if $(call host_binary_features,$(pkg)),$(pkg))))
_HOST_BINS_PLAIN := $(filter-out $(_HOST_BINS_WITH_FEATURES),$(ALL_HOST_BINARIES))
_HOST_BINS_PLAIN_PKGS := $(foreach pkg,$(_HOST_BINS_PLAIN),-p $(pkg))

# Batched build: group host binaries by feature set.
# GNU Make merges prerequisites from multiple rule headers for the same target;
# the first line adds feature-specific per-package builds as prerequisites
# (each per-package rule handles its own build and copy), the second provides
# the recipe that batches all featureless packages into a single cargo call.
all-host-binaries: $(foreach pkg,$(_HOST_BINS_WITH_FEATURES),all-host-binaries-$(pkg))
all-host-binaries: init
ifneq ($(_HOST_BINS_PLAIN_PKGS),)
	$(HOST_CARGO_BUILD_CMD) $(_HOST_BINS_PLAIN_PKGS)
	@for pkg in $(_HOST_BINS_PLAIN); do \
		$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/$$pkg$(CARGO_EXE_SUFFIX) $(BINARIES_DIR)/$$pkg.$(HOST_BIN_EXT); \
	done
endif

check-host-binaries: $(foreach pkg,$(_HOST_BINS_WITH_FEATURES),check-host-binaries-$(pkg))
ifneq ($(_HOST_BINS_PLAIN_PKGS),)
	@$(HOST_CARGO_CHECK_CMD) $(_HOST_BINS_PLAIN_PKGS)
endif

# Batched format: single cargo invocation for all host binaries.
_HOST_BINS_FMT_PKGS := $(foreach pkg,$(ALL_HOST_BINARIES),-p $(pkg))
format-host-binaries:
	$(HOST_CARGO_FMT_CMD) $(_HOST_BINS_FMT_PKGS)

format-check-host-binaries:
	$(HOST_CARGO_FMT_CMD) $(_HOST_BINS_FMT_PKGS) --check

clean-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),clean-host-binaries-$(target))

# Batched lint: group host binaries by feature set (same as check).
rust-lint-host-binaries: $(foreach pkg,$(_HOST_BINS_WITH_FEATURES),rust-lint-host-binaries-$(pkg))
ifneq ($(_HOST_BINS_PLAIN_PKGS),)
	$(HOST_CARGO_CLIPPY_CMD) --tests $(_HOST_BINS_PLAIN_PKGS) --fix --allow-dirty --allow-no-vcs
endif

rust-lint-check-host-binaries: $(foreach pkg,$(_HOST_BINS_WITH_FEATURES),rust-lint-check-host-binaries-$(pkg))
ifneq ($(_HOST_BINS_PLAIN_PKGS),)
	$(HOST_CARGO_CLIPPY_CMD) --tests $(_HOST_BINS_PLAIN_PKGS) -- -D warnings
endif
