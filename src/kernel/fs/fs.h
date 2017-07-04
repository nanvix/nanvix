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
 * @brief Private file system interface.
 */

#ifndef _FS_H_
#define _FS_H_

	#include <nanvix/const.h>
	#include <nanvix/fs.h>
	#include <stdint.h>
	#include <limits.h>	
	
	  /**
   * @brief add the ROOT_DEV in the mount table.
   */
  PUBLIC struct inode * mountRoot ();
  /**@}*/

#endif /* _FS_H_ */
