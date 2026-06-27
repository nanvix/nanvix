/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libleft.so - left arm of the diamond. Calls base_bump() so the test
 * can observe whether libleft and libright share the same libbase.so
 * instance (single counter) or each got their own (independent counters).
 */

extern int base_bump(void);

int left_bump(void)
{
    return base_bump();
}
