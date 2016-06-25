/* 
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix.  If not, see <http://www.gnu.org/licenses/>.
 */

#ifndef VGA_H_
#define VGA_H_

/**
 * Bochs VBE Definitions.
 *
 * @note Unnecessary elements have been removed here.
 */
#define VBE_DISPI_TOTAL_VIDEO_MEMORY_MB  16   /* Max video memory. */

#define VBE_DISPI_MAX_XRES               2560 /* Max X res. */
#define VBE_DISPI_MAX_YRES               1600 /* Max Y res. */
#define VBE_DISPI_MAX_BPP                32   /* Max depth. */

#define VBE_DISPI_IOPORT_INDEX           0x01CE /* Index register.  */
#define VBE_DISPI_IOPORT_DATA            0x01CF /* Data register.   */

#define VBE_DISPI_INDEX_ID               0x0 /* Supported mode. */
#define VBE_DISPI_INDEX_XRES             0x1 /* Current X res.  */
#define VBE_DISPI_INDEX_YRES             0x2 /* Current Y res.  */
#define VBE_DISPI_INDEX_BPP              0x3 /* Current depth.  */
#define VBE_DISPI_INDEX_ENABLE           0x4 /* Enable register. */

#define VBE_DISPI_ID0                    0xB0C0 /* BGA version 0. */
#define VBE_DISPI_ID1                    0xB0C1 /* BGA version 1. */
#define VBE_DISPI_ID2                    0xB0C2 /* BGA version 2. */
#define VBE_DISPI_ID3                    0xB0C3 /* BGA version 3. */
#define VBE_DISPI_ID4                    0xB0C4 /* BGA version 4. */
#define VBE_DISPI_ID5                    0xB0C5 /* BGA version 5. */

#define VBE_DISPI_BPP_4                  0x04 /* 04 bits per pixel. */
#define VBE_DISPI_BPP_8                  0x08 /* 08 bits per pixel. */
#define VBE_DISPI_BPP_15                 0x0F /* 15 bits per pixel. */
#define VBE_DISPI_BPP_16                 0x10 /* 16 bits per pixel. */
#define VBE_DISPI_BPP_24                 0x18 /* 24 bits per pixel. */
#define VBE_DISPI_BPP_32                 0x20 /* 32 bits per pixel. */

#define VBE_DISPI_DISABLED               0x00 /* VBE Disable.             */
#define VBE_DISPI_ENABLED                0x01 /* VBE Enable.              */
#define VBE_DISPI_GETCAPS                0x02 /* Get resources supported. */
#define VBE_DISPI_8BIT_DAC               0x20 /* DAC register.            */
#define VBE_DISPI_LFB_ENABLED            0x40 /* Enables LFB.             */
#define VBE_DISPI_NOCLEARMEM             0x80 /* Dont clear video mem.    */

#define VBE_DISPI_LFB_PHYSICAL_ADDRESS   0xE0000000 /* Physical address.  */

/**
 * Video definitions.
 */
#define SCREEN_WIDTH  1024                    /* Screen width.       */
#define SCREEN_HEIGHT 768                     /* Screen height.      */
#define DEPTH 32                              /* Screen color depht. */
#define PITCH  ((SCREEN_WIDTH * DEPTH) >> 3)  /* Bytes per row.      */
#define STRIDE (DEPTH >> 3)                   /* Pixel width.        */

/* Get screen video video by x/y coordinates. */
#define VIDEO_OFFSET(x,y) ( ( (y)*PITCH + (x)*STRIDE ) / (DEPTH >> 3) )

/**
 * Check if available and initialize Bochs VBE Extensions
 */
extern void vga_init(void);

#endif /* VGA_H_ */
