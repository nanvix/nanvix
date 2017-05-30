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
 * @brief Inode module implementation.
 */
#ifndef _INODE_MINIX_H_
#define _INODE_MINiX_H_

	#include <nanvix/clock.h>
	#include <nanvix/config.h>
	#include <nanvix/const.h>
	#include <nanvix/dev.h>
	#include <nanvix/fs.h>
	#include <nanvix/klib.h>
	#include <nanvix/mm.h>
	#include <errno.h>
	#include <limits.h>
	#include "fs.h"

	PUBLIC void inode_write_minix(struct inode *ip);
	PUBLIC int inode_read_minix(dev_t dev, ino_t num, struct inode *ip);
	PUBLIC void inode_free_minix(struct inode *ip);
	PUBLIC void inode_truncate_minix(struct inode *ip);
	PUBLIC int inode_alloc_minix(struct superblock *sb, struct inode *ip);
	PUBLIC void init_minix (void);
	
#endif