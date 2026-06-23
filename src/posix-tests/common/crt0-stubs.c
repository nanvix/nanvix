/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Minimal `_init`/`_fini` glue shared by every ported POSIX C test suite.
 *
 * The `c-main` startup driver (nvx-crt0's `__nanvix_main`) NO LONGER calls
 * `_init()`/`_fini()`: with the in-tree `nanvix_libc` bundle (which enables the
 * `init-array` feature) global constructors and destructors instead run via
 * `.init_array`/`.fini_array`, walked by `__nanvix_libc_start_main` (see
 * src/libs/posix/src/start.rs). These empty stubs therefore resolve nothing and
 * are normally unreferenced; they are kept only as harmless legacy scaffolding
 * so any not-yet-updated port object that still references `_init`/`_fini`
 * links cleanly (the host-clang posix-tests link path pulls in no
 * `crti.o`/`crtn.o`). It is compiled once and linked into each suite ELF (it is
 * not a suite of its own).
 */

void _init(void)
{
}

void _fini(void)
{
}
