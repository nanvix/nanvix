/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

extern int strong_missing(void);

int call_strong(void)
{
    return (strong_missing());
}
