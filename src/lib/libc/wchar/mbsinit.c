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

#include <reent.h>
#include <wchar.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>

/**
 * @brief Determines conversion object status.
 *
 * @details If @p ps is not a null pointer, the function determines
 * whether the object pointed to by @p ps describes an initial conversion
 * state.
 *
 * @return Returns non-zero if @p ps is a null pointer, or if the pointed-to
 * object describes an initial conversion state; otherwise, it returns zero.
 */
int mbsinit(const mbstate_t *ps)
{
  if (ps == NULL || ps->__count == 0)
    return 1;
  else
    return 0;
}
