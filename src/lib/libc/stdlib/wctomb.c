/*
 * Copyright(C) 2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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
 * Copyright (c) 1994-2009  Red Hat, Inc. All rights reserved.
 *
 * This copyrighted material is made available to anyone wishing to use,
 * modify, copy, or redistribute it subject to the terms and conditions
 * of the BSD License.   This program is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY expressed or implied,
 * including the implied warranties of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE.  A copy of this license is available at 
 * http://www.opensource.org/licenses. Any Red Hat trademarks that are
 * incorporated in the source code or documentation are not subject to
 * the BSD License and may only be used or replicated with the express
 * permission of Red Hat, Inc.
 */

/*
 * Copyright (c) 1981-2000 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, 
 *       this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *     * Neither the name of the University nor the names of its contributors 
 *       may be used to endorse or promote products derived from this software 
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

#ifndef _REENT_ONLY

#include <newlib.h>
#include <stdlib.h>
#include <errno.h>
#include "local.h"

/**
 * @brief Converts a wide-character code to a character.
 *
 * @details Determines the number of bytes needed to represent
 * the character corresponding to the wide-character code whose
 * value is @p wchar (including any change in the shift state).
 * Stores the character representation (possibly multiple bytes
 * and any special bytes to change shift state) in the array 
 * object pointed to by @p s (if @p s is not a null pointer).
 * At most {MB_CUR_MAX} bytes is stored. If @p wchar is 0, a null
 * byte is stored, preceded by any shift sequence needed to restore
 * the initial shift state, and wctomb() lefts in the initial shift
 * state.
 *
 * @return If @p s is a null pointer, wctomb() returns a non-zero or
 * 0 value, if character encodings, respectively, do or do not have
 * state-dependent encodings. If @p s is not a null pointer, wctomb()
 * returns -1 if the value of @p wchar does not correspond to a valid
 * character, or returns the number of bytes that constitute the 
 * character corresponding to the value of @p wchar.
 */
int wctomb(char *s, wchar_t wchar)
{
#ifdef _MB_CAPABLE
	struct _reent *reent = _REENT;

        _REENT_CHECK_MISC(reent);

        return __wctomb (reent, s, wchar, __locale_charset (),
			 &(_REENT_WCTOMB_STATE(reent)));
#else /* not _MB_CAPABLE */
        if (s == NULL)
                return 0;

	/* Verify that wchar is a valid single-byte character.  */
	if ((size_t)wchar >= 0x100) {
		errno = EILSEQ;
		return -1;
	}

        *s = (char) wchar;
        return 1;
#endif /* not _MB_CAPABLE */
}

#endif /* !_REENT_ONLY */
