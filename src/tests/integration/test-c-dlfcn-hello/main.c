/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-hello-c: The "dynamic hello" acceptance test.
 *
 * This is the capstone of the startup dynamic-linking work: a plain, hello-world
 * style executable that is linked against the REAL Nanvix shared libraries
 * libc.so and libm.so (via `-l:libc.so -l:libm.so`, see the build wiring) and
 * relies on the crt0 startup loader to auto-resolve them before main(), with NO
 * dlopen() call. It ties together two behaviors it builds on:
 *   - init/fini ordering: libc.so's constructors run before main;
 *   - runtime layout + search path: the bare DT_NEEDED names are located
 *     through the loader's default lib/ search path.
 *
 * Because the executable is linked PIE against libm.so instead of the static
 * libm.a, the math symbols below are left UNDEFINED in the executable and
 * recorded as DT_NEEDED entries plus R_386_JMP_SLOT relocations. libm.so in turn
 * imports libc.so's allocator and mem* surface, so the executable also carries a
 * DT_NEEDED on libc.so. At process startup, syscall::dlfcn::dllink_executable()
 * walks the executable's DT_NEEDED list, loads libc.so then libm.so into the
 * global scope, and binds the executable's GOT/PLT slots before main() runs.
 * The executable's statically linked libc.a remains the single heap owner (the
 * loader's global scope is first-wins), so printf() below is served by the
 * static libc.a while cos/pow/exp are served by the auto-loaded libm.so.
 *
 * cos/pow/exp are used (rather than sqrt, which the compiler lowers to a
 * hardware instruction / a local compiler_builtins symbol and so never becomes a
 * dynamic import) precisely because they stay undefined in the executable and
 * bind to libm.so at startup. The test computes a value of 42 THROUGH that
 * dynamic linkage and prints it, the canonical "value=42 from a dynamically
 * linked exe" acceptance signal. Pass/fail is signalled by the exit code
 * (0 = pass), which nanvixd propagates; the "ok" write mirrors the other suites'
 * magic marker.
 */

#include <math.h>
#include <stdio.h>
#include <unistd.h>

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /*
     * `volatile` inputs stop the optimizer (release builds compile at -O3) from
     * constant-folding these libm calls away: each must survive as a real PLT
     * reference (R_386_JMP_SLOT) into libm.so so the startup loader has GOT/PLT
     * slots to bind before main(). The chosen arguments yield exact results, so
     * the arithmetic below needs no floating-point tolerance.
     */
    volatile double zero = 0.0;
    volatile double base = 2.0;
    volatile double exponent = 10.0;

    /* R_386_JMP_SLOT relocations against libm.so, auto-bound before main(). */
    double c = cos(zero);           /* cos(0)     == 1    */
    double p = pow(base, exponent); /* pow(2, 10) == 1024 */
    double e = exp(zero);           /* exp(0)     == 1    */

    /*
     * Derive the answer THROUGH the dynamically-resolved results so a wrong
     * binding of cos/pow/exp would change it: cos(0) == exp(0) == 1 act as unit
     * factors and pow(2, 10) == 1024 as its own normalizer, leaving the classic
     * 6 * 7 == 42 exactly.
     */
    int value = (int)(6.0 * 7.0 * c * e * (p / 1024.0));

    /* The "hello" line: prints "value=42" via the statically linked libc.a. */
    printf("value=%d\n", value);
    fflush(stdout);

    if (value == 42) {
        write(STDOUT_FILENO, "ok", 2);
        return 0;
    }

    return 1;
}
