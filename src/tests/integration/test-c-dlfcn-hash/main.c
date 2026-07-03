/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-hash-c: validate DT_HASH / DT_GNU_HASH accelerated symbol lookup.
 *
 * The dynamic loader resolves symbol relocations and dlsym() queries through
 * the ELF symbol hash tables (`.hash` / `.gnu.hash`) instead of a linear
 * `.dynsym` scan. This is a performance optimization with no change in
 * observable behavior, so the acceptance criterion is that dlopen()/dlsym()
 * still resolve every symbol correctly when the loader walks a hash table.
 *
 * The suite dlopen()s two self-contained fixtures built from the same source but
 * with different hash tables:
 *   * libsyms-sysv.so - linked --hash-style=sysv, carrying only a SysV
 *     (`.hash` / DT_HASH) table, so the loader takes its SysV lookup path.
 *   * libsyms-gnu.so  - linked --hash-style=gnu, carrying only a GNU
 *     (`.gnu.hash` / DT_GNU_HASH) table, so the loader takes its GNU lookup path.
 *
 * For each fixture the test resolves every exported function plus an exported
 * data object through dlsym() and checks each returns the expected result (the
 * hash walk found the correct symbol), then confirms that a name the object does
 * not define resolves to NULL (the not-found path terminates correctly).
 */

#include <assert.h>
#include <dlfcn.h>
#include <stddef.h>
#include <stdio.h>
#include <unistd.h>

typedef int (*fn1_t)(int);
typedef int (*fn2_t)(int, int);

static void check_lib(const char *path)
{
    printf("=== %s ===\n", path);
    fflush(stdout);

    void *handle = dlopen(path, RTLD_NOW);
    assert(handle != NULL);

    /* Resolve every exported symbol through the loader's hash lookup path. */
    fn2_t add = (fn2_t)dlsym(handle, "hs_add");
    fn2_t sub = (fn2_t)dlsym(handle, "hs_sub");
    fn2_t mul = (fn2_t)dlsym(handle, "hs_mul");
    fn1_t neg = (fn1_t)dlsym(handle, "hs_neg");
    fn1_t square = (fn1_t)dlsym(handle, "hs_square");
    fn1_t cube = (fn1_t)dlsym(handle, "hs_cube");
    fn1_t twice = (fn1_t)dlsym(handle, "hs_double");
    fn1_t triple = (fn1_t)dlsym(handle, "hs_triple");
    fn2_t maximum = (fn2_t)dlsym(handle, "hs_max");
    fn2_t minimum = (fn2_t)dlsym(handle, "hs_min");
    fn1_t absval = (fn1_t)dlsym(handle, "hs_abs");
    fn1_t sign = (fn1_t)dlsym(handle, "hs_sign");
    fn1_t isqrt = (fn1_t)dlsym(handle, "hs_isqrt");
    fn2_t gcd = (fn2_t)dlsym(handle, "hs_gcd");
    fn1_t fib = (fn1_t)dlsym(handle, "hs_fib");
    fn1_t sum_to = (fn1_t)dlsym(handle, "hs_sum_to");
    int *magic = (int *)dlsym(handle, "hs_magic");

    assert(add && sub && mul && neg && square && cube && twice && triple);
    assert(maximum && minimum && absval && sign && isqrt && gcd && fib && sum_to);
    assert(magic != NULL);

    /* Each result confirms the hash walk resolved the correct symbol. */
    assert(add(3, 4) == 7);
    assert(sub(10, 3) == 7);
    assert(mul(6, 7) == 42);
    assert(neg(5) == -5);
    assert(square(9) == 81);
    assert(cube(4) == 64);
    assert(twice(21) == 42);
    assert(triple(14) == 42);
    assert(maximum(3, 9) == 9);
    assert(minimum(3, 9) == 3);
    assert(absval(-7) == 7);
    assert(sign(-3) == -1 && sign(0) == 0 && sign(5) == 1);
    assert(isqrt(144) == 12);
    assert(gcd(54, 24) == 6);
    assert(fib(10) == 55);
    assert(sum_to(10) == 55);
    assert(*magic == 42);

    /*
     * A name the object does not define must resolve to NULL. This exercises the
     * loader's not-found path: a SysV chain walked to STN_UNDEF, or a GNU
     * Bloom-filter reject / chain miss.
     */
    assert(dlsym(handle, "hs_absent_symbol") == NULL);

    dlclose(handle);

    printf("  PASS: %s\n", path);
    fflush(stdout);
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /* libsyms-sysv.so -> DT_HASH path; libsyms-gnu.so -> DT_GNU_HASH path. */
    check_lib("lib/libsyms-sysv.so");
    check_lib("lib/libsyms-gnu.so");

    /* Magic success marker (harmless; standalone runs are exit-code-only). */
    const char *magic = "ok";
    write(STDOUT_FILENO, magic, 2);

    return 0;
}
