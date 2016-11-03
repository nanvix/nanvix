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

#if !defined(PREFER_SIZE_OVER_SPEED) && !defined(__OPTIMIZE_SIZE__)
# define RETURN_TYPE char *
# define AVAILABLE(h, h_l, j, n_l)			\
  (!memchr ((h) + (h_l), '\0', (j) + (n_l) - (h_l))	\
   && ((h_l) = (j) + (n_l)))
# include "str-two-way.h"
#endif

/**
 * @brief Finds a substring.
 *
 * @details Locates the first occurrence in the string 
 * pointed to by @p searchee of the sequence of bytes 
 * (excluding the terminating NUL character) in the 
 * string pointed to by @p lookfor.
 *
 * @return Returns a pointer to the located string or 
 * a null pointer if the string is not found. If @p lookfor 
 * points to a string with zero length, the function returns 
 * @p searchee.
 */
char *strstr(const char *searchee, const char *lookfor)
{
#if defined(PREFER_SIZE_OVER_SPEED) || defined(__OPTIMIZE_SIZE__)

  /* Less code size, but quadratic performance in the worst case.  */
  if (*searchee == 0)
    {
      if (*lookfor)
	return (char *) NULL;
      return (char *) searchee;
    }

  while (*searchee)
    {
      size_t i;
      i = 0;

      while (1)
	{
	  if (lookfor[i] == 0)
	    {
	      return (char *) searchee;
	    }

	  if (lookfor[i] != searchee[i])
	    {
	      break;
	    }
	  i++;
	}
      searchee++;
    }

  return (char *) NULL;

#else /* compilation for speed */

  /* Larger code size, but guaranteed linear performance.  */
  const char *haystack = searchee;
  const char *needle = lookfor;
  size_t needle_len; /* Length of NEEDLE.  */
  size_t haystack_len; /* Known minimum length of HAYSTACK.  */
  int ok = 1; /* True if NEEDLE is prefix of HAYSTACK.  */

  /* Determine length of NEEDLE, and in the process, make sure
     HAYSTACK is at least as long (no point processing all of a long
     NEEDLE if HAYSTACK is too short).  */
  while (*haystack && *needle)
    ok &= *haystack++ == *needle++;
  if (*needle)
    return NULL;
  if (ok)
    return (char *) searchee;

  /* Reduce the size of haystack using strchr, since it has a smaller
     linear coefficient than the Two-Way algorithm.  */
  needle_len = needle - lookfor;
  haystack = strchr (searchee + 1, *lookfor);
  if (!haystack || needle_len == 1)
    return (char *) haystack;
  haystack_len = (haystack > searchee + needle_len ? 1
		  : needle_len + searchee - haystack);

  /* Perform the search.  */
  if (needle_len < LONG_NEEDLE_THRESHOLD)
    return two_way_short_needle ((const unsigned char *) haystack,
				 haystack_len,
				 (const unsigned char *) lookfor, needle_len);
  return two_way_long_needle ((const unsigned char *) haystack, haystack_len,
			      (const unsigned char *) lookfor, needle_len);
#endif /* compilation for speed */
}
