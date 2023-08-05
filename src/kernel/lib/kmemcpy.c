/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/const.h>
#include <sys/types.h>

/**
 * @brief Copy bytes in memory.
 *
 * @param dest Target memory area.
 * @param src  Source memory area.
 * @param n    Number of bytes to be copied.
 *
 * @returns A pointer to the target memory area.
 */
PUBLIC void *kmemcpy (void* dest, const void *src, size_t n)
{
    char *d;       /* Write pointer. */
    const char* s; /* Read pointer.  */

    s = src;
    d = dest;

    while (n-- > 0)
    	*d++ = *s++;

    return (d);
}
