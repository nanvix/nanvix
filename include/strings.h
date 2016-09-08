/*
 * strings.h
 *
 * Definitions for string operations.
 */

#ifndef _STRINGS_H_
#define _STRINGS_H_

#include "_ansi.h"
#include <sys/reent.h>

#include <sys/types.h> /* for size_t */

_BEGIN_STD_C

int	 _EXFUN(ffs,(int));
int	 _EXFUN(strcasecmp,(const char *, const char *));
int	 _EXFUN(strncasecmp,(const char *, const char *, size_t));

_END_STD_C

#endif /* _STRINGS_H_ */
