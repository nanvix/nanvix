/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libother.so - the library that libhook.so's constructor dlopen()s and whose
 * handle libhook.so's destructor dlclose()s, for dlfcn-ctor-dtor-reentry-c.
 *
 * Exports other_value() so the re-entrant dlsym() checks have a symbol to
 * resolve, and carries a `.fini_array` destructor that reports back to the main
 * executable through other_report_dtor() -- left UNDEFINED here and resolved
 * from the loader's global scope at load time (the suite ELF is PIE +
 * --export-dynamic). That report lets the suite confirm the destructor-time
 * dlclose() actually unloaded this library rather than leaving it resident.
 */

/* Value the re-entrant dlsym() checks expect; must match main.c. */
#define OTHER_VALUE 0x2474

/* Defined in the main executable, exported via --export-dynamic; resolved from
 * the loader's global symbol table at load time. Called from this library's
 * destructor while dlclose() (itself invoked from libhook.so's destructor) tears
 * it down. */
extern void other_report_dtor(void);

/* Exported so the re-entrant dlsym() checks can resolve it. */
int other_value(void)
{
    return OTHER_VALUE;
}

/* Runs from `.fini_array` when libother.so is unloaded. */
static void __attribute__((destructor)) other_dtor(void)
{
    other_report_dtor();
}
