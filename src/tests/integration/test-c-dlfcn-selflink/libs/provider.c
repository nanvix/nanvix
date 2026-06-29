/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libprovider.so - Self-contained shared library that plays the role of
 * "libc.so" for the crt0 self-linking test.
 *
 * It exports one data symbol and one function symbol with ZERO undefined
 * symbols of its own, so it always loads successfully. The main executable
 * references both as undefined externs, which makes the static linker emit:
 *   - R_386_GLOB_DAT for selflink_provider_value (a GOT data slot), and
 *   - R_386_JMP_SLOT for selflink_provider_func  (a PLT entry),
 * plus a DT_NEEDED entry on libprovider.so. nvx-crt0's self-linker
 * (syscall::dlfcn::dllink_executable) must bind both before main() runs.
 */

/* Distinctive sentinel so a failure to bind the GOT slot is obvious. */
int selflink_provider_value = 0xC0DE;

int selflink_provider_func(int x)
{
    return x + 1;
}
