/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

const char *VERSION = "0.0.1";

int add(int a, int b)
{
    return (a + b);
}

int fast_mul(int a, int b)
{
#if defined(__i386__)
    int result = 0;
    __asm__ __volatile__("movl %1, %%ecx;"
                         "movl $0, %0;"
                         "test %%ecx, %%ecx;"
                         "jz 1f;"
                         "0:;"
                         "pushl %2;"
                         "pushl %0;"
                         "call add;"
                         "addl $8, %%esp;"
                         "movl %%eax, %0;"
                         "loop 0b;"
                         "1:;"
                         : "=r"(result)
                         : "r"(b), "r"(a)
                         : "ecx", "eax", "cc");
    return (result);
#else
    int result = 0;

    for (int i = 0; i < b; i++) {
        result = add(result, a);
    }

    return (result);
#endif
}

int slow_mul(int a, int b)
{
    int result = 0;

    for (int i = 0; i < b; i++) {
        result = add(result, a);
    }

    return (result);
}

static int (*mul)(int, int) = &fast_mul;

int multiply(int a, int b)
{
    return (mul(a, b));
}

const char *get_version(void)
{
    return (VERSION);
}
