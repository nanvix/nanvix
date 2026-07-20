/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libalpha.so - Self-contained shared library (zero undefined symbols).
 *
 * Exports library_id() returning a sentinel that is DISTINCT from the one
 * libbeta.so returns for the same symbol name. The handle-reuse suite loads
 * this library first, then unloads it (freeing its file descriptor) before
 * loading libbeta.so, so a mismatched return value would reveal a stale handle
 * silently aliasing the other library.
 */

int library_id(void)
{
    return 0x0A0A0A0A;
}
