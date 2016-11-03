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
#include <stdio.h>
#include <string.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Writes signal information to standard error.
 *
 * @details Writes a language-dependent message associated
 * with a signal number to the standard error stream.
 */
void psignal(int sig, const char *s)
{
  if (s != NULL && *s != '\0')
    fprintf (stderr, "%s: %s\n", s, strsignal (sig));
  else
    fprintf (stderr, "%s\n", strsignal (sig));
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
