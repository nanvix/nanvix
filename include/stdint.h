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

#ifndef STDINT_H_
#define STDINT_H_

#ifndef _ASM_FILE_

	/* Signed integers. */
	typedef __INT8_TYPE__ int8_t;   /* 8-bit integer.  */
	typedef __INT16_TYPE__ int16_t; /* 16-bit integer. */
	typedef __INT32_TYPE__ int32_t; /* 32-bit integer. */
	typedef __INT64_TYPE__ int64_t; /* 64-bit integer. */

	/* Unsigned integers. */
	typedef __UINT8_TYPE__ uint8_t;   /* 8-bit integer.  */
	typedef __UINT16_TYPE__ uint16_t; /* 16-bit integer. */
	typedef __UINT32_TYPE__ uint32_t; /* 32-bit integer. */
	typedef __UINT64_TYPE__ uint64_t; /* 64-bit integer. */

#endif /* _ASM_FILE_*/

#endif /* STDINT_H_ */
