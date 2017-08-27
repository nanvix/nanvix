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

double _erand48_r(struct _reent *r, unsigned short xseed[3])
{
  __dorand48(r, xseed);
  return ldexp((double) xseed[0], -48) +
    ldexp((double) xseed[1], -32) +
    ldexp((double) xseed[2], -16);
}

#ifndef _REENT_ONLY

/**
 * @brief Generates uniformly distributed pseudo-random numbers.
 *
 * @details Generates pseudo-random numbers using the linear congruential
 * algorithm and 48-bit integer arithmetic. Unlike drand48(), erand48()
 * requires the calling program to provide storage for the successive Xi
 * values in the array argument @p xseed.
 *
 * @return Returns non-negative, double-precision, floating-point values,
 * uniformly distributed over the interval [0.0,1.0).
 */
double erand48(unsigned short xseed[3])
{
  return _erand48_r (_REENT, xseed);
}

#endif /* !_REENT_ONLY */
#endif /* _XOPEN_SOURCE */
