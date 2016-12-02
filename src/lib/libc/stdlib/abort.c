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
#include <unistd.h>
#include <signal.h>

/**
 * @brief Generates an abnormal process abort.
 *
 * @details Causes abnormal process termination to occur, unless
 * the signal SIGABRT is being caught and the signal handler does
 * not return.
 */
void abort(void)
{
#ifdef ABORT_MESSAGE
  write (2, "Abort called\n", sizeof ("Abort called\n")-1);
#endif

  while (1)
    {
      raise (SIGABRT);
      _exit (1);
    }
}
