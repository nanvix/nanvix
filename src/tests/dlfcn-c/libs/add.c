/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

extern int zero;

extern int sum(int a, int b);

int add(int a, int b)
{
    return zero + sum(a, b);
}
