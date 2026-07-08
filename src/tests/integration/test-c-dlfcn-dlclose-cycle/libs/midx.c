/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libmidx.so - one intermediate node of the dlfcn-dlclose-cycle graph:
 *
 *   libmidx.so DT_NEEDED libleaf.so
 *
 * `midx_touch()` references libleaf.so's `leaf_bump()`, which forces the
 * DT_NEEDED edge on libleaf.so at link time. libmidx.so and libmidy.so both
 * depend on the SAME libleaf.so, so a correct loader binds both to a single
 * shared instance; dlclose() must keep that instance loaded while EITHER
 * intermediate is still referenced.
 */

extern int leaf_bump(void);

int midx_touch(void)
{
    return leaf_bump();
}
