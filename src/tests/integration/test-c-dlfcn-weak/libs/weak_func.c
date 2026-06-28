/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <stddef.h>

#ifndef CALLBACK_NAME
#define CALLBACK_NAME main_callback
#endif

extern int CALLBACK_NAME(int) __attribute__((weak));

int try_callback(void)
{
    if (&CALLBACK_NAME != NULL) {
        return (CALLBACK_NAME(7));
    }

    return (-1);
}
