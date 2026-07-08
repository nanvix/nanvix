/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libroot.so - root of the dlfcn-dlclose-cycle graph:
 *
 *   libroot.so DT_NEEDED libmidx.so, libmidy.so, libleaf.so
 *
 * `root_touch()` references a symbol from each of libmidx.so (`midx_touch`),
 * libmidy.so (`midy_touch`) and libleaf.so (`leaf_bump`/`leaf_get`), which
 * forces all three DT_NEEDED edges at link time. The direct libleaf.so edge --
 * alongside the two that arrive transitively through the intermediates -- is
 * what makes libleaf.so reachable more than once during a single
 * dlclose(libroot.so) traversal, reproducing the repeated-visit condition.
 */

extern int leaf_bump(void);
extern int leaf_get(void);
extern int midx_touch(void);
extern int midy_touch(void);

int root_touch(void)
{
    (void)midx_touch();
    (void)midy_touch();
    (void)leaf_bump();
    return leaf_get();
}
