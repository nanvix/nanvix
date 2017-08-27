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

double _drand48_r(struct _reent *r)
{
  _REENT_CHECK_RAND48(r);
  return _erand48_r(r, __rand48_seed);
}

#ifndef _REENT_ONLY

/**
 * @brief Generates uniformly distributed pseudo-random numbers.
 *
 * @details Generates pseudo-random numbers using the linear congruential
 * algorithm and 48-bit integer arithmetic.
 *
 * @return Returns non-negative, double-precision, floating-point values,
 * uniformly distributed over the interval [0.0,1.0).
 */
double drand48(void)
{
  return _drand48_r (_REENT);
}

#endif /* !_REENT_ONLY */
#endif /* _XOPEN_SOURCE */
