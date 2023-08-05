/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#ifndef STDARG_H_
#define STDARG_H_

    typedef char* va_list;

    #define __va_rounded_size(TYPE)  \
      (((sizeof (TYPE) + sizeof (int) - 1) / sizeof (int)) * sizeof (int))

    #define va_start(AP, LASTARG) \
        (AP = ((char *) &(LASTARG) + __va_rounded_size (LASTARG)))

    void va_end (va_list);
    #define va_end(AP)

    #define va_arg(AP, TYPE)                        \
      (AP += __va_rounded_size (TYPE),              \
      *((TYPE *) (AP - __va_rounded_size (TYPE))))  \

#endif /* STDARG_H_ */
