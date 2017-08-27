/*
 * Copyright(C) 2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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

/*
 * Copyright (c) 1993 Martin Birgmeier
 * All rights reserved.
 *
 * You may redistribute unmodified or modified versions of this source
 * code provided that the above copyright notice and this and the
 * following conditions are retained.
 *
 * This software is provided ``as is'', and comes with no warranties
 * of any kind. I shall in no event be liable for anything that happens
 * to anyone/anything when using this software.
 */

#include "rand48.h"

#ifdef _XOPEN_SOURCE

void _lcong48_r(struct _reent *r, unsigned short p[7])
{
  _REENT_CHECK_RAND48(r);
  __rand48_seed[0] = p[0];
  __rand48_seed[1] = p[1];
  __rand48_seed[2] = p[2];
  __rand48_mult[0] = p[3];
  __rand48_mult[1] = p[4];
  __rand48_mult[2] = p[5];
  __rand48_add = p[6];
}

#ifndef _REENT_ONLY

/**
 * @brief Generates uniformly distributed pseudo-random numbers.
 *
 * @details This is a initialization function and should be called
 * before using drand48(), lrand48() or mrand48().
 */
void lcong48(unsigned short p[7])
{
  _lcong48_r (_REENT, p);
}

#endif /* !_REENT_ONLY */
#endif /* _XOPEN_SOURCE */
