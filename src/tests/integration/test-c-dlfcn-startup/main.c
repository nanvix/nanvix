/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-startup-c: Validate that crt0's startup DT_NEEDED loader auto-loads the
 * REAL Nanvix shared libraries (libc.so and libm.so) before main() runs, with
 * NO dlopen() call.
 *
 * Unlike dlfcn-selflink-c (which links the executable against a self-contained
 * libprovider.so fixture), this suite links against the actual shared libraries
 * shipped by the toolchain. The executable is linked PIE and against libm.so
 * (-l:libm.so) instead of the static libm.a, so the math symbols below are left
 * UNDEFINED in the executable and recorded as a DT_NEEDED entry plus
 * R_386_JMP_SLOT relocations. libm.so in turn imports libc.so's allocator and
 * mem* surface, so the executable also carries a DT_NEEDED on libc.so.
 *
 * At process startup, syscall::dlfcn::dllink_executable() walks the executable's
 * DT_NEEDED list and loads libc.so and then libm.so into the global scope (the
 * equivalent of dlopen(RTLD_GLOBAL | RTLD_NOW)), binding the executable's GOT/PLT
 * slots against them before .init_array / main(). libm.so's own imports resolve
 * from libc.so (loaded first); the executable's statically linked libc.a remains
 * the single heap owner, because the loader's global scope is first-wins. No
 * dlopen() is called by the test.
 *
 * Pass/fail is signalled by the exit code (0 = pass), which nanvixd propagates;
 * the "ok" write mirrors the other suites' magic marker.
 */

#include <math.h>
#include <unistd.h>

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /*
     * `volatile` inputs stop the optimizer (release builds compile at -O3) from
     * constant-folding these math calls away: the calls must survive as real
     * PLT references into libm.so so the startup loader has GOT/PLT slots to
     * bind. The chosen arguments yield exact results, so the equality checks
     * below need no floating-point tolerance.
     */
    volatile double zero = 0.0;
    volatile double base = 2.0;
    volatile double exponent = 10.0;

    /* R_386_JMP_SLOT relocations against libm.so, auto-bound before main(). */
    double c = cos(zero);           /* cos(0)    == 1    */
    double p = pow(base, exponent); /* pow(2, 10) == 1024 */
    double e = exp(zero);           /* exp(0)    == 1    */

    if (c == 1.0 && p == 1024.0 && e == 1.0) {
        write(STDOUT_FILENO, "ok", 2);
        return 0;
    }

    return 1;
}
