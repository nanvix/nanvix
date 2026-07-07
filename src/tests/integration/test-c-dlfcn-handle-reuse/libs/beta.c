/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libbeta.so - Self-contained shared library (zero undefined symbols).
 *
 * Exports library_id() returning a sentinel that is DISTINCT from the one
 * libalpha.so returns for the same symbol name. The handle-reuse suite loads
 * this library onto the file descriptor just freed by unloading libalpha.so, so
 * resolving library_id() through libalpha.so's stale handle must never return
 * this value.
 */

int library_id(void)
{
    return 0x0B0B0B0B;
}
