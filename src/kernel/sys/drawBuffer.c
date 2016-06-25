/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
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
#include <nanvix/const.h>
#include <framebuffer.h>
#include <dev/vga.h>

/**
 * @brief Draws on the screen buffer an given buffer as a parameter.
 *
 * @param fbo Frame Buffer Options, has the fields x, y, width, height.
 * One note to do, is that 'start' is the inicial offset inside buffer.
 *
 * The field 'buffer' is an unsigned char array that follows a sequence
 * of RGB colors with one byte each.
 *
 * @returns always 0.
 */
PUBLIC int sys_drawBuffer(struct frameBuff_opts *fbo)
{
	unsigned *video;  /* Video pointer.  */
	unsigned char *p; /* Image pointer.  */
	unsigned offset;  /* Current offset. */

	video = (unsigned *)VBE_DISPI_LFB_PHYSICAL_ADDRESS;
	p = fbo->buff;

	/* Checks if center. */
	if(fbo->x == CENTER || fbo->y == CENTER)
	{
		fbo->x = (SCREEN_WIDTH  - fbo->width ) >> 1;
		fbo->y = (SCREEN_HEIGHT - fbo->height) >> 1;
	}

	/* Calculates the offset. */
	offset = VIDEO_OFFSET(fbo->x, fbo->y);

	/* Draws. */
	for (int i = fbo->start; i < fbo->start + fbo->size; i += 3)
	{
		unsigned rgb = (p[i] << 16) | (p[i+1] << 8) | p[i+2];
		
		if(rgb != fbo->tColor)
			video[offset] = rgb;

		if ( (i-fbo->start+3) % (fbo->width*3) == 0 )
		{
			offset += SCREEN_WIDTH;
			offset -= fbo->width;
		}

		offset++;
	}

	return 0;
}
