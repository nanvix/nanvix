/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libbase.so - shared leaf of the diamond DT_NEEDED graph.
 *
 *   libdiamond.so
 *     +-- DT_NEEDED libleft.so   --+
 *     +-- DT_NEEDED libright.so  --+-- DT_NEEDED libbase.so
 *
 * Both libleft.so and libright.so declare DT_NEEDED libbase.so. When the
 * loader walks libdiamond.so's dependencies, it must NOT load libbase.so
 * twice -- exactly one copy should appear in the registry. Otherwise the
 * `unique_counter` global below would be visible at two distinct
 * addresses to libleft.so and libright.so, and the assertion in
 * dlfcn-diamond-c/main.c would fire.
 */

/* Process-lifetime counter. Bumped by every call to `base_bump()`. */
static int unique_counter = 0;

int base_bump(void)
{
    unique_counter += 1;
    return unique_counter;
}

int base_get(void)
{
    return unique_counter;
}
