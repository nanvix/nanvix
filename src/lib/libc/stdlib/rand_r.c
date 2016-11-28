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

#include <stdlib.h>

/* Pseudo-random generator based on Minimal Standard by
   Lewis, Goodman, and Miller in 1969.
 
   I[j+1] = a*I[j] (mod m)

   where a = 16807
         m = 2147483647

   Using Schrage's algorithm, a*I[j] (mod m) can be rewritten as:
  
     a*(I[j] mod q) - r*{I[j]/q}      if >= 0
     a*(I[j] mod q) - r*{I[j]/q} + m  otherwise

   where: {} denotes integer division 
          q = {m/a} = 127773 
          r = m (mod a) = 2836

   note that the seed value of 0 cannot be used in the calculation as
   it results in 0 itself
*/

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)
   
/**
 * @brief Pseudo-random number generator.
 *
 * @details Computes a sequence of pseudo-random integers in the range
 * [0, {RAND_MAX}].
 *
 * @return Returns a pseudo-random integer.
 */
int rand_r(unsigned int *seed)
{
        long k;
        long s = (long)(*seed);
        if (s == 0)
          s = 0x12345987;
        k = s / 127773;
        s = 16807 * (s - k * 127773) - 2836 * k;
        if (s < 0)
          s += 2147483647;
        (*seed) = (unsigned int)s;
        return (int)(s & RAND_MAX);
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
