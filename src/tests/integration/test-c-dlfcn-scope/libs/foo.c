/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libfoo.so - the library the test obtains a handle to.
 *
 * Exports scope_foo_value() and carries a DT_NEEDED entry on libdep.so (it
 * calls scope_dep_value()). It deliberately neither defines nor references the
 * main executable's scope_main_export() nor libother.so's scope_other_value(),
 * so those names are absent from libfoo.so's load group. A non-NULL
 * dlsym(libfoo_handle, "scope_main_export" | "scope_other_value") could only
 * come from an incorrect global-scope fallback.
 */

extern int scope_dep_value(void);

int scope_foo_value(void)
{
    /* Calling into libdep.so forces the DT_NEEDED edge and proves the
     * dependency is part of this object's load group. Result: 111 + 100 = 211
     * (FOO_VALUE in the suite's main.c). */
    return scope_dep_value() + 100;
}
