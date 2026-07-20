/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

extern int staged_value(void);

int good_root_value(void)
{
    return (staged_value() + 1);
}
