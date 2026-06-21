/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libprovider.so - Self-contained shared library that exports functions.
 *
 * All functions are implemented inline with no external dependencies,
 * so this library has ZERO undefined symbols and always loads successfully.
 */

int provider_add(int a, int b)
{
    return a + b;
}

int provider_mul(int a, int b)
{
    return a * b;
}

/* Integer square root via Newton's method. */
int provider_isqrt(int x)
{
    if (x <= 0)
        return 0;
    int r = x;
    while (r > x / r)
        r = (r + x / r) / 2;
    return r;
}
