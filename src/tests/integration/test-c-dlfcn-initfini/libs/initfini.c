/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libinitfini.so - constructor/destructor witness for the startup DT_NEEDED
 * init/fini ordering test.
 *
 * The executable carries a DT_NEEDED entry on this library but references none
 * of its symbols, so the startup loader (syscall::dlfcn::dllink_executable)
 * auto-loads it purely for its `.init_array` / `.fini_array` side effects, with
 * NO dlopen() call. All three symbols referenced below are UNDEFINED here and
 * resolved from the executable's exported global scope at load time (the
 * executable is linked --export-dynamic), exactly like
 * test-c-dlfcn-init-runpath's libctor.so resolves `g_dtor_ran`:
 *
 *   - `g_ctor_ran`     : data witness the constructor sets before main().
 *   - `g_main_ran`     : data witness main() sets, read by the destructor to
 *                        confirm it runs AFTER main().
 *   - `test_dtor_finish`: executable-side helper the destructor calls to make
 *                        destructor-at-exit observable through the exit code.
 */

/* Witness sentinels. Must match the executable's main.c. */
#define CTOR_SENTINEL 0xC70A
#define MAIN_SENTINEL 0x3A14

/* Witnesses owned by the executable, resolved from the loader's global scope. */
extern volatile int g_ctor_ran;
extern volatile int g_main_ran;

/* Executable-side exit helper; `ok` selects the process exit code. */
extern void test_dtor_finish(int ok);

/*
 * Constructor: must run before main(). Records the sentinel so main() can
 * confirm the dependency's `.init_array` ran first.
 */
static void __attribute__((constructor)) initfini_ctor(void)
{
    g_ctor_ran = CTOR_SENTINEL;
}

/*
 * Destructor: must run at process exit, after main() returned. Validates the
 * full ctor -> main -> dtor ordering and hands the verdict to the executable's
 * exit helper. Reaching here at all proves the dependency's `.fini_array` ran
 * on exit; the witness checks additionally pin down the ordering.
 */
static void __attribute__((destructor)) initfini_dtor(void)
{
    int ordered = (g_ctor_ran == CTOR_SENTINEL) && (g_main_ran == MAIN_SENTINEL);
    test_dtor_finish(ordered);
}
