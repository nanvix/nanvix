/*
 * Copyright(C) 2011-2015 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2015-2015 Davidson Francis <davidsondfgl@gmail.com>
 *              2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#ifndef TSH_H_
#define TSH_H_

	#include <stdlib.h>

	/* Software versioning. */
	#define TSH_NAME    "tsh" /* Name.    */
	#define TSH_VERSION "1.0" /* Version. */

	/* Software copyright message. */
	#define SH_COPYRIGHT \
		"Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>\n"

	/* Maximum line length. */
	#define LINELEN 256

	/**
	 * @brief History size.
	 */
	#define HISTORY_SIZE 50

	/* Max arguments of a command. */
	#define CMD_MAXARGS 32

	/* Command flags. */
	#define CMD_ASYNC 001 /* Asynchronous? */
	#define CMD_PIPE  002 /* Piping?       */

	/* Shell flags. */
	#define SH_INTERACTIVE 001 /* Interactive? */

	/*
	 * Aborts if not an interactive shell.
	 */
	#define sherror()                    \
		if (!(shflags & SH_INTERACTIVE)) \
			exit(EXIT_FAILURE);

	/* Shell flags. */
	extern int shflags;

	/* Shell return value. */
	extern int shret;

#endif /* TSH_H_ */
