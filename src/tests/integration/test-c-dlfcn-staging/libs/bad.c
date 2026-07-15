/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

extern int missing_value(void);

int bad_value(void)
{
    return (missing_value());
}
