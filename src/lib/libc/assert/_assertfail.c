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

/**
 * @file
 *
 * @brief _assertfail() implementation.
 */

#include <stdio.h>
#include <stdlib.h>

/**
 * @brief Prints failed assertion message and aborts.
 *
 * @param msg  Message.
 * @param cond Condition.
 * @param file Filename.
 * @param line Line number.
 */
void _assertfail(const char *msg, const char *cond, const char *file, int line)
{
	fprintf(stderr, msg, cond, file, line);
	abort();
}
