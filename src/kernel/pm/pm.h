/*
 * Copyright(C) 2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#ifndef _PM_H_
#define _PM_H_

	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	
	/*
	 * Function: abort
	 * 
	 * Aborts the current running process.
	 */
	EXTERN void abort(int err);
	
	/*
	 * Function: resume
	 * 
	 * Resumes a process.
	 */
	EXTERN void resume(struct process *proc);
	
	/*
	 * Function: stop
	 * 
	 * Stops the current running process.
	 */
	EXTERN void stop(void);
	
	/*
	 * Function: terminate
	 * 
	 * Terminates the current running process.
	 */
	EXTERN void terminate(int err);

#endif /* _PM_H_ */
