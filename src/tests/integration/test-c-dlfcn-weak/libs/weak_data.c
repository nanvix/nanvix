/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <stddef.h>

#ifndef WEAK_DATA_NAME
#define WEAK_DATA_NAME weak_data
#endif

extern int WEAK_DATA_NAME __attribute__((weak));

int read_weak_data(void)
{
    if (&WEAK_DATA_NAME != NULL) {
        return (WEAK_DATA_NAME);
    }

    return (-1);
}
