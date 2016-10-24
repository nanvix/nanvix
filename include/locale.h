/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
 *                   Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

/**
 * @file
 * 
 * @brief Category macros library.
 */

#ifndef LOCALE_H_
#define LOCALE_H_

	#include "_ansi.h"

	#define __need_NULL
	#include <stddef.h>

 	/**
 	 * @brief Locale selection.
 	 */
	/**@{*/
	#define LC_ALL	    0 /**< All categories.                                                                           */
	#define LC_COLLATE  1 /**< Regular expressions and collation functions.                                              */
	#define LC_CTYPE    2 /**< Regular expressions, character classification, character conversion, and wide-characters. */
	#define LC_MONETARY 3 /**< Monetary values.                                                                          */
	#define LC_NUMERIC  4 /**< Numeric values.                                                                           */
	#define LC_TIME     5 /**< Time conversion.                                                                          */
#ifdef _POSIX_C_SOURCE
	#define LC_MESSAGES 6 /**< Response expressions, message catalogs and read/writes.                                   */
#endif
	/**@}*/


 	/**
 	 * @brief Locale structure.
 	 */
	struct lconv
	{
	  char *decimal_point;
	  char *thousands_sep;
	  char *grouping;
	  char *int_curr_symbol;
	  char *currency_symbol;
	  char *mon_decimal_point;
	  char *mon_thousands_sep;
	  char *mon_grouping;
	  char *positive_sign;
	  char *negative_sign;
	  char int_frac_digits;
	  char frac_digits;
	  char p_cs_precedes;
	  char p_sep_by_space;
	  char n_cs_precedes;
	  char n_sep_by_space;
	  char p_sign_posn;
	  char n_sign_posn;
	  char int_n_cs_precedes;
	  char int_n_sep_by_space;
	  char int_n_sign_posn;
	  char int_p_cs_precedes;
	  char int_p_sep_by_space;
	  char int_p_sign_posn;
	};

	/* Forward definitions. */
	struct _reent;

	/* Forward definitions. */
	#ifndef _REENT_ONLY
		extern char *setlocale(int, const char *);
		extern struct lconv *localeconv(void);
	#endif
	extern char *_setlocale_r(struct _reent *, int, const char *);
	extern struct lconv *_localeconv_r(struct _reent *);


#endif /* LOCALE_H_ */
