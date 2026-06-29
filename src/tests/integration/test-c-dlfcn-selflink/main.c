/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-selflink-c: Validate that crt0 binds the MAIN EXECUTABLE's own
 * symbol-based GOT/PLT relocations against a DT_NEEDED shared library before
 * main() runs.
 *
 * Unlike the other dlfcn suites, this test never calls dlopen(): the binding
 * happens implicitly at process startup. The executable is linked PIE and
 * against libprovider.so (-lprovider), so the static linker records a
 * DT_NEEDED entry and emits:
 *   - R_386_GLOB_DAT for `selflink_provider_value` (read through the GOT), and
 *   - R_386_JMP_SLOT for `selflink_provider_func`  (called through the PLT).
 *
 * Historically nvx-crt0 only applied R_386_RELATIVE fixups, leaving these two
 * slots unbound; reading the value or calling the function would observe an
 * unrelocated slot (wrong value / faulting indirect jump). With the self-linker
 * (syscall::dlfcn::dllink_executable), both slots are bound before main() and
 * the references below resolve to libprovider.so.
 *
 * Pass/fail is signalled by the exit code (0 = pass), which nanvixd propagates;
 * the "ok" write mirrors the other suites' magic marker.
 */

#include <unistd.h>

/*
 * Defined in libprovider.so. The executable references these as undefined
 * externs, which drives the GOT/PLT relocations the self-linker must bind.
 */
extern int selflink_provider_value;
extern int selflink_provider_func(int x);

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /* R_386_GLOB_DAT: the GOT slot must point at libprovider.so's data. */
    int value = selflink_provider_value;

    /* R_386_JMP_SLOT: the PLT slot must point at libprovider.so's code. */
    int result = selflink_provider_func(41);

    if (value == 0xC0DE && result == 42) {
        write(STDOUT_FILENO, "ok", 2);
        return 0;
    }

    return 1;
}
