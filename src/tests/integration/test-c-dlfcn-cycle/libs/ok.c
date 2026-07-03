/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libok.so - dependency-free positive-control library for dlfcn-cycle-c.
 *
 * Carries zero DT_NEEDED edges and zero undefined symbols, so a correct
 * loader dlopen()s it without walking any dependencies. The suite loads it
 * FIRST (and again after the cyclic loads fail) to prove the per-suite
 * RAMFS is mounted and the loader resolves a well-formed library. Without
 * this control a later "dlopen(cyclic) == NULL" assertion could pass for
 * the wrong reason -- e.g. the library file was never found -- instead of
 * because the loader rejected a DT_NEEDED cycle.
 */

int ok_value(void)
{
    return 0x2479;
}
