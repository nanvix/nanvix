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

/**
 * @file
 *
 * @brief Wide-character handling.
 */

#ifndef _WCHAR_H_
#define _WCHAR_H_

	#define _NEED_WCHAR_T
	#include <decl.h>

	 /**
	  * @brief Stores any valid value of wchar_t or WEOF.
	  */
	 #ifndef _WIN_T
	 #define _WIN_T
		typedef unsigned wint_t;
	 #endif

#endif /* _WCHAR_H_ */
