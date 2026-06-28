/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libconsumer.so - Shared library that depends on libprovider.so
 *                  WITHOUT a DT_NEEDED entry.
 *
 * This library calls provider_add(), provider_mul(), provider_isqrt()
 * but is NOT linked against libprovider.so at build time.  The symbols
 * must be resolved at dlopen time from the global scope (RTLD_GLOBAL).
 *
 * This pattern occurs in plugin architectures where a host loads a
 * provider library with RTLD_GLOBAL before loading plugin modules.
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
