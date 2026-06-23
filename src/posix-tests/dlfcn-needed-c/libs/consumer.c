/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libconsumer.so - Shared library that depends on libprovider.so.
 *
 * Calls provider_add(), provider_mul(), provider_isqrt() which are
 * defined in libprovider.so.  This library is linked against
 * libprovider.so at build time, creating a DT_NEEDED entry.
 *
 * When the dynamic loader opens this library, it should automatically
 * load libprovider.so (via DT_NEEDED) and resolve the symbols.
 * This is the standard mechanism for shared library dependencies
 * (e.g., a Python C extension depending on libm.so or libc.so).
 */

extern int provider_add(int a, int b);
extern int provider_mul(int a, int b);
extern int provider_isqrt(int x);

int consumer_add(int a, int b)
{
    return provider_add(a, b);
}

int consumer_mul(int a, int b)
{
    return provider_mul(a, b);
}

int consumer_isqrt(int x)
{
    return provider_isqrt(x);
}

/* Function with no cross-library dependency (control case). */
int consumer_noop(int x)
{
    return x + 1;
}
