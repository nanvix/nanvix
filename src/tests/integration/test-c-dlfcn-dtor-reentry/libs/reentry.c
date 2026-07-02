/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libreentry.so - primary fixture for dlfcn-dtor-reentry-c.
 *
 * Carries a `.fini_array` destructor that runs during dlclose() teardown and
 * delegates to the main executable's dtor_probe(), which re-enters the loader
 * to prove this library and its dependency stay discoverable until destructors
 * finish. The library DT_NEEDEDs libdep.so (reentry_dep_value() references
 * dep_value(), so the linker records the edge), leaves dtor_probe() UNDEFINED
 * (resolved from the main executable's exported global scope at load time, as
 * the suite ELF is PIE + --export-dynamic), and exports reentry_value() so the
 * destructor-time probe can resolve it via dlsym().
 */

/* Defined in libdep.so; referenced here so the linker records a DT_NEEDED entry
 * on libdep.so and the loader keeps the edge to resolve during teardown. */
extern int dep_value(void);

/* Defined in the main executable and exported via --export-dynamic; resolved
 * from the loader's global symbol table at load time. Called from this
 * library's destructor while dlclose() is unloading it. */
extern void dtor_probe(void);

/* Exported so the destructor-time probe can resolve it via dlsym(). */
int reentry_value(void)
{
    return 1234;
}

/* Exported helper that references the dependency, forcing the DT_NEEDED edge on
 * libdep.so (and giving the probe a dependency symbol to resolve). */
int reentry_dep_value(void)
{
    return dep_value();
}

/* Runs from `.fini_array` during dlclose() teardown. */
static void __attribute__((destructor)) reentry_dtor(void)
{
    dtor_probe();
}
