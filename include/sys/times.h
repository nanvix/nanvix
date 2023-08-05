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

#ifndef SYS_TIMES_H_
#define SYS_TIMES_H_
#ifndef _ASM_FILE_

	#include <sys/types.h>

	/**
	 * @brief Process times.
	 */
	struct tms
	{
		clock_t tms_utime;  /**< User CPU time.                          */
		clock_t tms_stime;  /**< System CPU time.                        */
		clock_t tms_cutime; /**< User CPU time of terminated children.   */
		clock_t tms_cstime; /**< System CPU time of terminated children. */
	};

	extern clock_t times(struct tms *buffer);

#endif /* _ASM_FILE_ */
#endif /* TIMES_H_ */
