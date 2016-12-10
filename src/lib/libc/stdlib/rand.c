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

#ifndef _REENT_ONLY

#include <stdlib.h>
#include <reent.h>

/**
 * @brief Pseudo-random number generator.
 *
 * @details Uses the argument as a seed for a new sequence
 * of pseudo-random numbers to be returned by subsequent calls
 * to rand().
 */
void srand(unsigned int seed)
{
  struct _reent *reent = _REENT;

  _REENT_CHECK_RAND48(reent);
  _REENT_RAND_NEXT(reent) = seed;
}

/**
 * @brief Pseudo-random number generator.
 *
 * @details Computes a sequence of pseudo-random integers
 * in the range [0, {RAND_MAX}].
 *
 * @return Returns the next pseudo-random number in the
 * sequence.
 */
int rand(void)
{
  struct _reent *reent = _REENT;

  /* This multiplier was obtained from Knuth, D.E., "The Art of
     Computer Programming," Vol 2, Seminumerical Algorithms, Third
     Edition, Addison-Wesley, 1998, p. 106 (line 26) & p. 108 */
  _REENT_CHECK_RAND48(reent);
  _REENT_RAND_NEXT(reent) =
     _REENT_RAND_NEXT(reent) * __extension__ 6364136223846793005LL + 1;
  return (int)((_REENT_RAND_NEXT(reent) >> 32) & RAND_MAX);
}

#endif /* _REENT_ONLY */
