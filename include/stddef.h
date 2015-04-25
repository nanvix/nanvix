/*
 * Copyright(C) 2011-2015 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * @brief Standard type definitions.
 */

#ifndef _STDDEF_H_
#define _STDDEF_H_

	/**
	 * @brief Null pointer.
	 */
	#ifndef _NULL
	#define _NULL
		#define NULL ((void *) 0)
	#endif

	/**
	 * @brief Codes for all members of the largest extended character set.
	 */
	#ifndef _WCHAR_T	
	#define _WCHAR_T
		typedef unsigned wchar_t;
	#endif

	/**
	 * @brief Used for sizes of objects.
	 */
	#ifndef SIZE_T
	#define SIZE_T
		typedef unsigned size_t;
	#endif

#endif /* _STDDEF_H_ */
