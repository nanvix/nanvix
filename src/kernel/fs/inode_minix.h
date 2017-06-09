/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Romane Gallier <romanegallier@gmail.com>
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
 * @brief Inode module implementation.
 */
#ifndef _INODE_MINIX_H_
#define _INODE_MINiX_H_

	#include <nanvix/const.h>
	#include <nanvix/dev.h>
	#include <nanvix/fs.h>

	/* Forward definitions. */
	EXTERN void inode_write_minix(struct inode *);
	EXTERN int inode_read_minix(dev_t, ino_t, struct inode *);
	EXTERN void inode_free_minix(struct inode *);
	EXTERN void inode_truncate_minix(struct inode *);
	EXTERN int inode_alloc_minix(struct superblock *, struct inode *);
	EXTERN void init_minix (void);
	
#endif
