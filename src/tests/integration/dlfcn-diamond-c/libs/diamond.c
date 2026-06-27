/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libdiamond.so - root of the diamond. Pulls in libleft.so and
 * libright.so via DT_NEEDED, AND carries a direct DT_NEEDED on
 * libbase.so as well. Both arms transitively depend on libbase.so,
 * so a correct loader must consolidate all three "DT_NEEDED
 * libbase.so" edges (the two arms' plus libdiamond's own direct edge)
 * onto a single libbase.so instance. The direct edge is what exercises
 * the loader's load-loop re-check: an arm loads libbase.so first, then
 * libdiamond's own libbase.so edge must bind to that existing instance
 * instead of re-opening it.
 */

extern int left_bump(void);
extern int right_bump(void);
extern int right_get(void);
extern int base_get(void);

int diamond_left(void)
{
    return left_bump();
}

int diamond_right(void)
{
    return right_bump();
}

int diamond_observe(void)
{
    return right_get();
}

/*
 * Reads the shared counter through libdiamond's OWN direct DT_NEEDED
 * edge on libbase.so (not through an arm). If the loader re-opened
 * libbase.so for this direct edge instead of consolidating, this would
 * observe a separate counter than the arms bumped.
 */
int diamond_base_get(void)
{
    return base_get();
}
