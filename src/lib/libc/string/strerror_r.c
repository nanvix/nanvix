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

#undef __STRICT_ANSI__
#include <errno.h>
#include <string.h>

/**
 * @brief Gets error message string.
 *
 * @details Maps the error number in @p errnum to a locale-dependent
 * error message string and returns the string in the buffer
 * pointed to by @p strerrbuf, with length buflen.
 *
 * @return Returns 0 in success, otherwise, an error number.
 */
#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

int strerror_r(int errnum, char *buffer, size_t n)
{
  char *error;
  int result = 0;

  if (!n)
    return ERANGE;
  error = _strerror_r (_REENT, errnum, 1, &result);
  if (strlen (error) >= n)
    {
      memcpy (buffer, error, n - 1);
      buffer[n - 1] = '\0';
      return ERANGE;
    }
  strcpy (buffer, error);
  return (result || *error) ? result : EINVAL;
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
