/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libcycleb.so - the other node of the two-node DT_NEEDED cycle exercised by
 * dlfcn-cycle-c:
 *
 *   libcyclea.so DT_NEEDED libcycleb.so
 *   libcycleb.so DT_NEEDED libcyclea.so
 *
 * cycleb_value() references libcyclea.so's cyclea_value(), which forces the
 * DT_NEEDED edge on libcyclea.so at link time. The call is never actually
 * executed: the loader rejects the cycle while walking the dependency graph,
 * so the otherwise-mutual recursion below is unreachable.
 */

extern int cyclea_value(void);

int cycleb_value(void)
{
    return cyclea_value() + 1;
}
