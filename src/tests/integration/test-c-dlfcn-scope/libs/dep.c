/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libdep.so - self-contained DT_NEEDED dependency of libfoo.so.
 *
 * Exports scope_dep_value(). Because libfoo.so is linked against this library
 * (DT_NEEDED) and calls scope_dep_value(), this symbol belongs to libfoo.so's
 * load group and must therefore be resolvable through
 * dlsym(libfoo_handle, "scope_dep_value"). The return value (111) is asserted
 * by the suite's main.c (DEP_VALUE).
 */

int scope_dep_value(void)
{
    return 111;
}
