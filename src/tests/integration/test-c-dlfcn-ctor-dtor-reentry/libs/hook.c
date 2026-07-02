/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libhook.so - driver fixture for dlfcn-ctor-dtor-reentry-c.
 *
 * Models a library that manages another library's lifecycle across its own
 * constructor and destructor:
 *   - its `.init_array` constructor calls hook_open_other(), which dlopen()s
 *     libother.so from inside the still-in-progress outer dlopen() (a
 *     constructor calling dlopen);
 *   - its `.fini_array` destructor calls hook_close_other(), which dlsym()s and
 *     then dlclose()s that same libother.so handle from inside the
 *     in-progress dlclose() (a destructor calling dlsym / dlclose on another
 *     library).
 *
 * hook_open_other / hook_close_other are defined in the main executable and left
 * UNDEFINED here; the loader resolves them from the global scope at load time
 * (the suite ELF is PIE + --export-dynamic). Keeping the loader re-entry logic in
 * the executable mirrors the sibling dlfcn re-entrancy suites and lets main()
 * own the witnesses and the libother.so handle shared between the two callbacks.
 */

/* Defined in the main executable, exported via --export-dynamic; resolved from
 * the loader's global symbol table at load time. */
extern void hook_open_other(void);
extern void hook_close_other(void);

/* Runs from `.init_array` before dlopen("lib/libhook.so") returns. */
static void __attribute__((constructor)) hook_ctor(void)
{
    hook_open_other();
}

/* Runs from `.fini_array` while dlclose(hook_handle) tears libhook.so down. */
static void __attribute__((destructor)) hook_dtor(void)
{
    hook_close_other();
}
