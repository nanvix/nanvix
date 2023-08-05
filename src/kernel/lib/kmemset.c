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
 * @brief Sets bytes in memory.
 *
 * @param ptr Pointer to target memory area.
 * @param c   Character to use.
 * @param n   Number of bytes to be set.
 *
 * @returns A pointer to the target memory area.
 */
PUBLIC void *kmemset(void *ptr, int c, size_t n)
{
    unsigned char *p;

    p = ptr;

    /* Set bytes. */
    while (n-- > 0)
		*p++ = (unsigned char) c;

    return (ptr);
}
