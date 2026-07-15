/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

enum {
    STAGED_CONSTRUCTOR_VALUE = 0x5a61,
};

static int constructor_value = 0;

static void __attribute__((constructor)) staged_constructor(void)
{
    constructor_value = STAGED_CONSTRUCTOR_VALUE;
}

int staged_value(void)
{
    return (constructor_value);
}
