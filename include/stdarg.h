/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * stdarg.h - Variable argument list handling
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
