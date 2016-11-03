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

#include <string.h>

/**
 * @brief String comparison using collating information.
 *
 * @details Compares the string pointed to by @p a to the string 
 * pointed to by @p b, both interpreted as appropriate to the 
 * LC_COLLATE category of the current locale.
 *
 * @return Returns an integer greater than, equal to, or less 
 * than 0, according to whether the string pointed to by s1 is
 * greater than, equal to, or less than the string pointed to 
 * by s2 when both are interpreted as appropriate to the current
 * locale.
 */
int strcoll(const char *a, const char *b)
{
  return strcmp (a, b);
}
