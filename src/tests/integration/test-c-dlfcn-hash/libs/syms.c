/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libsyms.so - Self-contained shared library exporting a broad set of symbols.
 *
 * Every function is implemented inline with no external dependencies, so this
 * library has ZERO undefined symbols and always loads successfully. It is built
 * twice from this single source, once with `--hash-style=sysv` (yielding only a
 * `.hash` / DT_HASH table) and once with `--hash-style=gnu` (yielding only a
 * `.gnu.hash` / DT_GNU_HASH table), so the dynamic loader's accelerated
 * symbol-lookup paths are each exercised by dlfcn-hash-c.
 *
 * The set is deliberately wide (sixteen functions plus a data object) so the
 * hash buckets carry real chains and a lookup of an absent name walks a chain to
 * its end (SysV) or misses in the Bloom filter / chain (GNU).
 */

int hs_add(int a, int b)
{
    return a + b;
}

int hs_sub(int a, int b)
{
    return a - b;
}

int hs_mul(int a, int b)
{
    return a * b;
}

int hs_neg(int a)
{
    return -a;
}

int hs_square(int a)
{
    return a * a;
}

int hs_cube(int a)
{
    return a * a * a;
}

int hs_double(int a)
{
    return a + a;
}

int hs_triple(int a)
{
    return a * 3;
}

int hs_max(int a, int b)
{
    return a > b ? a : b;
}

int hs_min(int a, int b)
{
    return a < b ? a : b;
}

int hs_abs(int a)
{
    return a < 0 ? -a : a;
}

int hs_sign(int a)
{
    return (a > 0) - (a < 0);
}

/* Integer square root via Newton's method. */
int hs_isqrt(int x)
{
    if (x <= 0)
        return 0;
    int r = x;
    while (r > x / r)
        r = (r + x / r) / 2;
    return r;
}

/* Greatest common divisor via the Euclidean algorithm. */
int hs_gcd(int a, int b)
{
    a = a < 0 ? -a : a;
    b = b < 0 ? -b : b;
    while (b != 0) {
        int t = a % b;
        a = b;
        b = t;
    }
    return a;
}

/* n-th Fibonacci number (hs_fib(10) == 55). */
int hs_fib(int n)
{
    int prev = 0;
    int cur = 1;
    if (n <= 0)
        return 0;
    for (int i = 1; i < n; i++) {
        int next = prev + cur;
        prev = cur;
        cur = next;
    }
    return cur;
}

/* Triangular number sum 0..n (hs_sum_to(10) == 55). */
int hs_sum_to(int n)
{
    if (n < 0)
        return 0;
    return n * (n + 1) / 2;
}

/* Exported data object, resolved through the same hash path as the functions. */
int hs_magic = 42;
