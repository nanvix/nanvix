/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

extern int bad_value(void);
extern int staged_value(void);

int failed_root_value(void)
{
    return (staged_value() + bad_value());
}
