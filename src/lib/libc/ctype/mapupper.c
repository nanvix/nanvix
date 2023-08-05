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

/*
 * Copyright (c) 1989 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 * 4. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/**
 * @brief Uppercase character map.
 */
const char _mapupper[1 + 256] = {
	0,
	0000,	0001,	0002,	0003,	0004,	0005,	0006,	0007,
	0010,	0011,	0012,	0013,	0014,	0015,	0016,	0017,
	0020,	0021,	0022,	0023,	0024,	0025,	0026,	0027,
	0030,	0031,	0032,	0033,	0034,	0035,	0036,	0037,
	0040,	0041,	0042,	0043,	0044,	0045,	0046,	0047,
	0050,	0051,	0052,	0053,	0054,	0055,	0056,	0057,
	0060,	0061,	0062,	0063,	0064,	0065,	0066,	0067,
	0070,	0071,	0072,	0073,	0074,	0075,	0076,	0077,
	0100,	 'A',	 'B',	 'C',	 'D',	 'E',	 'F',	 'G',
	 'H',	 'I',	 'J',	 'K',	 'L',	 'M',	 'N',	 'O',
	 'P',	 'Q',	 'R',	 'S',	 'T',	 'U',	 'V',	 'W',
	 'X',	 'Y',	 'Z',	0133,	0134,	0135,	0136,	0137,
	0140,	 'A',	 'B',	 'C',	 'D',	 'E',	 'F',	 'G',
	 'H',	 'I',	 'J',	 'K',	 'L',	 'M',	 'N',	 'O',
	 'P',	 'Q',	 'R',	 'S',	 'T',	 'U',	 'V',	 'W',
	 'X',	 'Y',	 'Z',	0173,	0174,	0175,	0176,	0177,
};
