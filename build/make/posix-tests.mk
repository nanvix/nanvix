# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Ported POSIX C Test Suites (built against the bundled Nanvix libc)
#===================================================================================================
#
# Builds the C test suites ported from `nanvix/posix-tests` and runs them under
# nanvixd in standalone mode (nanvixd drives the UserVM). Each suite lives at
# `src/posix-tests/<suite>/` (one or
# more `*.c` files), is compiled with the host C toolchain against the in-tree
# headers in `include/`, and linked against the merged `libc.a` (the C library +
# the Nanvix system-call backend, produced by `nanvix-libc-bundle`) — exactly
# like `build/make/guest-c-apps.mk`, whose compiler/flag definitions this fragment
# reuses.
#
# The suites build against the bundled libc; the boot runner
# (`run-posix-tests`) is gated on DEPLOYMENT_MODE=standalone, the only mode it
# supports.
#
# Like the other guest test binaries, the suites are never shipped in releases:
# `install`/`release` copy only the kernel, daemons, libraries, and host tools.
#
# Pass/fail convention (matches the test-kernel exit-code signal): a suite passes
# when its `main()` returns 0, which nanvixd propagates as its own exit code.
# A failed `assert()` aborts (SIGABRT -> exit 134) or a non-zero return marks a
# failure. Guest stdout is discarded in standalone mode, so the exit code
# is authoritative.

#---------------------------------------------------------------------------------------------------
# Inputs and flags (reused from guest-c-apps.mk).
#---------------------------------------------------------------------------------------------------

POSIX_TESTS_SRCDIR := $(SOURCES_DIR)/posix-tests
POSIX_TESTS_OBJDIR := $(OBJECTS_DIR)/posix-tests

# Shared, weak `_init`/`_fini` glue, retained only as harmless legacy scaffolding.
# After doc/toolchain-migration.md §4.5 (decision 1), the c-main startup
# (nvx-crt0's __nanvix_main) NO LONGER calls `_init`/`_fini`: constructors and
# destructors run via `.init_array`/`.fini_array`, walked by
# `__nanvix_libc_start_main` (see src/libs/posix/src/start.rs) using the bounds
# the guest `user.ld` provides. These weak stubs therefore resolve nothing and
# are unreferenced; they are kept so any not-yet-updated port object that still
# references `_init`/`_fini` links cleanly, and disappear with the native
# toolchains.
POSIX_TEST_CRT_OBJ := $(POSIX_TESTS_OBJDIR)/common/crt0-stubs.o

# Compile flags: the freestanding guest C flags, plus the upstream suite defines
# (mirroring nanvix/posix-tests' src/Makefile) so the ported sources behave the
# same as upstream — standalone build, microvm platform, and the system/node
# name strings that misc-c compares against.
POSIX_TEST_CFLAGS := $(GUEST_C_APP_CFLAGS)
POSIX_TEST_CFLAGS += -D__NANVIX_STANDALONE__ -D__microvm__
POSIX_TEST_CFLAGS += -D__NANVIX_SYSNAME__=\"nanvix\" -D__NANVIX_NODENAME__=\"localhost\"

#---------------------------------------------------------------------------------------------------
# Per-suite build rule.
#---------------------------------------------------------------------------------------------------

# Object compile rule: one object per source file, mirroring the suite tree under
# the objects directory. Kept out of the per-suite `define` so it is parsed once
# (a shell loop inside an `$(eval $(call ...))` body would have its `$var`
# references eaten by the extra expansion pass). PIE suites add `-fPIE` through
# the per-object `POSIX_TEST_EXTRA_CFLAGS` target-specific variable.
$(POSIX_TESTS_OBJDIR)/%.o: $(POSIX_TESTS_SRCDIR)/%.c
	@command -v $(firstword $(GUEST_C_APP_CC)) >/dev/null 2>&1 || { \
		echo "ERROR: posix-tests need '$(firstword $(GUEST_C_APP_CC))' on PATH to cross-compile the guest C sources."; \
		exit 1; \
	}
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] compiling $< (CC=$(firstword $(GUEST_C_APP_CC)))"
	$(GUEST_C_APP_CC) $(POSIX_TEST_CFLAGS) $(POSIX_TEST_EXTRA_CFLAGS) -c $< -o $@

# $(1) = suite name (directory under src/posix-tests/ and resulting <suite>.elf).
#
# All `*.c` under the suite directory are compiled and linked together. The
# explicit `.elf` rule overrides the generic `$(BINARIES_DIR)/%.elf:
# all-guest-binaries ;` pattern bridge for these specific files (explicit rules
# win over pattern rules), so the standalone-images machinery bundles the
# resulting ELF into `<suite>.initrd` without any custom mkimage recipe.
#
# A suite may restrict the compiled set via POSIX_TEST_FILES_<suite> (a list of
# file names under the suite directory) — used by file-c, whose link-only and
# guarded sub-tests need features absent from the standalone VFS (FAT32 has no
# links/permissions; poll/select have no standalone backend).

# file-c: only the sub-tests that main.c runs under __NANVIX_STANDALONE__. The
# remaining files exercise links, permissions/ownership, timestamps, and
# poll/select, which the standalone FAT32 VFS does not support.
POSIX_TEST_FILES_file-c := \
	main.c open_close.c create_unlink.c write_read.c posix_fadvise.c lseek.c \
	posix_fallocate.c readv.c preadv.c writev.c pwritev.c pread.c pwrite.c \
	fdatasync.c stat.c ftruncate.c renameat.c unlinkat.c mkdirat.c mkdir.c \
	dirent.c getcwd.c chdir.c fchdir.c

# Extra link flags for position-independent executables. The dlfcn PIE variants
# build as PIE so the linker emits .dynsym/.dynstr/.dynamic and the executable's
# own symbols are resolvable via dlopen(NULL)/RTLD_DEFAULT. Mirrors the proven
# dlfcn-rust PIE link (src/tests/dlfcn-rust/build.rs): -z notext for the static
# libc's text relocations, -z norelro to keep .dynamic in the data segment (the
# kernel maps pages per-segment), SysV hashing, and no PT_INTERP (Nanvix has no
# system dynamic linker; nvx-crt0 self-relocates the PIE at startup).
POSIX_TEST_PIE_LDFLAGS := -pie --export-dynamic --no-dynamic-linker -z notext -z norelro --hash-style=sysv

# Suites linked as position-independent executables (PIE).
POSIX_TEST_PIE_dlfcn-pie-c := yes
POSIX_TEST_PIE_dlfcn-global-c := yes
POSIX_TEST_PIE_dlfcn-needed-c := yes

define POSIX_TEST_RULE
POSIX_TEST_SRCS_$(1) := $$(if $$(POSIX_TEST_FILES_$(1)),$$(addprefix $$(POSIX_TESTS_SRCDIR)/$(1)/,$$(POSIX_TEST_FILES_$(1))),$$(wildcard $$(POSIX_TESTS_SRCDIR)/$(1)/*.c))
POSIX_TEST_OBJS_$(1) := $$(patsubst $$(POSIX_TESTS_SRCDIR)/%.c,$$(POSIX_TESTS_OBJDIR)/%.o,$$(POSIX_TEST_SRCS_$(1)))
# PIE suites compile their objects with -fPIE and link with the PIE flags.
ifeq ($$(POSIX_TEST_PIE_$(1)),yes)
$$(POSIX_TEST_OBJS_$(1)): POSIX_TEST_EXTRA_CFLAGS := -fPIE
endif
$$(BINARIES_DIR)/$(1).$$(EXEC_FORMAT): $$(POSIX_TEST_OBJS_$(1)) $$(POSIX_TEST_CRT_OBJ) \
		$$(GUEST_C_APP_LIBC) $$(GUEST_C_APP_LIBM) $$(LIBNVX_CRT0) $$(GUEST_C_APP_LD_SCRIPT)
	@echo "[posix-test] linking $(1) against libc.a + libm.a$$(if $$(filter yes,$$(POSIX_TEST_PIE_$(1))), (PIE))"
	$$(GUEST_C_APP_LD) $$(GUEST_C_APP_LDFLAGS) $$(if $$(filter yes,$$(POSIX_TEST_PIE_$(1))),$$(POSIX_TEST_PIE_LDFLAGS)) \
		$$(POSIX_TEST_OBJS_$(1)) $$(POSIX_TEST_CRT_OBJ) \
		--start-group $$(LIBNVX_CRT0) $$(GUEST_C_APP_LIBC) $$(GUEST_C_APP_LIBM) --end-group -o $$@
	@echo "[posix-test] built $$@"
endef

$(foreach suite,$(ALL_POSIX_TESTS),$(eval $(call POSIX_TEST_RULE,$(suite))))

#---------------------------------------------------------------------------------------------------
# Per-suite standalone image rule.
#---------------------------------------------------------------------------------------------------

# Each suite is bundled into its own `<suite>.initrd` (procd + memd + vfsd + the
# suite ELF) with a custom command line, rather than going through the generic
# `standalone-images` machinery, so that:
#   * argv[0] is "<suite>.elf" — the upstream convention some suites assert on
#     (e.g. misc-c checks `strcmp(argv[0], "misc-c.elf") == 0`);
#   * suites that need environment variables (e.g. misc-c needs NANVIX_TEST=1)
#     can inject them via POSIX_TEST_ENV_<suite>.
#
# Command-line wire format (see src/libs/cmdline + src/utils/mkimage): the mkimage
# entry is `<path>;<cmdline>` and mkimage splits the path off on the first `;`;
# the kernel's split_cmdline() then splits the cmdline into `<args>;<env>` on the
# next `;`. Every `;` is written `\;` so the shell passes a literal `;` to mkimage.

# Per-suite environment variables (space-separated KEY=VALUE entries). Empty unless set.
POSIX_TEST_ENV_misc-c := NANVIX_TEST=1

# $(1) = suite name.
define POSIX_TEST_IMAGE_RULE
$$(BINARIES_DIR)/$(1).initrd: $$(BINARIES_DIR)/$(1).$$(EXEC_FORMAT) \
		$$(BINARIES_DIR)/procd.$$(EXEC_FORMAT) \
		$$(BINARIES_DIR)/memd.$$(EXEC_FORMAT) \
		$$(BINARIES_DIR)/vfsd.$$(EXEC_FORMAT) \
		$$(MKIMAGE)
	$$(MKIMAGE) -o $$@ \
		$$(BINARIES_DIR)/procd.$$(EXEC_FORMAT)\;procd \
		$$(BINARIES_DIR)/memd.$$(EXEC_FORMAT)\;memd \
		$$(BINARIES_DIR)/vfsd.$$(EXEC_FORMAT)\;vfsd \
		$$(BINARIES_DIR)/$(1).$$(EXEC_FORMAT)\;$(1).$$(EXEC_FORMAT)$$(if $$(strip $$(POSIX_TEST_ENV_$(1))),\;$$(POSIX_TEST_ENV_$(1)))
endef

$(foreach suite,$(ALL_POSIX_TESTS),$(eval $(call POSIX_TEST_IMAGE_RULE,$(suite))))

#---------------------------------------------------------------------------------------------------
# Aggregate and clean targets.
#---------------------------------------------------------------------------------------------------

.PHONY: all-posix-tests clean-posix-tests

all-posix-tests: $(foreach suite,$(ALL_POSIX_TESTS),$(BINARIES_DIR)/$(suite).$(EXEC_FORMAT))

clean-posix-tests:
	$(RM_CMD) $(foreach suite,$(ALL_POSIX_TESTS),$(BINARIES_DIR)/$(suite).$(EXEC_FORMAT))
	$(RM_CMD) $(foreach suite,$(ALL_POSIX_TESTS),$(BINARIES_DIR)/$(suite).initrd)
	$(RM_CMD) $(BINARIES_DIR)/posix-tests-ramfs.img
	$(RM_CMD) $(foreach suite,$(POSIX_TEST_SOLIB_SUITES),$(BINARIES_DIR)/posix-tests-ramfs-$(suite).img)
	$(FORCE_RM_CMD) $(BINARIES_DIR)/posix-tests-ramfs-seed
	$(FORCE_RM_CMD) $(foreach suite,$(POSIX_TEST_SOLIB_SUITES),$(BINARIES_DIR)/posix-tests-ramfs-$(suite)-seed)
	$(FORCE_RM_CMD) $(POSIX_TESTS_OBJDIR)

#---------------------------------------------------------------------------------------------------
# Boot runner: build + boot each ported suite under nanvixd in standalone mode
# (nanvixd drives the UserVM) and assert the propagated exit code is 0.
#---------------------------------------------------------------------------------------------------

POSIX_TEST_INITRDS := $(foreach suite,$(ALL_POSIX_TESTS),$(BINARIES_DIR)/$(suite).initrd)
POSIX_TEST_LOGDIR  := $(LOGS_DIR)/posix-tests

# Writable FAT32 RAMFS image for suites that exercise the file system (file-c)
# or load a shared library at runtime (dlfcn-c). Built from a tiny seed directory
# with mkramfs and handed to the UserVM via `-ramfs`; without it the standalone
# guest has no writable file system and open(O_CREAT) fails. The seed also
# carries lib/libmul.so (the prebuilt dlopen fixture that dlfcn-rust installs
# into lib/) so dlfcn-c can dlopen("lib/libmul.so"). Suites that need the image
# are listed in POSIX_TEST_RAMFS_SUITES. Suites with their own fixtures (the
# dlfcn global/needed variants, below) override the image with a per-suite one.
POSIX_TEST_RAMFS_SUITES := file-c dlfcn-c dlfcn-pie-c dlfcn-global-c dlfcn-needed-c
POSIX_TEST_RAMFS_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-seed
POSIX_TEST_RAMFS_IMG    := $(BINARIES_DIR)/posix-tests-ramfs.img

$(POSIX_TEST_RAMFS_SEED)/marker.txt:
	@$(MKDIR_CMD) $(POSIX_TEST_RAMFS_SEED)
	@echo "posix-tests ramfs marker" > $@

# Depends on dlfcn-rust's ELF so that its build script has installed
# lib/libmul.so and lib/libmul-pie.so before we stage them into the RAMFS seed.
$(POSIX_TEST_RAMFS_IMG): $(POSIX_TEST_RAMFS_SEED)/marker.txt \
		$(BINARIES_DIR)/dlfcn-rust.$(EXEC_FORMAT) all-host-binaries-mkramfs
	@$(MKDIR_CMD) $(POSIX_TEST_RAMFS_SEED)/lib
	$(CP_CMD) $(LIBRARIES_DIR)/libmul.so $(POSIX_TEST_RAMFS_SEED)/lib/
	$(CP_CMD) $(LIBRARIES_DIR)/libmul-pie.so $(POSIX_TEST_RAMFS_SEED)/lib/
	$(MKRAMFS) -o $(POSIX_TEST_RAMFS_IMG) $(POSIX_TEST_RAMFS_SEED)

#---------------------------------------------------------------------------------------------------
# Suites that build their own shared-library fixtures (a `libs/` subdirectory).
#---------------------------------------------------------------------------------------------------
#
# dlfcn-global-c and dlfcn-needed-c each ship a libprovider.so and a
# libconsumer.so, but with *different* linkage of the same file names:
#   * global-c: libconsumer.so has the provider symbols UNDEFINED (no DT_NEEDED);
#     they are resolved at dlopen time from the global scope (RTLD_GLOBAL).
#   * needed-c: libconsumer.so is linked against libprovider.so (DT_NEEDED), so
#     the loader auto-loads the provider.
# Because the names collide, each suite gets its own RAMFS image.
#
# The `.so` are built with the same host toolchain as the guest C apps:
# position-independent, freestanding objects linked with `ld -shared` and
# `-z notext` (the inline-asm-free fixtures still need text relocations allowed
# for R_386_* against local symbols), mirroring the prebuilt libmul.so recipe.

POSIX_TEST_SOLIB_SUITES := dlfcn-global-c dlfcn-needed-c
POSIX_TEST_SOLIB_CFLAGS := -m32 -march=pentiumpro -nostdlib -ffreestanding -fPIC -O2
POSIX_TEST_SOLIB_LDFLAGS := -shared -melf_i386 -z notext

# Consumer libraries that should carry a DT_NEEDED entry on libprovider.so.
POSIX_TEST_SOLIB_NEEDED_dlfcn-needed-c := yes

# $(1) = suite name with a libs/ subdir (provider.c + consumer.c).
define POSIX_TEST_SOLIB_RULE
POSIX_TEST_SOLIB_DIR_$(1) := $$(POSIX_TESTS_OBJDIR)/$(1)/libs
POSIX_TEST_RAMFS_SEED_$(1) := $$(BINARIES_DIR)/posix-tests-ramfs-$(1)-seed
POSIX_TEST_RAMFS_IMG_$(1)  := $$(BINARIES_DIR)/posix-tests-ramfs-$(1).img

# Provider: self-contained (zero undefined symbols).
$$(POSIX_TEST_SOLIB_DIR_$(1))/libprovider.so: $$(POSIX_TESTS_SRCDIR)/$(1)/libs/provider.c
	@$$(MKDIR_CMD) $$(dir $$@)
	@echo "[posix-test] building $(1)/libprovider.so"
	$$(GUEST_C_APP_CC) $$(POSIX_TEST_SOLIB_CFLAGS) -c $$< -o $$@.o
	$$(GUEST_C_APP_LD) $$(POSIX_TEST_SOLIB_LDFLAGS) $$@.o -o $$@

# Consumer: references the provider's symbols. With DT_NEEDED for needed-c
# (linked against libprovider.so); with the symbols left UNDEFINED for global-c.
$$(POSIX_TEST_SOLIB_DIR_$(1))/libconsumer.so: $$(POSIX_TESTS_SRCDIR)/$(1)/libs/consumer.c \
		$$(POSIX_TEST_SOLIB_DIR_$(1))/libprovider.so
	@$$(MKDIR_CMD) $$(dir $$@)
	@echo "[posix-test] building $(1)/libconsumer.so$$(if $$(POSIX_TEST_SOLIB_NEEDED_$(1)), (DT_NEEDED libprovider.so))"
	$$(GUEST_C_APP_CC) $$(POSIX_TEST_SOLIB_CFLAGS) -c $$< -o $$@.o
	$$(GUEST_C_APP_LD) $$(POSIX_TEST_SOLIB_LDFLAGS) $$@.o \
		$$(if $$(POSIX_TEST_SOLIB_NEEDED_$(1)),-L$$(POSIX_TEST_SOLIB_DIR_$(1)) -lprovider) -o $$@

# Per-suite RAMFS image carrying the two fixtures under lib/.
$$(POSIX_TEST_RAMFS_IMG_$(1)): $$(POSIX_TEST_SOLIB_DIR_$(1))/libprovider.so \
		$$(POSIX_TEST_SOLIB_DIR_$(1))/libconsumer.so all-host-binaries-mkramfs
	@$$(MKDIR_CMD) $$(POSIX_TEST_RAMFS_SEED_$(1))/lib
	$$(CP_CMD) $$(POSIX_TEST_SOLIB_DIR_$(1))/libprovider.so $$(POSIX_TEST_RAMFS_SEED_$(1))/lib/
	$$(CP_CMD) $$(POSIX_TEST_SOLIB_DIR_$(1))/libconsumer.so $$(POSIX_TEST_RAMFS_SEED_$(1))/lib/
	$$(MKRAMFS) -o $$@ $$(POSIX_TEST_RAMFS_SEED_$(1))
endef

$(foreach suite,$(POSIX_TEST_SOLIB_SUITES),$(eval $(call POSIX_TEST_SOLIB_RULE,$(suite))))

# All per-suite RAMFS images (built on demand by the runner).
POSIX_TEST_SOLIB_IMGS := $(foreach suite,$(POSIX_TEST_SOLIB_SUITES),$(POSIX_TEST_RAMFS_IMG_$(suite)))

# Maps each RAMFS suite to the image it boots with: a suite with its own
# fixtures uses its per-suite image (POSIX_TEST_RAMFS_IMG_<suite>, defined by the
# solib rule above), otherwise the shared image. The runner looks the suite up in
# this `suite:image` list.
POSIX_TEST_RAMFS_MAP := $(foreach s,$(POSIX_TEST_RAMFS_SUITES),$(s):$(or $(POSIX_TEST_RAMFS_IMG_$(s)),$(POSIX_TEST_RAMFS_IMG)))

# Suites that need host networking (AF_INET sockets bridged to the host). The
# runner boots these with nanvixd's `-allow-host-networking` flag, which enables
# the in-VMM network daemon in standalone mode.
POSIX_TEST_NET_SUITES := network-c

#---------------------------------------------------------------------------------------------------
# Aggregate image target.
#---------------------------------------------------------------------------------------------------

# Builds every suite's bootable `<suite>.initrd` plus the RAMFS images the
# file-system and dlopen suites need (the shared image and the per-suite
# global/needed fixtures). This is the build-side entry point used wherever
# `run-posix-tests` is skipped — notably Windows, where suites are booted
# manually under WHP (see the repo notes). `run-posix-tests` depends on the same
# set, so this also pre-stages everything the Linux runner consumes.
.PHONY: all-posix-test-images
all-posix-test-images: $(POSIX_TEST_INITRDS) \
		$(if $(strip $(POSIX_TEST_RAMFS_SUITES)),$(POSIX_TEST_RAMFS_IMG)) \
		$(POSIX_TEST_SOLIB_IMGS)
	@echo "All POSIX C test-suite images built."

.PHONY: run-posix-tests

# The boot runner is Linux- and i686-only: it relies on coreutils `timeout`,
# `/dev/null`, and a cloud-hypervisor-style nanvixd invocation (Linux), and the
# guest C toolchain is pinned to the i686 ABI (TARGET=x86). On Windows or other
# targets the suites can still be built (`all-posix-tests`); on Windows they boot
# manually under WHP (see doc + repo notes).
ifeq ($(IS_WINDOWS),yes)
run-posix-tests:
	@echo "Skipping POSIX C test suites (run-posix-tests is Linux-only; build with 'all-posix-tests' and boot manually under WHP)."
else ifneq ($(TARGET),x86)
run-posix-tests:
	@echo "Skipping POSIX C test suites (guest C toolchain is i686-only; TARGET=$(TARGET) unsupported)."
else ifeq ($(DEPLOYMENT_MODE),standalone)
run-posix-tests: $(POSIX_TEST_INITRDS) $(if $(strip $(POSIX_TEST_RAMFS_SUITES)),$(POSIX_TEST_RAMFS_IMG)) $(POSIX_TEST_SOLIB_IMGS)
	@test -f $(NANVIXD) || { echo "ERROR: $(NANVIXD) missing; run './z build -- all' first."; exit 1; }
	@test -f $(KERNEL) || { echo "ERROR: $(KERNEL) missing; run './z build -- all' first."; exit 1; }
	@test -f $(USERVM) || { echo "ERROR: $(USERVM) missing; run './z build -- all' first."; exit 1; }
	@$(MKDIR_CMD) $(POSIX_TEST_LOGDIR)
	@echo "================================================================================"
	@echo "Running ported POSIX C test suites under nanvixd (standalone)"
	@echo "================================================================================"
	@failures=""; \
	for suite in $(ALL_POSIX_TESTS); do \
		initrd="$(BINARIES_DIR)/$$suite.initrd"; \
		log="$(POSIX_TEST_LOGDIR)/$$suite.log"; \
		console="$(POSIX_TEST_LOGDIR)/$$suite.console.log"; \
		ramfs=""; \
		for entry in $(POSIX_TEST_RAMFS_MAP); do \
			case "$$entry" in \
				$$suite:*) ramfs="-ramfs $${entry#*:}" ;; \
			esac; \
		done; \
		net=""; \
		case " $(POSIX_TEST_NET_SUITES) " in \
			*" $$suite "*) net="-allow-host-networking" ;; \
		esac; \
		printf '%-24s ... ' "$$suite"; \
		rc=0; \
		timeout -k 5 $(TIMEOUT) $(NANVIXD) -console-file $$console -log-dir $(POSIX_TEST_LOGDIR) \
			$$ramfs $$net -- $$initrd \
			< /dev/null > $$log 2>&1 || rc=$$?; \
		if [ "$$rc" -eq 0 ]; then \
			echo "PASS (exit 0)"; \
		else \
			echo "FAIL (exit $$rc)"; \
			failures="$$failures $$suite"; \
		fi; \
	done; \
	echo "--------------------------------------------------------------------------------"; \
	if [ -n "$$failures" ]; then \
		echo "FAILED suites:$$failures"; \
		echo "(logs in $(POSIX_TEST_LOGDIR))"; \
		exit 1; \
	fi; \
	echo "All ported POSIX C test suites passed."
else
run-posix-tests:
	@echo "Skipping POSIX C test suites (DEPLOYMENT_MODE=$(DEPLOYMENT_MODE), requires standalone)."
endif
