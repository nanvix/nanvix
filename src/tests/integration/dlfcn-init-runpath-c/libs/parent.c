/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libparent.so - shared library whose DT_NEEDED entry points at
 * libchild.so, but whose runtime search path (DT_RUNPATH) is
 * `lib/subdir/`. The Nanvix loader must consult DT_RUNPATH before
 * falling back to the default `lib/` directory, otherwise libchild.so
 * will not be located and dlopen will fail.
 */

extern int child_value(int x);

int parent_value(int x)
{
    return child_value(x) * 2;
}
