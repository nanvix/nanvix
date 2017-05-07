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
 * Copyright (c) 1990 Regents of the University of California.
 * All rights reserved.
 *
 * %sccs.include.redist.c%
 */

#include <stdlib.h>
#include <unistd.h>
#include <reent.h>
#include "atexit.h"

/**
 * @brief Terminates the calling process.
 *
 * @details The function first call all functions
 * registered by atexit(), in the reverse order of
 * their registration, except that a function is
 * called after any previously registered functions
 * that had already been called at the time it was
 * registered. Each function is called as many times
 * as it was registered.
 * 
 * If a function registered by a call to atexit() fails
 * to return, the remaining registered functions is not
 * be called and the rest of the exit() processing is not
 * be completed. If exit() is called more than once, the
 * behavior is undefined.
 */
void exit(int code)
{
#ifdef _LITE_EXIT
  /* Refer to comments in __atexit.c for more details of lite exit.  */
  void __call_exitprocs _PARAMS ((int, _PTR)) __attribute__((weak));
  if (__call_exitprocs)
#endif
    __call_exitprocs (code, NULL);

  if (_GLOBAL_REENT->__cleanup)
    (*_GLOBAL_REENT->__cleanup) (_GLOBAL_REENT);
  _exit (code);
}
