/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libbravo.so - one of two providers that export the SAME symbol
 * `provider_id()`. It is the FIRST entry in libroot.so's DT_NEEDED list but the
 * SECOND when the dependency names are sorted alphabetically
 * ("libalpha.so" < "libbravo.so").
 *
 * A loader that searches dependencies in DT_NEEDED order (as POSIX/System V
 * require) must resolve a `provider_id()` lookup made through libroot.so to
 * THIS library, because libbravo.so precedes libalpha.so in DT_NEEDED order.
 * The test in ../main.c asserts exactly that.
 */

/* Identity of this provider. Distinct from libalpha.so's so the test can tell
 * which library a lookup resolved to. */
int provider_id(void)
{
    return 2;
}

/* Marker unique to this library. Referenced by libroot.so purely to force a
 * DT_NEEDED entry on libbravo.so regardless of the linker's as-needed default. */
int bravo_marker(void)
{
    return 0xB;
}
