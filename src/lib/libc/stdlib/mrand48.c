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

long _mrand48_r(struct _reent *r)
{
  _REENT_CHECK_RAND48(r);
  __dorand48(r, __rand48_seed);
  return ((long) __rand48_seed[2] << 16) + (long) __rand48_seed[1];
}

#ifndef _REENT_ONLY

/**
 * @brief Generates uniformly distributed pseudo-random numbers.
 *
 * @details Generates pseudo-random numbers using the linear congruential
 * algorithm and 48-bit integer arithmetic.
 
 * @return Returns signed long integers uniformly distributed over the
 * interval [-2^31,2^31).
 */
long mrand48(void)
{
  return _mrand48_r (_REENT);
}

#endif /* !_REENT_ONLY */
#endif /* _XOPEN_SOURCE */
