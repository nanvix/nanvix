/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libdep.so - dependency fixture for dlfcn-dtor-reentry-c.
 *
 * A self-contained shared library (zero undefined symbols) that exports a
 * single value function. libreentry.so DT_NEEDEDs it, and the suite resolves
 * dep_value() through libreentry.so's handle WHILE libreentry.so is being torn
 * down, proving the dependency edge stays intact until destructors finish.
 */

int dep_value(void)
{
    return 4321;
}
