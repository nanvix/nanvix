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

#include <_ansi.h>
#include <stdlib.h>
#include <limits.h>

#ifdef _XOPEN_SOURCE

/**
 * @brief Converts between a 32-bit integer and a radix-64 ASCII string.
 *
 * @details Takes a pointer to a radix-64 representation, in which the 
 * first digit is the least significant, and return the corresponding long
 * value.
 *
 * @return Returns the long value resulting from conversion of the input 
 * string.
 */
long a64l(const char *input)
{
  const char *ptr;
  char ch;
  int i, digit;
  unsigned long result = 0;

  if (input == NULL)
    return 0;

  ptr = input;

  /* it easiest to go from most significant digit to least so find end of input or up
     to 6 characters worth */
  for (i = 0; i < 6; ++i)
    {
      if (*ptr)
	++ptr;
    }

  while (ptr > input)
    {
      ch = *(--ptr);

#if defined(PREFER_SIZE_OVER_SPEED) || defined(__OPTIMIZE_SIZE__)
      if (ch >= 'a')
	digit = (ch - 'a') + 38;
      else if (ch >= 'A')
	digit = (ch - 'A') + 12;
      else if (ch >= '0')
	digit = (ch - '0') + 2;
      else if (ch == '/')
	digit = 1;
      else
	digit = 0;
#else /* !defined(PREFER_SIZE_OVER_SPEED) && !defined(__OPTIMIZE_SIZE__) */
      switch (ch)
	{
	case '/':
	  digit = 1;
	  break;
	case '0':
	case '1':
	case '2':
	case '3':
	case '4':
	case '5':
	case '6':
	case '7':
	case '8':
	case '9':
	  digit = (ch - '0') + 2;
	  break;
	case 'A':
	case 'B':
	case 'C':
	case 'D':
	case 'E':
	case 'F':
	case 'G':
	case 'H':
	case 'I':
	case 'J':
	case 'K':
	case 'L':
	case 'M':
	case 'N':
	case 'O':
	case 'P':
	case 'Q':
	case 'R':
	case 'S':
	case 'T':
	case 'U':
	case 'V':
	case 'W':
	case 'X':
	case 'Y':
	case 'Z':
	  digit = (ch - 'A') + 12;
	  break;
	case 'a':
	case 'b':
	case 'c':
	case 'd':
	case 'e':
	case 'f':
	case 'g':
	case 'h':
	case 'i':
	case 'j':
	case 'k':
	case 'l':
	case 'm':
	case 'n':
	case 'o':
	case 'p':
	case 'q':
	case 'r':
	case 's':
	case 't':
	case 'u':
	case 'v':
	case 'w':
	case 'x':
	case 'y':
	case 'z':
	  digit = (ch - 'a') + 38;
	  break;
	default:
	  digit = 0;
	  break;
	}
#endif /* !defined(PREFER_SIZE_OVER_SPEED) && !defined(__OPTIMIZE_SIZE__) */ 
      
      result = (result << 6) + digit;
    }

#if LONG_MAX > 2147483647
  /* for implementations where long is > 32 bits, the result must be sign-extended */
  if (result & 0x80000000)
      return (((long)-1 >> 32) << 32) + result;
#endif

  return result;
}

#endif /* _XOPEN_SOURCE */
