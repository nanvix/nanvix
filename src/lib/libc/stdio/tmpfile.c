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
#include <reent.h>
#include <stdio.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/stat.h>

#ifndef O_BINARY
# define O_BINARY 0
#endif

FILE *_tmpfile_r(struct _reent *ptr)
{
  FILE *fp;
  int e;
  char *f;
  char buf[L_tmpnam];
  int fd;

  do
    {
      if ((f = _tmpnam_r (ptr, buf)) == NULL)
	return NULL;
      fd = _open_r (ptr, f, O_RDWR | O_CREAT | O_EXCL | O_BINARY,
		    S_IRUSR | S_IWUSR);
    }
  while (fd < 0 && ptr->_errno == EEXIST);
  if (fd < 0)
    return NULL;
  fp = _fdopen_r (ptr, fd, "wb+");
  e = ptr->_errno;
  if (!fp)
    _close_r (ptr, fd);
  (void) _remove_r (ptr, f);
  ptr->_errno = e;
  return fp;
}

#ifndef _REENT_ONLY

/**
 * @brief Creates a temporary file.
 *
 * @details Creates a temporary file and open a corresponding
 * stream. The file is automatically deleted when all references
 * to the file are closed. The file is opened as in fopen() for
 * update (wb+).
 *
 * @return Returns a pointer to the stream of the file that is 
 * created. Otherwise, returns a null pointer and sets errno to
 * indicate the error.
 */
FILE *tmpfile(void)
{
  return _tmpfile_r (_REENT);
}

#endif
