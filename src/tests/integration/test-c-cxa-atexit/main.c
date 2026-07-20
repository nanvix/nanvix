/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <stdlib.h>
#include <unistd.h>

typedef void (*cxa_destructor_t)(void *);

extern int __cxa_atexit(cxa_destructor_t func, void *arg, void *dso_handle);
extern void __cxa_finalize(void *dso_handle);

static char dso_a;
static char dso_b;
static char dso_c;
static int first = 1;
static int second = 2;
static int third = 3;
static int fifth = 5;
static int call_order;

static void record_cxa(void *arg)
{
    call_order = call_order * 10 + *(int *)arg;
}

static void record_plain(void)
{
    call_order = call_order * 10 + 4;
}

static void verify_exit(void)
{
    _exit(call_order == 31542 ? 0 : 1);
}

int main(void)
{
    if (atexit(verify_exit) != 0)
        return 2;
    if (__cxa_atexit(record_cxa, &first, &dso_a) != 0)
        return 3;
    if (__cxa_atexit(record_cxa, &second, &dso_b) != 0)
        return 4;
    if (__cxa_atexit(record_cxa, &third, &dso_a) != 0)
        return 5;

    __cxa_finalize(&dso_a);
    if (call_order != 31)
        return 6;

    /* Finalizing the same DSO twice must not invoke its handlers again. */
    __cxa_finalize(&dso_a);
    if (call_order != 31)
        return 7;

    if (atexit(record_plain) != 0)
        return 8;
    if (__cxa_atexit(record_cxa, &fifth, &dso_c) != 0)
        return 9;

    /*
     * Returning exercises normal process-exit dispatch. The remaining handlers
     * must run in reverse registration order: cxa(5), plain(4), cxa(2), then
     * verify_exit(), which turns this sentinel into a successful exit status.
     */
    return 42;
}
