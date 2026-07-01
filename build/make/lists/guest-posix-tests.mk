# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Ported POSIX C test suites, compiled against the bundled libc by
# build/make/posix-tests.mk and booted by `run-posix-tests`. Suites live under
# src/tests/integration/<suite>/, except memory-c which lives under
# src/tests/stress/<suite>/ (see POSIX_TEST_STRESS_* in posix-tests.mk).
# `common/` holds shared crt0 scaffolding and is not a suite of its own.
#
# The guest C toolchain (build/make/guest-c-apps.mk) follows the active TARGET:
# i686 for x86 and x86-64 for x86_64. Suites that build and run on every guest
# ABI live in ALL_POSIX_TESTS; suites pinned to the i686 ABI are listed in
# POSIX_TESTS_X86_ONLY and appended to ALL_POSIX_TESTS only for x86 builds. The
# x86_64 `run-posix-tests` run skips the i686-only suites through the per-test
# `targets = ["x86"]` gate in test/test-posix.toml.
#
# One entry per line (with `\` continuations) so that adding a suite touches a
# single line, minimizing merge conflicts between branches that each add a suite.
ALL_POSIX_TESTS := \
	test-c-bindings \
	test-c-cxa-atexit \
	test-c-ctor \
	test-c-dlfcn-ctor-dtor-reentry \
	test-c-dlfcn-cycle \
	test-c-dlfcn-diamond \
	test-c-dlfcn-dlclose-cycle \
	test-c-dlfcn-dtor-reentry \
	test-c-dlfcn-global \
	test-c-dlfcn-handle-reuse \
	test-c-dlfcn-hash \
	test-c-dlfcn-hello \
	test-c-dlfcn-init-concurrent \
	test-c-dlfcn-init-runpath \
	test-c-dlfcn-initfini \
	test-c-dlfcn-needed \
	test-c-dlfcn-order \
	test-c-dlfcn-scope \
	test-c-dlfcn-searchpath \
	test-c-dlfcn-selflink \
	test-c-dlfcn-staging \
	test-c-dlfcn-startup \
	test-c-dlfcn-weak \
	test-c-echo \
	test-c-execvp \
	test-c-file \
	test-c-fork-pid \
	test-c-fork-pthread \
	test-c-glob \
	test-c-headers \
	test-c-hello \
	test-c-inet \
	test-c-libgen \
	test-c-locale \
	test-c-math \
	test-c-memory \
	test-c-misc \
	test-c-netdb \
	test-c-network \
	test-c-noop \
	test-c-pathconf \
	test-c-posix-timers \
	test-c-regex \
	test-c-send \
	test-c-setjmp \
	test-c-sigmask \
	test-c-stdio \
	test-c-termios \
	test-c-thread \
	test-c-wchar

# Suites pinned to the i686 guest ABI (TARGET=x86), appended to ALL_POSIX_TESTS
# only for x86 builds:
#  - test-c-dlfcn, test-c-dlfcn-pie, and test-c-dlfcn-refcount dlopen the
#    prebuilt libmul.so / libmul-pie.so fixtures, which are checked-in i386
#    shared objects built from i386 inline assembly (see
#    src/tests/integration/test-rust-dlfcn); no x86-64 build of those fixtures
#    exists yet.
# The rest of the dlfcn family builds its own per-ABI fixtures (or, for the
# startup/hello/searchpath suites, links the real libc.so/libm.so now that those
# are produced as x86-64 PIC shared objects) and runs on both guest ABIs.
POSIX_TESTS_X86_ONLY := \
	test-c-dlfcn \
	test-c-dlfcn-pie \
	test-c-dlfcn-refcount

ifneq ($(TARGET),x86_64)
ALL_POSIX_TESTS += $(POSIX_TESTS_X86_ONLY)
endif
