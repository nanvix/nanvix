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

#ifndef _MINIX_H_
#define _MINIX_H_

 	#include <sys/types.h>
 	#include <minix.h>

	/* Forward definitions. */
	extern void minix_inode_write(uint16_t, struct d_inode *);
	extern uint16_t dir_search(struct d_inode *, const char *);
	extern void minix_mount(const char *);
	extern void minix_umount(void);
	extern struct d_inode *minix_inode_read(uint16_t);
	extern uint16_t minix_mkdir(struct d_inode *, uint16_t, const char *, uint16_t, uint16_t);
	extern void minix_mknod(struct d_inode *, const char *, uint16_t, uint16_t, uint16_t, uint16_t);
	extern uint16_t minix_inode_dname(const char *, char *);
	extern uint16_t minix_create(const char *, uint16_t, uint16_t, uint16_t);
	extern void minix_write(uint16_t, const void *, size_t);
	extern void minix_mkfs(const char *, uint16_t, uint16_t, uint16_t, uint16_t);

#endif /* _MINIX_H_ */
