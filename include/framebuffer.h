/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */
#ifndef FRAMEBUFFER_H_
#define FRAMEBUFFER_H_
#ifndef _ASM_FILE_

#include <nanvix/const.h>

#define CENTER 0x1000 /* Centralize image. */

/**
 * @brief Structure used to draw an image on the screen.
 *
 * @note The element 'buff' is an sequence of RGB pixels with
 * 32 bpp. Another thing to pay attention, is that, the first
 * element of buff, points to the top left of image, i.e, (0,0).
 */
struct frameBuff_opts
{
	int x;                  /* X position.                   */
	int y;                  /* Y position.                   */
	unsigned char *buff;    /* Buffer to image.              */
	int start;              /* Start offset of buffer.       */
	int width;              /* Width.                        */
	int height;             /* Height.                       */
	int size;               /* Size of buffer.               */
	unsigned tColor;        /* Transparent color, 0 if none. */
};

/* Draws on the screen buffer an given buffer as a parameter. */
EXTERN int drawBuffer(struct frameBuff_opts *fbo);

#endif /* _ASM_FILE_ */
#endif /* FRAMEBUFFER_H_ */
