/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
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

#include <_ansi.h>
#include <reent.h>
#include <stdlib.h>
#include <malloc.h>

#ifndef _REENT_ONLY

/**
 * @brief A memory allocator.
 *
 * @details Allocates unused space for an object whose
 * size in bytes is specified by size and whose value is
 * unspecified.
 * 
 * @return Returns a pointer to the allocated space,
 * otherwise, a null pointer.
 */
void *malloc(size_t nbytes)
{
  return _malloc_r (_REENT, nbytes);
}

/**
 * @brief Frees allocated memory.
 *
 * @details Causes the space pointed to by @p aptr to
 * be deallocated; that is, made available for further
 * allocation. If @p aptr is a null pointer, no action 
 * occurs.
 */
void free(void *aptr)
{
  _free_r (_REENT, aptr);
}

#endif
