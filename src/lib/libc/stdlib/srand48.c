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

void _srand48_r(struct _reent *r, long seed)
{
  _REENT_CHECK_RAND48(r);
  __rand48_seed[0] = _RAND48_SEED_0;
  __rand48_seed[1] = (unsigned short) seed;
  __rand48_seed[2] = (unsigned short) ((unsigned long)seed >> 16);
  __rand48_mult[0] = _RAND48_MULT_0;
  __rand48_mult[1] = _RAND48_MULT_1;
  __rand48_mult[2] = _RAND48_MULT_2;
  __rand48_add = _RAND48_ADD;
}

#ifndef _REENT_ONLY

/**
 * @brief Generates uniformly distributed pseudo-random numbers.
 *
 * @details This is a initialization function and should be called
 * before using drand48(), lrand48() or mrand48(). The function sets
 * the high order 32-bits of Xi to the argument seedval. The low order
 * 16-bits are set to the arbitrary value 0x330E.
 */
void srand48(long seed)
{
  _srand48_r (_REENT, seed);
}

#endif /* !_REENT_ONLY */
#endif /* _XOPEN_SOURCE */
