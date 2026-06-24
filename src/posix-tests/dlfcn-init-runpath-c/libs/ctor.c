/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libctor.so - shared library that exercises `.init_array` and
 * `.fini_array` semantics required by the System V gABI.
 *
 * The constructor sets `ctor_ran` to a known sentinel value, and the
 * destructor writes a different sentinel into the test program's
 * `g_dtor_ran` global so that observation outlives the unloaded
 * library. `g_dtor_ran` is declared `extern` here and resolved by the
 * Nanvix loader's global symbol table (the main executable was linked
 * with `--export-dynamic`).
 */

/* Public state exposed via dlsym so the test can read it after dlopen. */
volatile int ctor_ran = 0;

/* Main executable owns the destructor witness. */
extern volatile int g_dtor_ran;

static void __attribute__((constructor)) my_ctor(void)
{
    ctor_ran = 0xC70A;
}

static void __attribute__((destructor)) my_dtor(void)
{
    g_dtor_ran = 0xD70A;
}

/* Sanity entry point so the test can confirm dlsym still works. */
int ctor_value(void)
{
    return 42;
}
