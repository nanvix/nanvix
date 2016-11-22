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
#include <wchar.h>
#include "../stdio/local.h"

int _fwide_r(struct _reent *ptr, FILE *fp, int mode)
{
  int ret;

  CHECK_INIT(ptr, fp);

  _newlib_flockfile_start (fp);
  if (mode != 0) {
    ORIENT (fp, mode);
  }
  if (!(fp->_flags & __SORD))
    ret = 0;
  else
    ret = (fp->_flags2 & __SWID) ? 1 : -1;
  _newlib_flockfile_end (fp);
  return ret;
}

/**
 * @brief Sets stream orientation.
 *
 * @details Determines the orientation of the stream pointed to by @p fp.
 * If @p mode is greater than zero, the function first attempts to make
 * the stream wide-oriented. If @p mode is less than zero, the function
 * first attempts to make the stream byte-oriented. Otherwise, @p mode is
 * zero and the function does not alter the orientation of the stream.
 *
 * @return Returns a value greater than zero if, after the call, the stream 
 * has wide-orientation, a value less than zero if the stream has byte-orientation,
 * or zero if the stream has no orientation.
 */
int fwide(FILE *fp, int mode)
{
  return _fwide_r (_REENT, fp, mode);
}
