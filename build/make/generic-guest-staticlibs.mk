# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

GUEST_STATICLIB_FEATURES := staticlib $(LOG_LEVEL)
# Enable standalone mode: routes stdout/stderr to debug kcall,
# file I/O to in-memory VFS, and disables IPC-based syscalls (no linuxd).
ifeq ($(DEPLOYMENT_MODE),standalone)
GUEST_STATICLIB_FEATURES += standalone
endif
GUEST_STATICLIB_FEATURES := $(strip $(GUEST_STATICLIB_FEATURES))
GUEST_STATICLIB_CARGO_FEATURES := $(if $(GUEST_STATICLIB_FEATURES),--features "$(GUEST_STATICLIB_FEATURES)")

# Per-package rules retained for direct invocation (e.g., make all-guest-staticlib-<pkg>).
define GUEST_STATICLIB_RULES
all-guest-staticlib-$(1): init
	$(GUEST_CARGO_BUILD_CMD) -p $(1) $(GUEST_STATICLIB_CARGO_FEATURES)
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/lib$(1).a $(LIBRARIES_DIR)/lib$(1).a

check-guest-staticlib-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1)
	$(GUEST_CARGO_CHECK_CMD) -p $(1) $(GUEST_STATICLIB_CARGO_FEATURES)

format-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-staticlib-$(1):
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(LIBRARIES_DIR)/lib$(1).a

rust-lint-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(GUEST_STATICLIB_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs

rust-lint-check-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) $(GUEST_STATICLIB_CARGO_FEATURES) -- -D warnings
endef

$(foreach target,$(ALL_GUEST_STATIC_LIBS),$(eval $(call GUEST_STATICLIB_RULES,$(target))))

# Batched targets: single cargo invocations for all guest staticlibs.
_GUEST_STATICLIB_PKGS := $(foreach pkg,$(ALL_GUEST_STATIC_LIBS),-p $(pkg))

all-guest-staticlibs: init
	$(GUEST_CARGO_BUILD_CMD) $(_GUEST_STATICLIB_PKGS) $(GUEST_STATICLIB_CARGO_FEATURES)
	@for pkg in $(ALL_GUEST_STATIC_LIBS); do \
		$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/lib$$pkg.a $(LIBRARIES_DIR)/lib$$pkg.a; \
	done

check-guest-staticlibs:
	$(GUEST_CARGO_CHECK_CMD) $(_GUEST_STATICLIB_PKGS)
	$(GUEST_CARGO_CHECK_CMD) $(_GUEST_STATICLIB_PKGS) $(GUEST_STATICLIB_CARGO_FEATURES)

format-guest-staticlibs:
	$(GUEST_CARGO_FMT_CMD) $(_GUEST_STATICLIB_PKGS)

format-check-guest-staticlibs:
	$(GUEST_CARGO_FMT_CMD) $(_GUEST_STATICLIB_PKGS) --check

clean-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),clean-guest-staticlib-$(target))

rust-lint-guest-staticlibs:
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_STATICLIB_PKGS) $(GUEST_STATICLIB_CARGO_FEATURES) --fix --allow-dirty --allow-no-vcs

rust-lint-check-guest-staticlibs:
	$(GUEST_CARGO_CLIPPY_CMD) $(_GUEST_STATICLIB_PKGS) $(GUEST_STATICLIB_CARGO_FEATURES) -- -D warnings
