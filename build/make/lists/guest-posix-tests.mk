# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Ported POSIX C test suites, compiled against the bundled libc by
# build/make/posix-tests.mk and booted by `run-posix-tests`. Suites live under
# src/tests/integration/<suite>/, except memory-c which lives under
# src/tests/stress/<suite>/ (see POSIX_TEST_STRESS_* in posix-tests.mk).
# `common/` holds shared crt0 scaffolding and is not a suite of its own. The
# guest C toolchain (build/make/guest-c-apps.mk) is pinned to the i686 ABI
# (-m32 / -melf_i386), so the suites are i686-only; the `run-posix-tests` runner
# is gated on TARGET=x86 accordingly.
#
# One entry per line (with `\` continuations) so that adding a suite touches a
# single line, minimizing merge conflicts between branches that each add a suite.
ALL_POSIX_TESTS := \
	test-c-bindings \
	test-c-ctor \
	test-c-dlfcn \
	test-c-dlfcn-ctor-dtor-reentry \
	test-c-dlfcn-cycle \
	test-c-dlfcn-diamond \
	test-c-dlfcn-dtor-reentry \
	test-c-dlfcn-global \
	test-c-dlfcn-handle-reuse \
	test-c-dlfcn-hash \
	test-c-dlfcn-hello \
	test-c-dlfcn-init-concurrent \
	test-c-dlfcn-init-runpath \
	test-c-dlfcn-initfini \
	test-c-dlfcn-needed \
	test-c-dlfcn-pie \
	test-c-dlfcn-refcount \
	test-c-dlfcn-searchpath \
	test-c-dlfcn-selflink \
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
	test-c-setjmp \
	test-c-sigmask \
	test-c-stdio \
	test-c-termios \
	test-c-thread \
	test-c-wchar
