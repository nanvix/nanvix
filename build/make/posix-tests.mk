# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Ported POSIX C Test Suites (built against the bundled Nanvix libc)
#===================================================================================================
#
# Builds the C test suites ported from `nanvix/posix-tests` and runs them under
# nanvixd in standalone mode (nanvixd drives the UserVM). Each suite lives at
# `src/tests/integration/<suite>/` or `src/tests/stress/<suite>/` (one or
# more `*.c` files), is compiled with the host C toolchain against the in-tree
# headers in `include/`, and linked against the merged `libc.a` (the C library +
# the Nanvix system-call backend, produced by `nanvix-libc-bundle`) — exactly
# like `build/make/guest-c-apps.mk`, whose compiler/flag definitions this fragment
# reuses.
#
# The suites build against the bundled libc and run in standalone mode.
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

# After the test reorganization the POSIX C suites are split across two source
# roots: most live under src/tests/integration, while the memory stress suite
# lives under src/tests/stress. POSIX_TESTS_SRCDIR is the integration root — the
# default for suites and the only root the shared-library fixtures use;
# POSIX_TESTS_STRESS_SRCDIR holds the stress suites. Objects from both roots are
# mirrored into a single POSIX_TESTS_OBJDIR (suite names are unique across roots).
POSIX_TESTS_SRCDIR := $(SOURCES_DIR)/tests/integration
POSIX_TESTS_STRESS_SRCDIR := $(SOURCES_DIR)/tests/stress
# Namespace the object tree by TARGET. The guest C objects are ABI-specific
# (i686 vs x86-64) but $(OBJECTS_DIR) is shared across targets, so a TARGET
# switch without a clean must not relink stale wrong-arch objects (e.g. the
# shared crt0-stubs.o).
POSIX_TESTS_OBJDIR := $(OBJECTS_DIR)/posix-tests/$(TARGET)

# Suites that live under the stress root (POSIX_TESTS_STRESS_SRCDIR) rather than
# the default integration root. Consumed by POSIX_TEST_RULE to pick the source
# directory and by the second object compile pattern rule below.
POSIX_TEST_STRESS_test-c-memory := yes

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
#
# Two pattern rules — one per source root (integration and stress) — funnel into
# the same objects directory; the recipe is identical, so any change must be made
# to both.
$(POSIX_TESTS_OBJDIR)/%.o: $(POSIX_TESTS_SRCDIR)/%.c
	@command -v $(firstword $(GUEST_C_APP_CC)) >/dev/null 2>&1 || { \
		echo "ERROR: posix-tests need '$(firstword $(GUEST_C_APP_CC))' on PATH to cross-compile the guest C sources."; \
		exit 1; \
	}
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] compiling $< (CC=$(firstword $(GUEST_C_APP_CC)))"
	$(GUEST_C_APP_CC) $(POSIX_TEST_CFLAGS) $(POSIX_TEST_EXTRA_CFLAGS) -c $< -o $@

$(POSIX_TESTS_OBJDIR)/%.o: $(POSIX_TESTS_STRESS_SRCDIR)/%.c
	@command -v $(firstword $(GUEST_C_APP_CC)) >/dev/null 2>&1 || { \
		echo "ERROR: posix-tests need '$(firstword $(GUEST_C_APP_CC))' on PATH to cross-compile the guest C sources."; \
		exit 1; \
	}
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] compiling $< (CC=$(firstword $(GUEST_C_APP_CC)))"
	$(GUEST_C_APP_CC) $(POSIX_TEST_CFLAGS) $(POSIX_TEST_EXTRA_CFLAGS) -c $< -o $@

# $(1) = suite name (directory under the suite's source root and resulting <suite>.elf).
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
# links/permissions; select has no standalone backend).

# file-c: only the sub-tests that main.c runs under __NANVIX_STANDALONE__. The
# remaining files exercise links, permissions/ownership, timestamps, and
# select(), which the standalone FAT32 VFS does not support.
POSIX_TEST_FILES_test-c-file := \
	main.c open_close.c create_unlink.c write_read.c poll.c posix_fadvise.c lseek.c \
	posix_fallocate.c readv.c preadv.c writev.c pwritev.c pread.c pwrite.c \
	fdatasync.c stat.c ftruncate.c truncate.c renameat.c unlinkat.c mkdirat.c mkdir.c \
	mkfifo.c mknod.c umask_ramfs.c umask.c dirent.c getcwd.c chdir.c fchdir.c

# Extra link flags for position-independent executables. The dlfcn PIE variants
# build as PIE so the linker emits .dynsym/.dynstr/.dynamic and the executable's
# own symbols are resolvable via dlopen(NULL)/RTLD_DEFAULT. Mirrors the proven
# dlfcn-rust PIE link (src/tests/dlfcn-rust/build.rs): -z notext for the static
# libc's text relocations, -z norelro to keep .dynamic in the data segment (the
# kernel maps pages per-segment), SysV hashing, and no PT_INTERP (Nanvix has no
# system dynamic linker; nvx-crt0 self-relocates the PIE at startup).
#
# On x86-64 a PIE link is impossible here: the static libc.a is built with
# relocation-model=static and carries R_X86_64_32S relocations that `ld -pie`
# rejects ("can not be used when making a PIE object"). Instead these suites link
# as a non-PIE ET_EXEC at the fixed guest base (< 2 GiB, so the 32-bit absolute
# relocations resolve at link time). --export-dynamic still emits
# .dynsym/.dynamic so the executable's own symbols populate the loader's global
# scope, and linking against a fixture .so still records DT_NEEDED plus the
# .rela.plt/.rela.dyn entries the startup self-linker binds. nvx-crt0's PIE
# self-relocation is a no-op for a non-PIE image.
ifeq ($(TARGET),x86_64)
POSIX_TEST_PIE_LDFLAGS := --export-dynamic --no-dynamic-linker -z notext -z norelro --hash-style=sysv
else
POSIX_TEST_PIE_LDFLAGS := -pie --export-dynamic --no-dynamic-linker -z notext -z norelro --hash-style=sysv
endif

# Suites linked as position-independent executables (PIE).
POSIX_TEST_PIE_test-c-dlfcn-pie := yes
POSIX_TEST_PIE_test-c-dlfcn-global := yes
POSIX_TEST_PIE_test-c-dlfcn-handle-reuse := yes
POSIX_TEST_PIE_test-c-dlfcn-needed := yes
POSIX_TEST_PIE_test-c-dlfcn-diamond := yes
POSIX_TEST_PIE_test-c-dlfcn-staging := yes
# dlfcn-order-c dlopen()s libroot.so, whose two providers both export
# provider_id(); the executable's own symbols land in .dynsym for RTLD_DEFAULT,
# matching every other dlfcn dlopen suite.
POSIX_TEST_PIE_test-c-dlfcn-order := yes
# dlfcn-dlclose-cycle-c's fixtures reference only each other (never the main
# executable), so PIE is not strictly required; it is set anyway to match every
# other dlfcn dlopen suite (the executable's own symbols land in .dynsym for
# RTLD_DEFAULT).
POSIX_TEST_PIE_test-c-dlfcn-dlclose-cycle := yes
# dlfcn-cycle-c dlopen()s freestanding fixtures. Its cyclic libraries are refused
# before relocation and its positive-control library exports no undefined symbols,
# so PIE is not strictly required; it is set anyway to match every other dlfcn
# dlopen suite (the executable's own symbols land in .dynsym for RTLD_DEFAULT).
POSIX_TEST_PIE_test-c-dlfcn-cycle := yes
# dlfcn-ctor-dtor-reentry-c links PIE + --export-dynamic so the main executable's
# `hook_open_other`/`hook_close_other`/`other_report_dtor` helpers land in
# `.dynsym`; libhook.so's constructor and destructor -- and libother.so's
# destructor -- then resolve those references from the loader's global symbol
# table while they run inside the in-progress dlopen()/dlclose().
POSIX_TEST_PIE_test-c-dlfcn-ctor-dtor-reentry := yes
# dlfcn-dtor-reentry-c links PIE + --export-dynamic so the main executable's
# `dtor_probe` helper (and its witness globals) land in `.dynsym`; libreentry.so's
# destructor then resolves its `extern void dtor_probe(void)` reference from the
# loader's global symbol table while it is being torn down.
POSIX_TEST_PIE_test-c-dlfcn-dtor-reentry := yes
# dlfcn-init-concurrent-c links PIE + --export-dynamic so the main executable's
# `ctor_mark_started`/`ctor_racer_arrived`/`ctor_mark_done` helpers land in
# `.dynsym`; libslowctor.so's constructor then resolves those references from
# the loader's global symbol table while it runs.
POSIX_TEST_PIE_test-c-dlfcn-init-concurrent := yes
# dlfcn-init-runpath-c links PIE with --export-dynamic so the main executable's
# `g_dtor_ran` global lands in `.dynsym`; the loader's global symbol table then
# satisfies libctor.so's `extern volatile int g_dtor_ran` reference at load time.
POSIX_TEST_PIE_test-c-dlfcn-init-runpath := yes
# dlfcn-weak-c links PIE with --export-dynamic so the main executable's
# `main_callback`/`weak_data` globals land in `.dynsym`; the loader resolves the
# helper `.so` files' weak undefined references against that global scope.
POSIX_TEST_PIE_test-c-dlfcn-weak := yes
# dlfcn-selflink-c links PIE against libprovider.so (-lprovider) so the main
# executable carries a DT_NEEDED entry plus an R_386_GLOB_DAT (data) and an
# R_386_JMP_SLOT (function) relocation against the provider's symbols. This is
# the forward direction the other dlfcn suites do not cover: nvx-crt0's
# self-linker (syscall::dlfcn::dllink_executable) must bind the executable's own
# GOT/PLT against the loaded library before main() runs.
POSIX_TEST_PIE_test-c-dlfcn-selflink := yes
# dlfcn-searchpath-c dlopen()s the REAL libc.so at run time. libc.so carries an
# UNDEFINED `__nanvix_main` (the app entry symbol, normally supplied by the main
# executable), which RTLD_NOW must resolve from the executable's exported global
# scope. Linking PIE with --export-dynamic lands the executable's symbols
# (including `__nanvix_main`) in `.dynsym` so dlinit() seeds the loader's global
# symbol table and the dlopen("libc.so") relocation succeeds.
POSIX_TEST_PIE_test-c-dlfcn-searchpath := yes
# dlfcn-scope-c links PIE + --export-dynamic so the main executable's
# `scope_main_export` helper lands in `.dynsym` and is seeded into the loader's
# global scope by dlinit(). The suite then asserts that dlsym(handle, ...) does
# NOT resolve that global-only symbol (nor an RTLD_GLOBAL library's symbol)
# through a specific handle -- only its own load group (self + DT_NEEDED). This
# is the acceptance test for handle-scoped lookup.
POSIX_TEST_PIE_test-c-dlfcn-scope := yes

# Per-suite hooks consumed by POSIX_TEST_RULE: extra link-time prerequisites and
# extra linker arguments appended AFTER the libc/libm `--end-group`. Used by
# dlfcn-selflink-c to link the suite ELF against its libprovider.so fixture
# (recording the DT_NEEDED entry and the GOT/PLT relocations) with eager binding
# (`-z now`), since Nanvix has no lazy PLT resolver. These must be set before the
# POSIX_TEST_RULE foreach below expands each suite's link rule.
POSIX_TEST_SELFLINK_DIR := $(POSIX_TESTS_OBJDIR)/test-c-dlfcn-selflink/libs
POSIX_TEST_EXTRA_LD_DEPS_test-c-dlfcn-selflink := $(POSIX_TEST_SELFLINK_DIR)/libprovider.so
POSIX_TEST_EXTRA_LDLIBS_test-c-dlfcn-selflink := -L$(POSIX_TEST_SELFLINK_DIR) -lprovider -z now

# dlfcn-initfini-c is the acceptance test for init/fini ordering across a
# startup-loaded DT_NEEDED dependency: it links the executable PIE
# + --export-dynamic against a purpose-built libinitfini.so (-linitfini) that the
# executable references by NO symbol, so the dependency is auto-loaded at startup
# purely for its `.init_array` constructor (run before main) and `.fini_array`
# destructor (run at exit by syscall::dlfcn::dlfini_executable), with NO dlopen()
# call. `--no-as-needed` forces the DT_NEEDED entry even though the executable
# references none of libinitfini.so's symbols; `-z now` forces eager binding
# (Nanvix has no lazy PLT resolver). The bare DT_NEEDED name (pinned by the
# fixture's -soname) is resolved through the loader's default lib/ search path,
# where the per-suite RAMFS stages libinitfini.so.
POSIX_TEST_PIE_test-c-dlfcn-initfini := yes
POSIX_TEST_INITFINI_DIR := $(POSIX_TESTS_OBJDIR)/test-c-dlfcn-initfini/libs
POSIX_TEST_EXTRA_LD_DEPS_test-c-dlfcn-initfini := $(POSIX_TEST_INITFINI_DIR)/libinitfini.so
POSIX_TEST_EXTRA_LDLIBS_test-c-dlfcn-initfini := -L$(POSIX_TEST_INITFINI_DIR) --no-as-needed -linitfini -z now

# dlfcn-startup-c is the acceptance test for the startup DT_NEEDED loader (issue
# #2773): it links the executable PIE against the REAL toolchain-shipped shared
# libraries libc.so and libm.so (via `-l:libc.so -l:libm.so`) instead of the
# static libm.a, so the executable carries DT_NEEDED entries for both plus
# R_386_JMP_SLOT relocations against libm.so's math symbols. The self-linker
# (syscall::dlfcn::dllink_executable) must auto-load libc.so and then libm.so
# into the global scope and bind the executable's GOT/PLT before main() runs,
# with NO dlopen() call. libm.so's allocator / mem* imports resolve from libc.so
# (loaded first, in DT_NEEDED order); the executable's statically linked libc.a
# stays the single heap owner because the loader's global scope is first-wins.
# `-z now` forces eager binding (Nanvix has no lazy PLT resolver). The bare
# DT_NEEDED names (`-l:` with no SONAME on the .so) are resolved through the
# loader's default lib/ search path, where the per-suite RAMFS stages them.
POSIX_TEST_PIE_test-c-dlfcn-startup := yes
# Drop the static libm.a from the link group so the math symbols stay undefined
# and bind to libm.so at startup instead of being pulled in statically.
POSIX_TEST_NO_STATIC_LIBM_test-c-dlfcn-startup := yes
POSIX_TEST_EXTRA_LD_DEPS_test-c-dlfcn-startup := $(LIBRARIES_DIR)/libc.so $(LIBRARIES_DIR)/libm.so
POSIX_TEST_EXTRA_LDLIBS_test-c-dlfcn-startup := -L$(LIBRARIES_DIR) -l:libc.so -l:libm.so -z now

# dlfcn-hello-c is the "dynamic hello" acceptance test: the capstone
# that boots a plain hello-world executable linked against the REAL shared
# libraries libc.so and libm.so and lets the crt0 startup loader auto-resolve them
# before main(), with NO dlopen() call. Its link wiring is identical to
# dlfcn-startup-c above — PIE, static libm.a dropped so the math symbols bind to
# libm.so (R_386_JMP_SLOT), DT_NEEDED on the bare libc.so/libm.so names, `-z now`
# for eager binding — but main.c folds the dynamically-resolved cos/pow/exp
# results into a single printed "value=42" instead of asserting each math result
# separately. The bare DT_NEEDED names are resolved through the loader's default
# lib/ search path, where the per-suite RAMFS stages libc.so and libm.so.
POSIX_TEST_PIE_test-c-dlfcn-hello := yes
POSIX_TEST_NO_STATIC_LIBM_test-c-dlfcn-hello := yes
POSIX_TEST_EXTRA_LD_DEPS_test-c-dlfcn-hello := $(LIBRARIES_DIR)/libc.so $(LIBRARIES_DIR)/libm.so
POSIX_TEST_EXTRA_LDLIBS_test-c-dlfcn-hello := -L$(LIBRARIES_DIR) -l:libc.so -l:libm.so -z now

define POSIX_TEST_RULE
POSIX_TEST_SRCROOT_$(1) := $$(if $$(POSIX_TEST_STRESS_$(1)),$$(POSIX_TESTS_STRESS_SRCDIR),$$(POSIX_TESTS_SRCDIR))
POSIX_TEST_SRCS_$(1) := $$(if $$(POSIX_TEST_FILES_$(1)),$$(addprefix $$(POSIX_TEST_SRCROOT_$(1))/$(1)/,$$(POSIX_TEST_FILES_$(1))),$$(wildcard $$(POSIX_TEST_SRCROOT_$(1))/$(1)/*.c))
POSIX_TEST_OBJS_$(1) := $$(patsubst $$(POSIX_TEST_SRCROOT_$(1))/%.c,$$(POSIX_TESTS_OBJDIR)/%.o,$$(POSIX_TEST_SRCS_$(1)))
# PIE suites compile their objects with -fPIE and link with the PIE flags. On
# x86-64 the final link is non-PIE (see POSIX_TEST_PIE_LDFLAGS), but the objects
# are still compiled -fPIE so that references to a shared library's symbols go
# through the GOT/PLT (R_X86_64_GLOB_DAT / R_X86_64_JUMP_SLOT, which the startup
# self-linker binds) rather than becoming R_X86_64_COPY relocations, which a
# non-PIC executable would otherwise emit for shared data.
ifeq ($$(POSIX_TEST_PIE_$(1)),yes)
$$(POSIX_TEST_OBJS_$(1)): POSIX_TEST_EXTRA_CFLAGS := -fPIE
endif
$$(BINARIES_DIR)/$(1).$$(EXEC_FORMAT): $$(POSIX_TEST_OBJS_$(1)) $$(POSIX_TEST_CRT_OBJ) \
		$$(GUEST_C_APP_LIBC) $$(GUEST_C_APP_LIBM) $$(LIBNVX_CRT0) $$(GUEST_C_APP_LD_SCRIPT) \
		$$(POSIX_TEST_EXTRA_LD_DEPS_$(1))
	@echo "[posix-test] linking $(1) against libc.a$$(if $$(POSIX_TEST_NO_STATIC_LIBM_$(1)),, + libm.a)$$(if $$(filter yes,$$(POSIX_TEST_PIE_$(1))), (PIE))"
	$$(GUEST_C_APP_LD) $$(GUEST_C_APP_LDFLAGS) $$(if $$(filter yes,$$(POSIX_TEST_PIE_$(1))),$$(POSIX_TEST_PIE_LDFLAGS)) \
		$$(POSIX_TEST_OBJS_$(1)) $$(POSIX_TEST_CRT_OBJ) \
		--start-group $$(LIBNVX_CRT0) $$(GUEST_C_APP_LIBC) $$(if $$(POSIX_TEST_NO_STATIC_LIBM_$(1)),,$$(GUEST_C_APP_LIBM)) --end-group \
		$$(POSIX_TEST_EXTRA_LDLIBS_$(1)) -o $$@
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
#     (e.g. test-c-misc checks `strcmp(argv[0], "test-c-misc.elf") == 0`);
#   * suites that need environment variables (e.g. misc-c needs NANVIX_TEST=1)
#     can inject them via POSIX_TEST_ENV_<suite>.
#
# Command-line wire format (see src/libs/cmdline + src/utils/mkimage): the mkimage
# entry is `<path>;<cmdline>` and mkimage splits the path off on the first `;`;
# the kernel's split_cmdline() then splits the cmdline into `<args>;<env>` on the
# next `;`. Every `;` is written `\;` so the shell passes a literal `;` to mkimage.

# Per-suite environment variables (space-separated KEY=VALUE entries). Empty unless set.
POSIX_TEST_ENV_test-c-misc := NANVIX_TEST=1
ifneq ($(IS_WINDOWS),yes)
POSIX_TEST_ENV_test-c-file := NANVIX_TEST_HOSTFS=1
POSIX_TEST_ENV_test-c-network := NANVIX_TEST_HOSTFS=1
endif

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

POSIX_HEADERS_CXX_FLAGS := -m32 -march=pentiumpro -ffreestanding -nostdinc -nostdinc++ -std=c++17
POSIX_HEADERS_CXX_FLAGS += -isystem $(ROOT_DIR)/include
POSIX_PUBLIC_HEADERS := $(filter-out $(ROOT_DIR)/include/stdatomic.h,\
	$(wildcard $(ROOT_DIR)/include/*.h $(ROOT_DIR)/include/*/*.h))
POSIX_HEADERS_CXX_STAMP := $(POSIX_TESTS_OBJDIR)/headers-cxx-check.stamp

#---------------------------------------------------------------------------------------------------
# Aggregate and clean targets.
#---------------------------------------------------------------------------------------------------

.PHONY: all-posix-tests clean-posix-tests check-headers-cxx

all-posix-tests: $(POSIX_HEADERS_CXX_STAMP) $(foreach suite,$(ALL_POSIX_TESTS),$(BINARIES_DIR)/$(suite).$(EXEC_FORMAT))

clean-posix-tests:
	$(RM_CMD) $(foreach suite,$(ALL_POSIX_TESTS),$(BINARIES_DIR)/$(suite).$(EXEC_FORMAT))
	$(RM_CMD) $(foreach suite,$(ALL_POSIX_TESTS),$(BINARIES_DIR)/$(suite).initrd)
	$(RM_CMD) $(BINARIES_DIR)/posix-tests-ramfs.img
	$(RM_CMD) $(foreach suite,$(POSIX_TEST_SOLIB_SUITES),$(BINARIES_DIR)/posix-tests-ramfs-$(suite).img)
	$(RM_CMD) $(POSIX_TEST_RUNPATH_IMG)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-ctor-dtor-reentry)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-cycle)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-diamond)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-order)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-dlclose-cycle)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-dtor-reentry)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-handle-reuse)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-hash)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-init-concurrent)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-hello)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-scope)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-searchpath)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-selflink)
	$(RM_CMD) $(POSIX_TEST_STAGING_IMG)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-startup)
	$(RM_CMD) $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-initfini)
	$(RM_CMD) $(POSIX_TEST_WEAK_IMG)
	$(RM_CMD) $(POSIX_TEST_EXECVP_IMG)
	$(FORCE_RM_CMD) $(BINARIES_DIR)/posix-tests-ramfs-seed
	$(FORCE_RM_CMD) $(foreach suite,$(POSIX_TEST_SOLIB_SUITES),$(BINARIES_DIR)/posix-tests-ramfs-$(suite)-seed)
	$(FORCE_RM_CMD) $(POSIX_TEST_RUNPATH_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_CTOR_DTOR_REENTRY_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_CYCLE_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_DIAMOND_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_ORDER_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_DLCLOSE_CYCLE_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_DTOR_REENTRY_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_HANDLE_REUSE_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_HASH_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_INIT_CONCURRENT_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_SCOPE_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_SELFLINK_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_STAGING_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_HELLO_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_SEARCHPATH_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_STARTUP_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_INITFINI_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_WEAK_SEED)
	$(FORCE_RM_CMD) $(POSIX_TEST_EXECVP_SEED)
	$(FORCE_RM_CMD) $(POSIX_TESTS_OBJDIR)

#---------------------------------------------------------------------------------------------------
# C++ header-safety check.
#---------------------------------------------------------------------------------------------------
#
# The generated libc headers in include/ are wrapped in `extern "C"` and are meant to be includable
# from C++ (libunwind, libc++, libc++abi, user C++). This check compiles every public header as a
# standalone C++ translation unit (syntax-only) to guard that contract: no C++ keyword is used as a
# parameter identifier, and the C99 `restrict` qualifier is neutralized under C++. It
# complements the C-only suites, which compile the same headers as C and so cannot catch a C++ parse
# regression. The headers are also re-checked with `-Drestrict=__restrict` to prove the private
# `__nanvix_restrict` qualifier macro does not depend on the caller's `restrict` macro state.
#
# <stdatomic.h> is excluded: C11 atomics are intentionally C-only (C++ provides <atomic>), and no
# libc header includes it.

check-headers-cxx: $(POSIX_HEADERS_CXX_STAMP)

$(POSIX_HEADERS_CXX_STAMP): $(POSIX_PUBLIC_HEADERS)
	@command -v $(firstword $(GUEST_C_APP_CC)) >/dev/null 2>&1 || { \
		echo "ERROR: posix-tests need '$(firstword $(GUEST_C_APP_CC))' on PATH to C++-check the generated headers."; \
		exit 1; \
	}
	@$(MKDIR_CMD) $(dir $@)
	@for header in $(patsubst $(ROOT_DIR)/include/%,%,$(POSIX_PUBLIC_HEADERS)); do \
		echo "[posix-test] C++-checking <$$header>"; \
		printf '#include <%s>\nint main(void){return 0;}\n' "$$header" \
			| $(GUEST_C_APP_CC) $(POSIX_HEADERS_CXX_FLAGS) -x c++ -fsyntax-only - || exit 1; \
		printf '#include <%s>\nint main(void){return 0;}\n' "$$header" \
			| $(GUEST_C_APP_CC) $(POSIX_HEADERS_CXX_FLAGS) -Drestrict=__restrict -x c++ -fsyntax-only - || exit 1; \
	done
	@touch $@

#---------------------------------------------------------------------------------------------------
# Boot runner: build + boot each ported suite under nanvixd in standalone mode
# (nanvixd drives the UserVM) and assert the propagated exit code is 0.
#---------------------------------------------------------------------------------------------------

POSIX_TEST_INITRDS := $(foreach suite,$(ALL_POSIX_TESTS),$(BINARIES_DIR)/$(suite).initrd)

# Writable FAT32 RAMFS image for suites that exercise the file system (file-c)
# or load a shared library at runtime (dlfcn-c). Built from a tiny seed directory
# with mkramfs and handed to the UserVM via `-ramfs`; without it the standalone
# guest has no writable file system and open(O_CREAT) fails. The seed also
# carries lib/libmul.so (the prebuilt dlopen fixture that dlfcn-rust installs
# into lib/) so dlfcn-c can dlopen("lib/libmul.so"). Suites that need the image
# are listed in POSIX_TEST_RAMFS_SUITES. Suites with their own fixtures (the
# dlfcn global/needed variants, below) override the image with a per-suite one.
# The shared-image dlfcn entries are i686-only; on x86_64 only the file-system
# suites need the shared writable RAMFS.
ifeq ($(TARGET),x86_64)
POSIX_TEST_RAMFS_SUITES := test-c-file test-c-stdio
else
POSIX_TEST_RAMFS_SUITES := test-c-file test-c-stdio test-c-dlfcn test-c-dlfcn-refcount test-c-dlfcn-pie test-c-dlfcn-global test-c-dlfcn-needed test-c-dlfcn-diamond
endif
POSIX_TEST_RAMFS_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-seed
POSIX_TEST_RAMFS_IMG    := $(BINARIES_DIR)/posix-tests-ramfs.img

$(POSIX_TEST_RAMFS_SEED)/marker.txt:
	@$(MKDIR_CMD) $(POSIX_TEST_RAMFS_SEED)
	@echo "posix-tests ramfs marker" > $@

# The shared writable RAMFS. On x86 it also carries the dlfcn dlopen fixtures
# (lib/libmul.so + lib/libmul-pie.so, staged by test-rust-dlfcn's build script),
# so it depends on that ELF. On x86_64 the dlfcn suites are not built, so the
# image only provides the writable file system the file-system suites need.
ifeq ($(TARGET),x86_64)
$(POSIX_TEST_RAMFS_IMG): $(POSIX_TEST_RAMFS_SEED)/marker.txt all-host-binaries-mkramfs
	$(MKRAMFS) -o $(POSIX_TEST_RAMFS_IMG) $(POSIX_TEST_RAMFS_SEED)
else
$(POSIX_TEST_RAMFS_IMG): $(POSIX_TEST_RAMFS_SEED)/marker.txt \
		$(BINARIES_DIR)/test-rust-dlfcn.$(EXEC_FORMAT) all-host-binaries-mkramfs
	@$(MKDIR_CMD) $(POSIX_TEST_RAMFS_SEED)/lib
	$(CP_CMD) $(LIBRARIES_DIR)/libmul.so $(POSIX_TEST_RAMFS_SEED)/lib/
	$(CP_CMD) $(LIBRARIES_DIR)/libmul-pie.so $(POSIX_TEST_RAMFS_SEED)/lib/
	$(MKRAMFS) -o $(POSIX_TEST_RAMFS_IMG) $(POSIX_TEST_RAMFS_SEED)
endif

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

POSIX_TEST_SOLIB_SUITES := test-c-dlfcn-global test-c-dlfcn-needed
# The fixture shared objects follow the active guest ABI: i686 PIC on x86, x86-64
# PIC on x86_64. Both are position-independent (`-fPIC` / `-shared`); `-z notext`
# tolerates the freestanding fixtures' text relocations against local symbols.
ifeq ($(TARGET),x86_64)
POSIX_TEST_SOLIB_CFLAGS := -m64 -march=x86-64 -nostdlib -ffreestanding -fPIC -O2 -isystem $(ROOT_DIR)/include
POSIX_TEST_SOLIB_LDFLAGS := -shared -melf_x86_64 -z notext
else
POSIX_TEST_SOLIB_CFLAGS := -m32 -march=pentiumpro -nostdlib -ffreestanding -fPIC -O2 -isystem $(ROOT_DIR)/include
POSIX_TEST_SOLIB_LDFLAGS := -shared -melf_i386 -z notext
endif

# Consumer libraries that should carry a DT_NEEDED entry on libprovider.so.
POSIX_TEST_SOLIB_NEEDED_test-c-dlfcn-needed := yes

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

#---------------------------------------------------------------------------------------------------
# dlfcn-scope-c: load-group-only dlsym(handle) fixtures.
#---------------------------------------------------------------------------------------------------
#
#   libfoo.so -> libdep.so        (DT_NEEDED: scope_dep_value lives in the group)
#   libother.so                   (isolated; dlopen'd with RTLD_GLOBAL at run time)
#
# libfoo.so is the object the test obtains a handle to. It carries a DT_NEEDED
# edge on libdep.so (it calls scope_dep_value()), so both scope_foo_value() and
# scope_dep_value() are in libfoo.so's load group and must resolve through
# dlsym(libfoo_handle, ...). libother.so has NO relationship with libfoo.so; the
# suite dlopen()s it with RTLD_GLOBAL to publish scope_other_value() into the
# loader's global scope. The acceptance criterion is that neither
# scope_other_value() (RTLD_GLOBAL) nor the main executable's scope_main_export()
# (--export-dynamic) is reachable through libfoo.so's handle -- only through
# RTLD_DEFAULT. Built with the same i686 freestanding toolchain as the
# global/needed/diamond fixtures above; the DT_NEEDED edge is produced by linking
# libfoo.so against libdep.so with `-L<dir> -ldep` (the linker records the found
# libdep.so as a bare DT_NEEDED entry, resolved through the loader's default
# lib/ search path).
POSIX_TEST_SCOPE_DIR  := $(POSIX_TESTS_OBJDIR)/test-c-dlfcn-scope/libs
POSIX_TEST_SCOPE_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-scope-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-scope := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-scope.img

# Dependency: libdep.so (no dependencies).
$(POSIX_TEST_SCOPE_DIR)/libdep.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-scope/libs/dep.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-scope-c/libdep.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# Handle library: DT_NEEDED libdep.so (calls scope_dep_value()).
$(POSIX_TEST_SCOPE_DIR)/libfoo.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-scope/libs/foo.c \
		$(POSIX_TEST_SCOPE_DIR)/libdep.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-scope-c/libfoo.so (DT_NEEDED libdep.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -L$(POSIX_TEST_SCOPE_DIR) -ldep -o $@

# Isolated library published with RTLD_GLOBAL at run time (no dependencies).
$(POSIX_TEST_SCOPE_DIR)/libother.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-scope/libs/other.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-scope-c/libother.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# Per-suite RAMFS image carrying all three fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-scope): $(POSIX_TEST_SCOPE_DIR)/libdep.so \
		$(POSIX_TEST_SCOPE_DIR)/libfoo.so \
		$(POSIX_TEST_SCOPE_DIR)/libother.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_SCOPE_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_SCOPE_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_SCOPE_DIR)/libdep.so $(POSIX_TEST_SCOPE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_SCOPE_DIR)/libfoo.so $(POSIX_TEST_SCOPE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_SCOPE_DIR)/libother.so $(POSIX_TEST_SCOPE_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_SCOPE_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-staging-c: failed-load staging fixtures.
#---------------------------------------------------------------------------------------------------
#
#   libfailed-root.so -> libstaged.so
#                     -> libbad.so -> missing_value (strong undefined symbol)
#
# The failed root reaches relocation only after all three libraries have been
# opened. The loader must discard that staged graph when libbad.so cannot be
# resolved. A later direct load of libstaged.so must therefore open a fresh copy
# and run its constructor. libgood-root.so then verifies that a newly staged root
# can bind against the now-resident libstaged.so from the stable registry.
POSIX_TEST_STAGING_DIR := $(POSIX_TESTS_OBJDIR)/test-c-dlfcn-staging/libs
POSIX_TEST_STAGING_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-staging-seed
POSIX_TEST_STAGING_IMG := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-staging.img

$(POSIX_TEST_STAGING_DIR)/libstaged.so: \
		$(POSIX_TESTS_SRCDIR)/test-c-dlfcn-staging/libs/staged.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-staging-c/libstaged.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libstaged.so $@.o -o $@

$(POSIX_TEST_STAGING_DIR)/libbad.so: \
		$(POSIX_TESTS_SRCDIR)/test-c-dlfcn-staging/libs/bad.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-staging-c/libbad.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libbad.so $@.o -o $@

$(POSIX_TEST_STAGING_DIR)/libfailed-root.so: \
		$(POSIX_TESTS_SRCDIR)/test-c-dlfcn-staging/libs/failed_root.c \
		$(POSIX_TEST_STAGING_DIR)/libstaged.so \
		$(POSIX_TEST_STAGING_DIR)/libbad.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-staging-c/libfailed-root.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libfailed-root.so $@.o \
		--no-as-needed $(POSIX_TEST_STAGING_DIR)/libstaged.so \
		$(POSIX_TEST_STAGING_DIR)/libbad.so -o $@

$(POSIX_TEST_STAGING_DIR)/libgood-root.so: \
		$(POSIX_TESTS_SRCDIR)/test-c-dlfcn-staging/libs/good_root.c \
		$(POSIX_TEST_STAGING_DIR)/libstaged.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-staging-c/libgood-root.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libgood-root.so $@.o \
		--no-as-needed $(POSIX_TEST_STAGING_DIR)/libstaged.so -o $@

POSIX_TEST_STAGING_LIBS := \
	libstaged.so \
	libbad.so \
	libfailed-root.so \
	libgood-root.so

$(POSIX_TEST_STAGING_IMG): \
		$(foreach lib,$(POSIX_TEST_STAGING_LIBS),$(POSIX_TEST_STAGING_DIR)/$(lib)) \
		all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_STAGING_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_STAGING_SEED)/lib
	$(CP_CMD) $(foreach lib,$(POSIX_TEST_STAGING_LIBS),$(POSIX_TEST_STAGING_DIR)/$(lib)) \
		$(POSIX_TEST_STAGING_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_STAGING_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-diamond-c: four-library diamond DT_NEEDED fixture.
#---------------------------------------------------------------------------------------------------
#
#   libdiamond.so -> libleft.so  -> libbase.so
#                 -> libright.so -> libbase.so
#                 -> libbase.so                (direct edge)
#
# The two arms (libleft.so, libright.so) each carry a DT_NEEDED on libbase.so;
# libdiamond.so carries DT_NEEDED on both arms AND a direct DT_NEEDED on
# libbase.so. A correct loader consolidates all three libbase.so edges onto a
# single in-memory instance instead of opening it more than once (or
# dead-locking on the recursive load). The two arm edges are consolidated by
# the per-frame registry scan; libdiamond.so's own direct edge is consolidated
# by the loader's load-loop re-check (libbase.so is loaded by an arm after
# libdiamond's entry scan already ran). Built with the same i686 freestanding
# toolchain as the provider/consumer fixtures above; each DT_NEEDED edge is
# produced by linking the dependent against its dependencies with `-L<dir>
# -l<name>` (the linker records each found `lib<name>.so` as a bare DT_NEEDED
# entry, which the loader resolves through its default `lib/` search path —
# exactly like dlfcn-needed-c).
POSIX_TEST_DIAMOND_DIR  := $(POSIX_TESTS_OBJDIR)/test-c-dlfcn-diamond/libs
POSIX_TEST_DIAMOND_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-diamond-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-diamond := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-diamond.img

# Leaf: libbase.so (no dependencies).
$(POSIX_TEST_DIAMOND_DIR)/libbase.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-diamond/libs/base.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-diamond-c/libbase.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# Left arm: DT_NEEDED libbase.so.
$(POSIX_TEST_DIAMOND_DIR)/libleft.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-diamond/libs/left.c \
		$(POSIX_TEST_DIAMOND_DIR)/libbase.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-diamond-c/libleft.so (DT_NEEDED libbase.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -L$(POSIX_TEST_DIAMOND_DIR) -lbase -o $@

# Right arm: DT_NEEDED libbase.so.
$(POSIX_TEST_DIAMOND_DIR)/libright.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-diamond/libs/right.c \
		$(POSIX_TEST_DIAMOND_DIR)/libbase.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-diamond-c/libright.so (DT_NEEDED libbase.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -L$(POSIX_TEST_DIAMOND_DIR) -lbase -o $@

# Root: DT_NEEDED libleft.so + libright.so + libbase.so. The direct libbase.so
# edge (alongside the two arms that also pull it in) is what exercises the
# load-loop re-check in load_all_dependencies(): an arm loads libbase.so first,
# then libdiamond.so's own libbase.so edge must bind to that existing instance
# instead of re-opening it.
$(POSIX_TEST_DIAMOND_DIR)/libdiamond.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-diamond/libs/diamond.c \
		$(POSIX_TEST_DIAMOND_DIR)/libleft.so $(POSIX_TEST_DIAMOND_DIR)/libright.so \
		$(POSIX_TEST_DIAMOND_DIR)/libbase.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-diamond-c/libdiamond.so (DT_NEEDED libleft.so libright.so libbase.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o \
		-L$(POSIX_TEST_DIAMOND_DIR) -lleft -lright -lbase -o $@

# Per-suite RAMFS image carrying all four fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-diamond): $(POSIX_TEST_DIAMOND_DIR)/libbase.so \
		$(POSIX_TEST_DIAMOND_DIR)/libleft.so \
		$(POSIX_TEST_DIAMOND_DIR)/libright.so \
		$(POSIX_TEST_DIAMOND_DIR)/libdiamond.so all-host-binaries-mkramfs
	@$(MKDIR_CMD) $(POSIX_TEST_DIAMOND_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_DIAMOND_DIR)/libbase.so $(POSIX_TEST_DIAMOND_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_DIAMOND_DIR)/libleft.so $(POSIX_TEST_DIAMOND_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_DIAMOND_DIR)/libright.so $(POSIX_TEST_DIAMOND_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_DIAMOND_DIR)/libdiamond.so $(POSIX_TEST_DIAMOND_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_DIAMOND_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-order-c: DT_NEEDED dependency search-order fixture (issue #2091).
#---------------------------------------------------------------------------------------------------
#
#   libroot.so -> libbravo.so    (provider_id() == 2)  [first  in DT_NEEDED]
#              -> libalpha.so     (provider_id() == 1)  [second in DT_NEEDED]
#
# Both providers export the SAME symbol provider_id(). libroot.so records its
# DT_NEEDED entries as (libbravo.so, libalpha.so) -- pinned by the `-lbravo
# -lalpha` link order -- which is the REVERSE of the alphabetical order of the
# names. A loader that searches a load group in DT_NEEDED order (BFS, matching
# glibc's l_searchlist) resolves provider_id() through libroot.so's handle to
# libbravo.so (2); a loader that searches alphabetically resolves it to
# libalpha.so (1). The suite asserts the former. Built with the same i686
# freestanding toolchain as the diamond fixtures above; each DT_NEEDED edge is a
# bare name recorded by linking with `-L<dir> -l<name>`, resolved through the
# loader's default lib/ search path where the per-suite RAMFS stages the .so
# files.
POSIX_TEST_ORDER_DIR  := $(POSIX_TESTS_OBJDIR)/test-c-dlfcn-order/libs
POSIX_TEST_ORDER_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-order-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-order := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-order.img

# Provider: libalpha.so (no dependencies). Exports provider_id() == 1.
$(POSIX_TEST_ORDER_DIR)/libalpha.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-order/libs/alpha.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-order-c/libalpha.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# Provider: libbravo.so (no dependencies). Exports provider_id() == 2.
$(POSIX_TEST_ORDER_DIR)/libbravo.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-order/libs/bravo.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-order-c/libbravo.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# Root: DT_NEEDED libbravo.so then libalpha.so. The `-lbravo -lalpha` order pins
# the DT_NEEDED order to bravo-before-alpha (the reverse of alphabetical), so the
# search order is observable. Built after both providers so the linker records
# the edges.
$(POSIX_TEST_ORDER_DIR)/libroot.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-order/libs/root.c \
		$(POSIX_TEST_ORDER_DIR)/libbravo.so $(POSIX_TEST_ORDER_DIR)/libalpha.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-order-c/libroot.so (DT_NEEDED libbravo.so libalpha.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o \
		-L$(POSIX_TEST_ORDER_DIR) -lbravo -lalpha -o $@

# Per-suite RAMFS image carrying all three fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-order): $(POSIX_TEST_ORDER_DIR)/libalpha.so \
		$(POSIX_TEST_ORDER_DIR)/libbravo.so \
		$(POSIX_TEST_ORDER_DIR)/libroot.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_ORDER_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_ORDER_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_ORDER_DIR)/libalpha.so $(POSIX_TEST_ORDER_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_ORDER_DIR)/libbravo.so $(POSIX_TEST_ORDER_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_ORDER_DIR)/libroot.so $(POSIX_TEST_ORDER_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_ORDER_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-dlclose-cycle-c: dlclose() over a multiply-referenced dependency graph.
#---------------------------------------------------------------------------------------------------
#
#   libroot.so -> libmidx.so -> libleaf.so
#              -> libmidy.so -> libleaf.so
#              -> libleaf.so                (direct edge)
#
# libleaf.so is reachable from libroot.so through THREE DT_NEEDED edges, so a
# single dlclose(libroot.so) visits it more than once while walking the
# dependency graph. That repeated visit is what made the pre-fix dlclose() panic
# (it removed each dependency with `extract_if` and asserted exactly one entry
# was removed per step); the post-fix reference-count peel records visited nodes
# and must instead unload libleaf.so exactly once, and only after every edge that
# references it is gone. A true DT_NEEDED cycle is refused at load time, so this
# loadable diamond-with-a-direct-edge graph is the equivalent that still drives
# the repeated-visit teardown path.
# Built with the same i686 freestanding toolchain as the diamond fixtures above;
# each DT_NEEDED edge is produced by linking the dependent against its
# dependencies with `-L<dir> -l<name>` (the linker records each found
# `lib<name>.so` as a bare DT_NEEDED entry, resolved through the loader's default
# lib/ search path).
POSIX_TEST_DLCLOSE_CYCLE_DIR  := $(POSIX_TESTS_OBJDIR)/test-c-dlfcn-dlclose-cycle/libs
POSIX_TEST_DLCLOSE_CYCLE_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-dlclose-cycle-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-dlclose-cycle := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-dlclose-cycle.img

# Leaf: libleaf.so (no dependencies).
$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libleaf.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-dlclose-cycle/libs/leaf.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-dlclose-cycle-c/libleaf.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# Intermediate libmidx.so: DT_NEEDED libleaf.so.
$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libmidx.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-dlclose-cycle/libs/midx.c \
		$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libleaf.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-dlclose-cycle-c/libmidx.so (DT_NEEDED libleaf.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -L$(POSIX_TEST_DLCLOSE_CYCLE_DIR) -lleaf -o $@

# Intermediate libmidy.so: DT_NEEDED libleaf.so.
$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libmidy.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-dlclose-cycle/libs/midy.c \
		$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libleaf.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-dlclose-cycle-c/libmidy.so (DT_NEEDED libleaf.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -L$(POSIX_TEST_DLCLOSE_CYCLE_DIR) -lleaf -o $@

# Root: DT_NEEDED libmidx.so + libmidy.so + libleaf.so. The direct libleaf.so
# edge (alongside the two intermediates that also pull it in) is what makes
# libleaf.so reachable more than once during dlclose(libroot.so).
$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libroot.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-dlclose-cycle/libs/root.c \
		$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libmidx.so $(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libmidy.so \
		$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libleaf.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-dlclose-cycle-c/libroot.so (DT_NEEDED libmidx.so libmidy.so libleaf.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o \
		-L$(POSIX_TEST_DLCLOSE_CYCLE_DIR) -lmidx -lmidy -lleaf -o $@

# Per-suite RAMFS image carrying all four fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-dlclose-cycle): $(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libleaf.so \
		$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libmidx.so \
		$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libmidy.so \
		$(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libroot.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_DLCLOSE_CYCLE_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_DLCLOSE_CYCLE_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libleaf.so $(POSIX_TEST_DLCLOSE_CYCLE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libmidx.so $(POSIX_TEST_DLCLOSE_CYCLE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libmidy.so $(POSIX_TEST_DLCLOSE_CYCLE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_DLCLOSE_CYCLE_DIR)/libroot.so $(POSIX_TEST_DLCLOSE_CYCLE_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_DLCLOSE_CYCLE_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-cycle-c: DT_NEEDED cycle-rejection fixtures.
#---------------------------------------------------------------------------------------------------
#
#   libcyclea.so    DT_NEEDED libcycleb.so    --+
#   libcycleb.so    DT_NEEDED libcyclea.so    --+  (two-node cycle)
#   libselfcycle.so DT_NEEDED libselfcycle.so       (single-node self-loop)
#   libok.so        (no dependencies)               (positive control)
#
# The loader must refuse a dlopen() of any cyclic graph with a clean NULL +
# dlerror() result instead of recursing without bound (see
# load_all_dependencies_recursive in
# src/libs/syscall/src/dlfcn/syscall/dlopen.rs); dlfcn-cycle-c/main.c asserts that
# rejection and that the dependency-free control library still loads before and
# after it.
#
# A circular DT_NEEDED pair cannot be produced by a single link -- each library
# must already exist before the other can record a DT_NEEDED entry on it -- so the
# cycle is bootstrapped in stages. A "stage-1" image of one node is linked first
# with NO dependency (its cross-reference to the other node is left undefined,
# which shared objects permit) purely so the other node has something to link
# against; the two final images are then linked against each other. Each final
# link lists its dependency's image directly on the command line under
# --no-as-needed, so the linker records that image's SONAME as a bare DT_NEEDED
# entry that the loader resolves through its default lib/ search path (exactly like
# the diamond fixture above). -soname pins each recorded name regardless of linker
# (GNU ld vs ld.lld). Built with the same i686 freestanding toolchain as the
# fixtures above.
POSIX_TEST_CYCLE_DIR  := $(POSIX_TESTS_OBJDIR)/test-c-dlfcn-cycle/libs
POSIX_TEST_CYCLE_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-cycle-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-cycle := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-cycle.img

# Positive control: dependency-free, zero undefined symbols.
$(POSIX_TEST_CYCLE_DIR)/libok.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-cycle/libs/ok.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-cycle-c/libok.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libok.so $@.o -o $@

# Stage-1 libcycleb.so: no DT_NEEDED yet (its cyclea_value reference is left
# undefined) so libcyclea.so has something to link against. SONAME=libcycleb.so.
$(POSIX_TEST_CYCLE_DIR)/libcycleb-stage1.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-cycle/libs/cycleb.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-cycle-c/libcycleb.so (stage 1, no DT_NEEDED)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libcycleb.so $@.o -o $@

# Final libcyclea.so: DT_NEEDED libcycleb.so (linked against the stage-1 image).
$(POSIX_TEST_CYCLE_DIR)/libcyclea.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-cycle/libs/cyclea.c \
		$(POSIX_TEST_CYCLE_DIR)/libcycleb-stage1.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-cycle-c/libcyclea.so (DT_NEEDED libcycleb.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libcyclea.so \
		$@.o --no-as-needed $(POSIX_TEST_CYCLE_DIR)/libcycleb-stage1.so -o $@

# Final libcycleb.so: DT_NEEDED libcyclea.so (linked against the final
# libcyclea.so), closing the libcyclea.so <-> libcycleb.so cycle.
$(POSIX_TEST_CYCLE_DIR)/libcycleb.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-cycle/libs/cycleb.c \
		$(POSIX_TEST_CYCLE_DIR)/libcyclea.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-cycle-c/libcycleb.so (DT_NEEDED libcyclea.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libcycleb.so \
		$@.o --no-as-needed $(POSIX_TEST_CYCLE_DIR)/libcyclea.so -o $@

# Stage-1 libselfcycle.so: no DT_NEEDED yet. SONAME=libselfcycle.so.
$(POSIX_TEST_CYCLE_DIR)/libselfcycle-stage1.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-cycle/libs/selfcycle.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-cycle-c/libselfcycle.so (stage 1, no DT_NEEDED)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libselfcycle.so $@.o -o $@

# Final libselfcycle.so: DT_NEEDED libselfcycle.so (linked against its own stage-1
# image under --no-as-needed, since it references no symbol from it), forming a
# single-node self-loop.
$(POSIX_TEST_CYCLE_DIR)/libselfcycle.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-cycle/libs/selfcycle.c \
		$(POSIX_TEST_CYCLE_DIR)/libselfcycle-stage1.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building dlfcn-cycle-c/libselfcycle.so (DT_NEEDED libselfcycle.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libselfcycle.so \
		$@.o --no-as-needed $(POSIX_TEST_CYCLE_DIR)/libselfcycle-stage1.so -o $@

# Per-suite RAMFS image carrying the control + cyclic fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-cycle): $(POSIX_TEST_CYCLE_DIR)/libok.so \
		$(POSIX_TEST_CYCLE_DIR)/libcyclea.so \
		$(POSIX_TEST_CYCLE_DIR)/libcycleb.so \
		$(POSIX_TEST_CYCLE_DIR)/libselfcycle.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_CYCLE_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_CYCLE_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_CYCLE_DIR)/libok.so $(POSIX_TEST_CYCLE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_CYCLE_DIR)/libcyclea.so $(POSIX_TEST_CYCLE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_CYCLE_DIR)/libcycleb.so $(POSIX_TEST_CYCLE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_CYCLE_DIR)/libselfcycle.so $(POSIX_TEST_CYCLE_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_CYCLE_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-selflink-c: main-executable GOT/PLT self-linking fixture.
#---------------------------------------------------------------------------------------------------
#
# Unlike the suites above (which dlopen() shared libraries at runtime), this
# suite exercises the FORWARD direction: the main executable itself is linked
# against libprovider.so (-lprovider, see POSIX_TEST_EXTRA_LDLIBS_* above), so
# it carries a DT_NEEDED entry plus an R_386_GLOB_DAT (data) and an
# R_386_JMP_SLOT (function) relocation against the provider's symbols. nvx-crt0's
# self-linker (syscall::dlfcn::dllink_executable) must load libprovider.so into
# the global scope and bind both slots before main() runs. libprovider.so plays
# the role of a "libc.so" and is staged at lib/libprovider.so, reached through
# the loader's default lib/ search path. Built with the same i686 freestanding
# toolchain as the fixtures above; an explicit -soname pins the bare DT_NEEDED
# name regardless of linker (GNU ld vs ld.lld).
POSIX_TEST_SELFLINK_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-selflink-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-selflink := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-selflink.img

# Provider: self-contained (zero undefined symbols), exporting one data symbol
# (-> R_386_GLOB_DAT in the executable) and one function (-> R_386_JMP_SLOT).
$(POSIX_TEST_SELFLINK_DIR)/libprovider.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-selflink/libs/provider.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building test-c-dlfcn-selflink/libprovider.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libprovider.so $@.o -o $@

# Per-suite RAMFS image carrying libprovider.so under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-selflink): $(POSIX_TEST_SELFLINK_DIR)/libprovider.so \
		all-host-binaries-mkramfs
	@$(MKDIR_CMD) $(POSIX_TEST_SELFLINK_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_SELFLINK_DIR)/libprovider.so $(POSIX_TEST_SELFLINK_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_SELFLINK_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-startup-c: real libc.so + libm.so startup auto-load fixture.
#---------------------------------------------------------------------------------------------------
#
# Unlike the fixtures above (purpose-built .so files), this suite stages the
# ACTUAL shared libraries produced by nanvix-libc-bundle (libc.so + libm.so)
# under lib/, reached through the loader's default lib/ search path. The
# executable links against them via DT_NEEDED (see the per-suite hooks above) and
# the startup loader auto-loads them before main(); see
# test-c-dlfcn-startup/main.c.
POSIX_TEST_STARTUP_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-startup-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-startup := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-startup.img

$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-startup): $(LIBRARIES_DIR)/libc.so $(LIBRARIES_DIR)/libm.so \
		all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_STARTUP_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_STARTUP_SEED)/lib
	$(CP_CMD) $(LIBRARIES_DIR)/libc.so $(POSIX_TEST_STARTUP_SEED)/lib/
	$(CP_CMD) $(LIBRARIES_DIR)/libm.so $(POSIX_TEST_STARTUP_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_STARTUP_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-hello-c: "dynamic hello" startup auto-load of real libc.so + libm.so.
#---------------------------------------------------------------------------------------------------
#
# Like dlfcn-startup-c, this suite stages the ACTUAL shared libraries produced by
# nanvix-libc-bundle (libc.so + libm.so) under lib/, reached through the loader's
# default lib/ search path. The executable links against them via DT_NEEDED (see
# the per-suite hooks above) and the startup loader auto-loads them before main();
# see test-c-dlfcn-hello/main.c, which computes and prints "value=42" through that
# dynamic linkage.
POSIX_TEST_HELLO_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-hello-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-hello := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-hello.img

$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-hello): $(LIBRARIES_DIR)/libc.so $(LIBRARIES_DIR)/libm.so \
		all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_HELLO_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_HELLO_SEED)/lib
	$(CP_CMD) $(LIBRARIES_DIR)/libc.so $(POSIX_TEST_HELLO_SEED)/lib/
	$(CP_CMD) $(LIBRARIES_DIR)/libm.so $(POSIX_TEST_HELLO_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_HELLO_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-searchpath-c: default search-path resolution of bare libc.so + libm.so.
#---------------------------------------------------------------------------------------------------
#
# Like dlfcn-startup-c, this suite stages the ACTUAL shared libraries produced by
# nanvix-libc-bundle (libc.so + libm.so) under lib/. The difference is in how the
# suite reaches them: instead of linking the executable against them (DT_NEEDED +
# startup auto-load), dlfcn-searchpath-c dlopen()s the BARE names "libc.so" and
# "libm.so" at run time, so the loader's default lib/ search path
# (syscall::dlfcn::resolve_library_path -> LIBRARY_SEARCH_PATHS) is what locates
# them. This is the acceptance test for the runtime layout + default search path
# work (issue #2775). The suite links PIE + --export-dynamic (see POSIX_TEST_PIE_*
# above) so that libc.so's undefined `__nanvix_main` resolves from the
# executable's exported global scope when RTLD_NOW relocates it; the per-suite
# RAMFS below stages the libraries under lib/.
POSIX_TEST_SEARCHPATH_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-searchpath-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-searchpath := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-searchpath.img

$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-searchpath): $(LIBRARIES_DIR)/libc.so $(LIBRARIES_DIR)/libm.so \
		all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_SEARCHPATH_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_SEARCHPATH_SEED)/lib
	$(CP_CMD) $(LIBRARIES_DIR)/libc.so $(POSIX_TEST_SEARCHPATH_SEED)/lib/
	$(CP_CMD) $(LIBRARIES_DIR)/libm.so $(POSIX_TEST_SEARCHPATH_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_SEARCHPATH_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-initfini-c: startup DT_NEEDED constructor/destructor ordering fixture.
#---------------------------------------------------------------------------------------------------
#
# Ships ONE shared library, libinitfini.so, built with the same i686 freestanding
# toolchain as the fixtures above. It carries a `.init_array` constructor and a
# `.fini_array` destructor and leaves three symbols UNDEFINED — `g_ctor_ran`,
# `g_main_ran` (data) and `test_dtor_finish` (function) — which the loader
# resolves from the main executable's exported global scope at load time (the
# suite ELF is PIE + --export-dynamic, see POSIX_TEST_PIE_* above). The executable
# DT_NEEDEDs it but references none of its symbols, so the startup loader
# auto-loads it for its constructor/destructor side effects alone (no dlopen).
# An explicit -soname pins the bare DT_NEEDED name regardless of linker. Staged
# at lib/libinitfini.so, reached through the loader's default lib/ search path.
POSIX_TEST_INITFINI_SEED := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-initfini-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-initfini := $(BINARIES_DIR)/posix-tests-ramfs-test-c-dlfcn-initfini.img

$(POSIX_TEST_INITFINI_DIR)/libinitfini.so: $(POSIX_TESTS_SRCDIR)/test-c-dlfcn-initfini/libs/initfini.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building test-c-dlfcn-initfini/libinitfini.so (.init_array/.fini_array)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libinitfini.so $@.o -o $@

# Per-suite RAMFS image carrying libinitfini.so under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-initfini): $(POSIX_TEST_INITFINI_DIR)/libinitfini.so \
		all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_INITFINI_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_INITFINI_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_INITFINI_DIR)/libinitfini.so $(POSIX_TEST_INITFINI_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_INITFINI_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-dtor-reentry-c: destructor-time loader re-entrancy fixtures (issue #2538).
#---------------------------------------------------------------------------------------------------
#
# Ships TWO shared libraries (built with the same i686 freestanding toolchain as
# the fixtures above):
#   * libdep.so     - self-contained dependency (zero undefined symbols),
#                     exporting dep_value(). An explicit -soname pins the bare
#                     DT_NEEDED name recorded in libreentry.so to `libdep.so`
#                     regardless of linker (GNU ld vs ld.lld), so the loader
#                     resolves it through the default lib/ search path.
#   * libreentry.so - DT_NEEDED=libdep.so (it references dep_value via -ldep).
#                     Carries a `.fini_array` destructor that calls the main
#                     executable's dtor_probe() -- left UNDEFINED and resolved
#                     from the global scope at load time (the suite ELF is PIE +
#                     --export-dynamic, see POSIX_TEST_PIE_* above).
# Both are staged under lib/, where main.c dlopen()s libreentry.so. A correct
# dlclose() keeps both entries (and the libreentry.so -> libdep.so edge) in the
# registry until the destructors finish, so the destructor-time probe re-resolves
# them instead of mapping a second copy.
POSIX_TEST_DTOR_REENTRY_SUITE  := test-c-dlfcn-dtor-reentry
POSIX_TEST_DTOR_REENTRY_LIBDIR := $(POSIX_TESTS_OBJDIR)/$(POSIX_TEST_DTOR_REENTRY_SUITE)/libs
POSIX_TEST_DTOR_REENTRY_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_DTOR_REENTRY_SUITE)-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-dtor-reentry := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_DTOR_REENTRY_SUITE).img

# libdep.so: self-contained dependency with SONAME=libdep.so.
$(POSIX_TEST_DTOR_REENTRY_LIBDIR)/libdep.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_DTOR_REENTRY_SUITE)/libs/dep.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_DTOR_REENTRY_SUITE)/libdep.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libdep.so $@.o -o $@

# libreentry.so: DT_NEEDED=libdep.so; `.fini_array` destructor calls dtor_probe.
$(POSIX_TEST_DTOR_REENTRY_LIBDIR)/libreentry.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_DTOR_REENTRY_SUITE)/libs/reentry.c \
		$(POSIX_TEST_DTOR_REENTRY_LIBDIR)/libdep.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_DTOR_REENTRY_SUITE)/libreentry.so (DT_NEEDED libdep.so)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o \
		-L$(POSIX_TEST_DTOR_REENTRY_LIBDIR) -ldep -o $@

# Per-suite RAMFS image carrying both fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-dtor-reentry): $(POSIX_TEST_DTOR_REENTRY_LIBDIR)/libdep.so \
		$(POSIX_TEST_DTOR_REENTRY_LIBDIR)/libreentry.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_DTOR_REENTRY_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_DTOR_REENTRY_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_DTOR_REENTRY_LIBDIR)/libdep.so $(POSIX_TEST_DTOR_REENTRY_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_DTOR_REENTRY_LIBDIR)/libreentry.so $(POSIX_TEST_DTOR_REENTRY_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_DTOR_REENTRY_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-init-concurrent-c: constructor-vs-concurrent-dlopen fixture.
#---------------------------------------------------------------------------------------------------
#
# Ships ONE shared library, libslowctor.so, built with the same i686 freestanding
# toolchain as the fixtures above. It carries a `.init_array` constructor whose
# run is deliberately slow and observable, and leaves three symbols UNDEFINED --
# ctor_mark_started / ctor_racer_arrived / ctor_mark_done (functions) -- which the
# loader resolves from the main executable's exported global scope at load time
# (the suite ELF is PIE + --export-dynamic, see POSIX_TEST_PIE_* above). An
# explicit -soname pins the SONAME to the bare name `libslowctor.so`. Staged at
# lib/libslowctor.so, where main.c dlopen()s it from two threads at once: a
# correct loader makes the racing dlopen() wait for the constructor, so the
# racing thread never observes the library before its constructor finished.
POSIX_TEST_INIT_CONCURRENT_SUITE  := test-c-dlfcn-init-concurrent
POSIX_TEST_INIT_CONCURRENT_LIBDIR := $(POSIX_TESTS_OBJDIR)/$(POSIX_TEST_INIT_CONCURRENT_SUITE)/libs
POSIX_TEST_INIT_CONCURRENT_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_INIT_CONCURRENT_SUITE)-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-init-concurrent := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_INIT_CONCURRENT_SUITE).img

# libslowctor.so: slow `.init_array` constructor; SONAME=libslowctor.so.
$(POSIX_TEST_INIT_CONCURRENT_LIBDIR)/libslowctor.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_INIT_CONCURRENT_SUITE)/libs/slowctor.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_INIT_CONCURRENT_SUITE)/libslowctor.so (.init_array)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libslowctor.so $@.o -o $@

# Per-suite RAMFS image carrying libslowctor.so under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-init-concurrent): $(POSIX_TEST_INIT_CONCURRENT_LIBDIR)/libslowctor.so \
		all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_INIT_CONCURRENT_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_INIT_CONCURRENT_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_INIT_CONCURRENT_LIBDIR)/libslowctor.so $(POSIX_TEST_INIT_CONCURRENT_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_INIT_CONCURRENT_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-ctor-dtor-reentry-c: constructor/destructor cross-library re-entrancy.
#---------------------------------------------------------------------------------------------------
#
# Ships TWO shared libraries (built with the same i686 freestanding toolchain as
# the fixtures above):
#   * libother.so - the library opened by libhook.so's constructor and closed by
#                   its destructor. Exports other_value() for the re-entrant
#                   dlsym() checks, and carries a `.fini_array` destructor that
#                   calls the main executable's other_report_dtor() (left
#                   UNDEFINED, resolved from the global scope at load time) so the
#                   suite can confirm the destructor-time dlclose() unloaded it.
#                   An explicit -soname pins the SONAME to the bare name
#                   `libother.so`.
#   * libhook.so  - carries a `.init_array` constructor that calls hook_open_other()
#                   and a `.fini_array` destructor that calls hook_close_other()
#                   (both UNDEFINED, resolved from the main executable's exported
#                   global scope; the suite ELF is PIE + --export-dynamic, see
#                   POSIX_TEST_PIE_* above). Those helpers dlopen()/dlsym()/
#                   dlclose() libother.so from inside the in-progress outer
#                   dlopen()/dlclose(). libhook.so does NOT DT_NEEDED libother.so
#                   -- it reaches it purely through a runtime dlopen() -- so no
#                   -lother link edge is recorded here.
# Both are staged under lib/, where main.c dlopen()s libhook.so; libother.so is
# reached through the loader's default lib/ search path.
POSIX_TEST_CTOR_DTOR_REENTRY_SUITE  := test-c-dlfcn-ctor-dtor-reentry
POSIX_TEST_CTOR_DTOR_REENTRY_LIBDIR := $(POSIX_TESTS_OBJDIR)/$(POSIX_TEST_CTOR_DTOR_REENTRY_SUITE)/libs
POSIX_TEST_CTOR_DTOR_REENTRY_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_CTOR_DTOR_REENTRY_SUITE)-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-ctor-dtor-reentry := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_CTOR_DTOR_REENTRY_SUITE).img

# libother.so: opened/closed by libhook.so's ctor/dtor; SONAME=libother.so.
$(POSIX_TEST_CTOR_DTOR_REENTRY_LIBDIR)/libother.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_CTOR_DTOR_REENTRY_SUITE)/libs/other.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_CTOR_DTOR_REENTRY_SUITE)/libother.so (.fini_array)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libother.so $@.o -o $@

# libhook.so: `.init_array` opens libother.so, `.fini_array` closes it.
$(POSIX_TEST_CTOR_DTOR_REENTRY_LIBDIR)/libhook.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_CTOR_DTOR_REENTRY_SUITE)/libs/hook.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_CTOR_DTOR_REENTRY_SUITE)/libhook.so (.init_array/.fini_array)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libhook.so $@.o -o $@

# Per-suite RAMFS image carrying both fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-ctor-dtor-reentry): $(POSIX_TEST_CTOR_DTOR_REENTRY_LIBDIR)/libother.so \
		$(POSIX_TEST_CTOR_DTOR_REENTRY_LIBDIR)/libhook.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_CTOR_DTOR_REENTRY_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_CTOR_DTOR_REENTRY_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_CTOR_DTOR_REENTRY_LIBDIR)/libother.so $(POSIX_TEST_CTOR_DTOR_REENTRY_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_CTOR_DTOR_REENTRY_LIBDIR)/libhook.so $(POSIX_TEST_CTOR_DTOR_REENTRY_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_CTOR_DTOR_REENTRY_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-hash-c: DT_HASH / DT_GNU_HASH accelerated symbol lookup.
#---------------------------------------------------------------------------------------------------
#
# Ships TWO shared libraries built from the SAME self-contained source (syms.c,
# zero undefined symbols) but with different symbol-hash tables, so the loader's
# find() exercises each of its accelerated lookup paths end to end:
#   * libsyms-sysv.so - linked --hash-style=sysv -> only a .hash (DT_HASH) table.
#   * libsyms-gnu.so  - linked --hash-style=gnu  -> only a .gnu.hash (DT_GNU_HASH)
#                       table (Bloom-filter prefixed).
# main.c dlopen()s both, resolves every exported function plus a data object
# through dlsym() (each result confirms the hash walk returned the correct
# symbol), and asserts an absent name resolves to NULL (the not-found path). The
# fixtures are opened by explicit path, so no SONAME/DT_NEEDED wiring is needed;
# they are staged under lib/, where main.c opens them.
POSIX_TEST_HASH_SUITE  := test-c-dlfcn-hash
POSIX_TEST_HASH_LIBDIR := $(POSIX_TESTS_OBJDIR)/$(POSIX_TEST_HASH_SUITE)/libs
POSIX_TEST_HASH_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_HASH_SUITE)-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-hash := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_HASH_SUITE).img

# libsyms-sysv.so: only a SysV (.hash / DT_HASH) symbol hash table.
$(POSIX_TEST_HASH_LIBDIR)/libsyms-sysv.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_HASH_SUITE)/libs/syms.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_HASH_SUITE)/libsyms-sysv.so (--hash-style=sysv)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) --hash-style=sysv $@.o -o $@

# libsyms-gnu.so: only a GNU (.gnu.hash / DT_GNU_HASH) symbol hash table.
$(POSIX_TEST_HASH_LIBDIR)/libsyms-gnu.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_HASH_SUITE)/libs/syms.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_HASH_SUITE)/libsyms-gnu.so (--hash-style=gnu)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) --hash-style=gnu $@.o -o $@

# Per-suite RAMFS image carrying both fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-hash): $(POSIX_TEST_HASH_LIBDIR)/libsyms-sysv.so \
		$(POSIX_TEST_HASH_LIBDIR)/libsyms-gnu.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_HASH_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_HASH_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_HASH_LIBDIR)/libsyms-sysv.so $(POSIX_TEST_HASH_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_HASH_LIBDIR)/libsyms-gnu.so $(POSIX_TEST_HASH_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_HASH_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-handle-reuse-c: stable-handle / stale-alias regression fixtures.
#---------------------------------------------------------------------------------------------------
#
# Ships TWO self-contained shared libraries (zero undefined symbols), built with
# the same i686 freestanding toolchain as the fixtures above. Both export the
# SAME symbol name library_id() but with a DISTINCT return value:
#   * libalpha.so - library_id() returns 0x0A0A0A0A.
#   * libbeta.so  - library_id() returns 0x0B0B0B0B.
# main.c dlopen()s libalpha.so, dlclose()s it (freeing its file descriptor), then
# dlopen()s libbeta.so -- which the loader typically maps onto the just-freed
# descriptor. The distinct return values make any stale-handle aliasing directly
# observable. The fixtures are opened by explicit path, so no SONAME/DT_NEEDED
# wiring is needed; both are staged under lib/, where main.c opens them.
POSIX_TEST_HANDLE_REUSE_SUITE  := test-c-dlfcn-handle-reuse
POSIX_TEST_HANDLE_REUSE_LIBDIR := $(POSIX_TESTS_OBJDIR)/$(POSIX_TEST_HANDLE_REUSE_SUITE)/libs
POSIX_TEST_HANDLE_REUSE_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_HANDLE_REUSE_SUITE)-seed
POSIX_TEST_RAMFS_IMG_test-c-dlfcn-handle-reuse := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_HANDLE_REUSE_SUITE).img

# libalpha.so: self-contained; library_id() returns 0x0A0A0A0A.
$(POSIX_TEST_HANDLE_REUSE_LIBDIR)/libalpha.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_HANDLE_REUSE_SUITE)/libs/alpha.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_HANDLE_REUSE_SUITE)/libalpha.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# libbeta.so: self-contained; library_id() returns 0x0B0B0B0B.
$(POSIX_TEST_HANDLE_REUSE_LIBDIR)/libbeta.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_HANDLE_REUSE_SUITE)/libs/beta.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_HANDLE_REUSE_SUITE)/libbeta.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# Per-suite RAMFS image carrying both fixtures under lib/.
$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-handle-reuse): $(POSIX_TEST_HANDLE_REUSE_LIBDIR)/libalpha.so \
		$(POSIX_TEST_HANDLE_REUSE_LIBDIR)/libbeta.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_HANDLE_REUSE_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_HANDLE_REUSE_SEED)/lib
	$(CP_CMD) $(POSIX_TEST_HANDLE_REUSE_LIBDIR)/libalpha.so $(POSIX_TEST_HANDLE_REUSE_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_HANDLE_REUSE_LIBDIR)/libbeta.so $(POSIX_TEST_HANDLE_REUSE_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_HANDLE_REUSE_SEED)

# All per-suite RAMFS images (built on demand by the runner).
POSIX_TEST_SOLIB_IMGS := $(foreach suite,$(POSIX_TEST_SOLIB_SUITES),$(POSIX_TEST_RAMFS_IMG_$(suite))) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-ctor-dtor-reentry) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-cycle) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-diamond) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-order) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-dlclose-cycle) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-dtor-reentry) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-handle-reuse) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-hash) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-init-concurrent) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-hello) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-scope) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-searchpath) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-selflink) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-startup) \
	$(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-initfini)

#---------------------------------------------------------------------------------------------------
# dlfcn-init-runpath-c: constructor/destructor + DT_RUNPATH fixtures.
#---------------------------------------------------------------------------------------------------
#
# This suite ships THREE shared libraries (built with the same freestanding host
# toolchain as the global/needed fixtures above):
#   * libctor.so   - carries `.init_array`/`.fini_array` (a constructor and a
#                    destructor). The destructor stores a sentinel into the main
#                    executable's exported `g_dtor_ran` global, so libctor.so is
#                    left with an UNDEFINED `g_dtor_ran` that the loader resolves
#                    from the global scope (the suite ELF is PIE +
#                    --export-dynamic, see POSIX_TEST_PIE_* above). Staged at
#                    lib/libctor.so.
#   * libchild.so  - a self-contained dependency, given an explicit SONAME so the
#                    DT_NEEDED entry recorded in libparent.so is the bare name
#                    `libchild.so` regardless of linker (GNU ld vs ld.lld). Staged
#                    ONLY at lib/subdir/libchild.so (never lib/).
#   * libparent.so - DT_NEEDED=libchild.so (via -lchild) and DT_RUNPATH=lib/subdir
#                    (via --enable-new-dtags -rpath lib/subdir). --enable-new-dtags
#                    forces DT_RUNPATH instead of the deprecated DT_RPATH, which
#                    the Nanvix loader intentionally ignores. The loader must
#                    consult DT_RUNPATH to locate libchild.so. Staged at
#                    lib/libparent.so.

POSIX_TEST_RUNPATH_SUITE  := test-c-dlfcn-init-runpath
POSIX_TEST_RUNPATH_LIBDIR := $(POSIX_TESTS_OBJDIR)/$(POSIX_TEST_RUNPATH_SUITE)/libs
POSIX_TEST_RUNPATH_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_RUNPATH_SUITE)-seed
POSIX_TEST_RUNPATH_IMG    := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_RUNPATH_SUITE).img

# libctor.so: constructor/destructor witness. The undefined `g_dtor_ran` is left
# for the loader to resolve from the main executable's global scope at load time.
$(POSIX_TEST_RUNPATH_LIBDIR)/libctor.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_RUNPATH_SUITE)/libs/ctor.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_RUNPATH_SUITE)/libctor.so (.init_array/.fini_array)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o -o $@

# libchild.so: self-contained dependency with SONAME=libchild.so.
$(POSIX_TEST_RUNPATH_LIBDIR)/libchild.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_RUNPATH_SUITE)/libs/subdir/child.c
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_RUNPATH_SUITE)/libchild.so"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) -soname libchild.so $@.o -o $@

# libparent.so: DT_NEEDED=libchild.so, DT_RUNPATH=lib/subdir.
$(POSIX_TEST_RUNPATH_LIBDIR)/libparent.so: $(POSIX_TESTS_SRCDIR)/$(POSIX_TEST_RUNPATH_SUITE)/libs/parent.c \
		$(POSIX_TEST_RUNPATH_LIBDIR)/libchild.so
	@$(MKDIR_CMD) $(dir $@)
	@echo "[posix-test] building $(POSIX_TEST_RUNPATH_SUITE)/libparent.so (DT_NEEDED libchild.so, DT_RUNPATH lib/subdir)"
	$(GUEST_C_APP_CC) $(POSIX_TEST_SOLIB_CFLAGS) -c $< -o $@.o
	$(GUEST_C_APP_LD) $(POSIX_TEST_SOLIB_LDFLAGS) $@.o \
		-L$(POSIX_TEST_RUNPATH_LIBDIR) -lchild --enable-new-dtags -rpath lib/subdir -o $@

# Per-suite RAMFS image: libctor.so + libparent.so under lib/, libchild.so under
# lib/subdir/ (reachable ONLY via libparent.so's DT_RUNPATH).
$(POSIX_TEST_RUNPATH_IMG): $(POSIX_TEST_RUNPATH_LIBDIR)/libctor.so \
		$(POSIX_TEST_RUNPATH_LIBDIR)/libparent.so \
		$(POSIX_TEST_RUNPATH_LIBDIR)/libchild.so all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_RUNPATH_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_RUNPATH_SEED)/lib/subdir
	$(CP_CMD) $(POSIX_TEST_RUNPATH_LIBDIR)/libctor.so $(POSIX_TEST_RUNPATH_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_RUNPATH_LIBDIR)/libparent.so $(POSIX_TEST_RUNPATH_SEED)/lib/
	$(CP_CMD) $(POSIX_TEST_RUNPATH_LIBDIR)/libchild.so $(POSIX_TEST_RUNPATH_SEED)/lib/subdir/
	$(MKRAMFS) -o $@ $(POSIX_TEST_RUNPATH_SEED)

#---------------------------------------------------------------------------------------------------
# dlfcn-weak-c: STB_WEAK undefined-symbol fixtures.
#---------------------------------------------------------------------------------------------------
#
# This suite ships SEVEN shared libraries built from FOUR sources, each compiled
# with different -D defines, to exercise the loader's handling of weak undefined
# symbols across both i686 relocation classes (built with the same freestanding
# host toolchain as the global/needed fixtures above):
#   * libweak-func-resolved.so / libweak-func-missing.so  (weak_func.c)
#       NULL-guarded weak function ref  -> R_386_GLOB_DAT.
#   * libweak-data-resolved.so / libweak-data-missing.so  (weak_data.c)
#       NULL-guarded weak data ref      -> R_386_GLOB_DAT.
#   * libweak-plt-resolved.so / libweak-plt-missing.so    (weak_func_plt.c)
#       unguarded weak function call    -> R_386_JUMP_SLOT.
#   * libstrong-missing.so                                (strong_missing.c)
#       unguarded strong undefined call -> R_386_JUMP_SLOT (regression guard).
#
# The "resolved" variants reference the symbol names the main executable exports
# (main_callback/weak_data); the "missing" variants reference names nothing
# defines, so the loader must zero them per the STB_WEAK rule. The suite ELF is
# PIE + --export-dynamic (POSIX_TEST_PIE_dlfcn-weak-c above) so its
# main_callback/weak_data land in .dynsym for the resolved cases.

POSIX_TEST_WEAK_SUITE  := test-c-dlfcn-weak
POSIX_TEST_WEAK_LIBDIR := $(POSIX_TESTS_OBJDIR)/$(POSIX_TEST_WEAK_SUITE)/libs
POSIX_TEST_WEAK_SEED   := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_WEAK_SUITE)-seed
POSIX_TEST_WEAK_IMG    := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_WEAK_SUITE).img

# $(1) = output .so name; $(2) = source file under libs/; $(3) = extra -D defines.
define POSIX_TEST_WEAK_SOLIB_RULE
$$(POSIX_TEST_WEAK_LIBDIR)/$(1): $$(POSIX_TESTS_SRCDIR)/$$(POSIX_TEST_WEAK_SUITE)/libs/$(2)
	@$$(MKDIR_CMD) $$(dir $$@)
	@echo "[posix-test] building $$(POSIX_TEST_WEAK_SUITE)/$(1)"
	$$(GUEST_C_APP_CC) $$(POSIX_TEST_SOLIB_CFLAGS) $(3) -c $$< -o $$@.o
	$$(GUEST_C_APP_LD) $$(POSIX_TEST_SOLIB_LDFLAGS) $$@.o -o $$@
endef

$(eval $(call POSIX_TEST_WEAK_SOLIB_RULE,libweak-func-resolved.so,weak_func.c,-DCALLBACK_NAME=main_callback))
$(eval $(call POSIX_TEST_WEAK_SOLIB_RULE,libweak-func-missing.so,weak_func.c,-DCALLBACK_NAME=missing_callback))
$(eval $(call POSIX_TEST_WEAK_SOLIB_RULE,libweak-data-resolved.so,weak_data.c,-DWEAK_DATA_NAME=weak_data))
$(eval $(call POSIX_TEST_WEAK_SOLIB_RULE,libweak-data-missing.so,weak_data.c,-DWEAK_DATA_NAME=missing_weak_data))
$(eval $(call POSIX_TEST_WEAK_SOLIB_RULE,libweak-plt-resolved.so,weak_func_plt.c,-DCALLBACK_NAME=main_callback))
$(eval $(call POSIX_TEST_WEAK_SOLIB_RULE,libweak-plt-missing.so,weak_func_plt.c,-DCALLBACK_NAME=missing_plt_callback))
$(eval $(call POSIX_TEST_WEAK_SOLIB_RULE,libstrong-missing.so,strong_missing.c,))

# The seven fixtures, in main.c's dlopen() order.
POSIX_TEST_WEAK_LIBS := \
	libstrong-missing.so \
	libweak-func-resolved.so libweak-func-missing.so \
	libweak-data-resolved.so libweak-data-missing.so \
	libweak-plt-resolved.so libweak-plt-missing.so

# Per-suite RAMFS image carrying all seven fixtures under lib/, where main.c
# dlopen()s them.
$(POSIX_TEST_WEAK_IMG): $(foreach lib,$(POSIX_TEST_WEAK_LIBS),$(POSIX_TEST_WEAK_LIBDIR)/$(lib)) \
		all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_WEAK_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_WEAK_SEED)/lib
	$(CP_CMD) $(foreach lib,$(POSIX_TEST_WEAK_LIBS),$(POSIX_TEST_WEAK_LIBDIR)/$(lib)) \
		$(POSIX_TEST_WEAK_SEED)/lib/
	$(MKRAMFS) -o $@ $(POSIX_TEST_WEAK_SEED)

#---------------------------------------------------------------------------------------------------
# execvp-c: PATH-search fixture.
#---------------------------------------------------------------------------------------------------
#
# test-c-execvp validates execvp()'s PATH search. The suite is a dual-role ELF:
# booted as the driver it fork()s and execvp()s a copy of itself, and that copy
# (re-exec'd with argv[1]=="execvp-child") plays the target role. The target must
# be reachable through PATH, so the suite ELF is staged a second time in this
# per-suite RAMFS image as /bin/prog; the driver sets PATH=/bin and asserts the
# child it execvp()s exits with the target's sentinel code. /bin/prog IS the
# suite ELF, so no extra build step is needed beyond copying it into the seed.

POSIX_TEST_EXECVP_SUITE := test-c-execvp
POSIX_TEST_EXECVP_SEED  := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_EXECVP_SUITE)-seed
POSIX_TEST_EXECVP_IMG   := $(BINARIES_DIR)/posix-tests-ramfs-$(POSIX_TEST_EXECVP_SUITE).img

$(POSIX_TEST_EXECVP_IMG): $(BINARIES_DIR)/$(POSIX_TEST_EXECVP_SUITE).$(EXEC_FORMAT) \
		all-host-binaries-mkramfs
	$(FORCE_RM_CMD) $(POSIX_TEST_EXECVP_SEED)
	@$(MKDIR_CMD) $(POSIX_TEST_EXECVP_SEED)/bin
	$(CP_CMD) $(BINARIES_DIR)/$(POSIX_TEST_EXECVP_SUITE).$(EXEC_FORMAT) \
		$(POSIX_TEST_EXECVP_SEED)/bin/prog
	$(MKRAMFS) -o $@ $(POSIX_TEST_EXECVP_SEED)

# The per-suite boot arguments that pair each suite with its RAMFS image
# (`-ramfs <img>`) or enable host networking (`-allow-host-networking`) now live
# in the nanvix-test harness configs (test/test-posix.toml and
# test/test-posix-windows.toml) as `extra_nanvixd_args`, since the harness — not
# this makefile — boots the suites. This makefile only builds the images above.

#---------------------------------------------------------------------------------------------------
# Aggregate image target.
#---------------------------------------------------------------------------------------------------

# Builds every suite's bootable `<suite>.initrd` plus the RAMFS images the
# file-system and dlopen suites need (the shared image and the per-suite
# global/needed fixtures). This is the build-side entry point used wherever
# `run-posix-tests` is skipped — notably Windows, where suites are booted
# manually under WHP (see the repo notes). `run-posix-tests` depends on the same
# set, so this also pre-stages everything the Linux runner consumes.
# The dlfcn shared-library fixtures (the global/needed/diamond/selflink/initfini
# RAMFS images, the init-runpath image, and the weak-symbol image) now build for
# every guest ABI: the fixture `.so` follow the active TARGET
# (`POSIX_TEST_SOLIB_*`), and the dlfcn suites that consume them run on x86 and
# x86_64. The startup suite's image is excluded on x86_64 because it stages the
# real libc.so/libm.so, which cannot be built as x86-64 shared objects (see the
# i686-only note in build/make/lists/guest-posix-tests.mk). Basic test-c-dlfcn /
# test-c-dlfcn-pie stay i686-only (they dlopen the prebuilt i386 libmul.so).
ifeq ($(TARGET),x86_64)
POSIX_TEST_DLFCN_FIXTURE_IMGS := $(filter-out $(POSIX_TEST_RAMFS_IMG_test-c-dlfcn-startup),$(POSIX_TEST_SOLIB_IMGS)) $(POSIX_TEST_RUNPATH_IMG) $(POSIX_TEST_STAGING_IMG) $(POSIX_TEST_WEAK_IMG)
else
POSIX_TEST_DLFCN_FIXTURE_IMGS := $(POSIX_TEST_SOLIB_IMGS) $(POSIX_TEST_RUNPATH_IMG) $(POSIX_TEST_STAGING_IMG) $(POSIX_TEST_WEAK_IMG)
endif

.PHONY: all-posix-test-images
all-posix-test-images: $(POSIX_TEST_INITRDS) \
		$(if $(strip $(POSIX_TEST_RAMFS_SUITES)),$(POSIX_TEST_RAMFS_IMG)) \
		$(POSIX_TEST_DLFCN_FIXTURE_IMGS) \
		$(POSIX_TEST_EXECVP_IMG)
	@echo "All POSIX C test-suite images built."

.PHONY: run-posix-tests

# The boot runner drives the suites through the nanvix-test harness, which boots
# each <suite>.initrd under nanvixd in standalone mode (the `terminal` executor)
# and asserts a guest exit code of 0. The harness is cross-platform: on Linux it
# launches nanvixd directly; on Windows it launches nanvixd.exe under WHP. The
# suites build for every guest ABI (TARGET=x86 and x86_64); the i686-only suites
# listed in POSIX_TESTS_X86_ONLY are gated
# to x86 through their per-test `targets` field. The suites are
# standalone-only (they bundle the guest daemons). On unsupported targets the
# portable suites can still be built with `all-posix-tests`.

# Harness configuration: Windows uses the .exe nanvixd and a `.`-rooted temp dir.
ifeq ($(IS_WINDOWS),yes)
POSIX_TEST_CONFIG := test/test-posix-windows.toml
else
POSIX_TEST_CONFIG := test/test-posix.toml
endif

# Optional test sharding. When SHARD is set to INDEX/TOTAL (e.g. SHARD=1/4), the
# harness runs only that disjoint, round-robin slice of the suites via its
# `-shard` option, letting CI partition the POSIX suites across independent
# runners. The selector must precede the config path (the harness treats the
# last argument as the config file). Leave SHARD empty to run every suite.
POSIX_TEST_SHARD_FLAG := $(if $(strip $(SHARD)),-shard $(strip $(SHARD)))

ifeq ($(filter $(TARGET),x86 x86_64),)
run-posix-tests:
	@echo "Skipping POSIX C test suites (no guest C toolchain for TARGET=$(TARGET))."
else
run-posix-tests: $(POSIX_HEADERS_CXX_STAMP) $(POSIX_TEST_INITRDS) $(if $(strip $(POSIX_TEST_RAMFS_SUITES)),$(POSIX_TEST_RAMFS_IMG)) $(POSIX_TEST_DLFCN_FIXTURE_IMGS) $(POSIX_TEST_EXECVP_IMG)
	@test -f $(NANVIX_TEST_BIN) || { echo "ERROR: $(NANVIX_TEST_BIN) missing; run './z build -- all' first."; exit 1; }
	@test -f $(NANVIXD) || { echo "ERROR: $(NANVIXD) missing; run './z build -- all' first."; exit 1; }
	@test -f $(KERNEL) || { echo "ERROR: $(KERNEL) missing; run './z build -- all' first."; exit 1; }
	@test -f $(USERVM) || { echo "ERROR: $(USERVM) missing; run './z build -- all' first."; exit 1; }
	@echo "Running ported POSIX C test suites with configuration: $(POSIX_TEST_CONFIG)$(if $(strip $(SHARD)), (shard $(strip $(SHARD))))"
	RUST_LOG=$(LOG_LEVEL) $(NANVIX_TEST_BIN) $(POSIX_TEST_SHARD_FLAG) $(POSIX_TEST_CONFIG)
endif
