/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * libright.so - right arm of the diamond. Symmetric to libleft.so.
 */

extern int base_bump(void);
extern int base_get(void);

int right_bump(void)
{
    return base_bump();
}

int right_get(void)
{
    return base_get();
}
