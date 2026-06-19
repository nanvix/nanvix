/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_IEEEFP_H
#define _NANVIX_IEEEFP_H

/**
 * @file ieeefp.h
 * @brief Legacy IEEE floating-point predicates.
 *
 * Provides the historical `finite()` predicates in terms of the C99
 * `isfinite` classification macro from <math.h>.
 */

#include <math.h>

#ifndef finite
#define finite(x) isfinite(x)
#endif

#ifndef finitef
#define finitef(x) isfinite(x)
#endif

#endif /* _NANVIX_IEEEFP_H */
