/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libcyclea.so - one node of the two-node DT_NEEDED cycle exercised by
 * dlfcn-cycle-c:
 *
 *   libcyclea.so DT_NEEDED libcycleb.so
 *   libcycleb.so DT_NEEDED libcyclea.so
 *
 * cyclea_value() references libcycleb.so's cycleb_value(), which forces the
 * DT_NEEDED edge on libcycleb.so at link time. The call is never actually
 * executed: the loader must reject dlopen("libcyclea.so") while walking the
 * dependency graph -- before any relocation or code in this library runs --
 * so the otherwise-mutual recursion below is unreachable.
 */

extern int cycleb_value(void);

int cyclea_value(void)
{
    return cycleb_value() + 1;
}
