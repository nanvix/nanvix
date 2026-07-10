/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libalpha.so - one of two providers that export the SAME symbol
 * `provider_id()`. It is the SECOND entry in libroot.so's DT_NEEDED list but
 * the FIRST when the dependency names are sorted alphabetically
 * ("libalpha.so" < "libbravo.so").
 *
 * A loader that searches dependencies in DT_NEEDED order (as POSIX/System V
 * require) must NOT pick this library's `provider_id()` for a lookup made
 * through libroot.so, because libbravo.so precedes it in DT_NEEDED order. A
 * loader that instead searches alphabetically WOULD pick this one -- the bug
 * exercised by the test in ../main.c.
 */

/* Identity of this provider. Distinct from libbravo.so's so the test can tell
 * which library a lookup resolved to. */
int provider_id(void)
{
    return 1;
}

/* Marker unique to this library. Referenced by libroot.so purely to force a
 * DT_NEEDED entry on libalpha.so regardless of the linker's as-needed default. */
int alpha_marker(void)
{
    return 0xA;
}
