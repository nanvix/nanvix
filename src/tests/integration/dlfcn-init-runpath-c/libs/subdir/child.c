/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libchild.so - dependency of libparent.so. Installed under a
 * non-default directory (`lib/subdir/`) so the only way the loader
 * can find it via the DT_NEEDED edge in libparent.so is by honouring
 * libparent's DT_RUNPATH.
 */

int child_value(int x)
{
    return x + 7;
}
