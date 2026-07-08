/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libmidy.so - the other intermediate node of the dlfcn-dlclose-cycle graph:
 *
 *   libmidy.so DT_NEEDED libleaf.so
 *
 * `midy_touch()` references libleaf.so's `leaf_bump()`, which forces the
 * DT_NEEDED edge on libleaf.so at link time. It shares libleaf.so with
 * libmidx.so; the staggered-dlclose test uses this shared edge to prove that
 * closing one intermediate leaves libleaf.so loaded for the other.
 */

extern int leaf_bump(void);

int midy_touch(void)
{
    return leaf_bump();
}
