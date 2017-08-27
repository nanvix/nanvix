/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
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

#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <locale.h>
#include "mbctype.h"
#include "local.h"

int (*__wctomb) (struct _reent *, char *, wchar_t, const char *charset,
		 mbstate_t *)
#ifdef __CYGWIN__
   /* Cygwin starts up in UTF-8 mode. */
    = __utf8_wctomb;
#else
    = __ascii_wctomb;
#endif

int
_DEFUN (_wctomb_r, (r, s, wchar, state),
        struct _reent *r     _AND 
        char          *s     _AND
        wchar_t        _wchar _AND
        mbstate_t     *state)
{
  return __wctomb (r, s, _wchar, __locale_charset (), state);
}

int
_DEFUN (__ascii_wctomb, (r, s, wchar, charset, state),
        struct _reent *r       _AND 
        char          *s       _AND
        wchar_t        _wchar  _AND
	const char    *charset _AND
        mbstate_t     *state)
{
  ((void)charset);
  ((void)state);
  /* Avoids compiler warnings about comparisons that are always false
     due to limited range when sizeof(wchar_t) is 2 but sizeof(wint_t)
     is 4, as is the case on cygwin.  */
  wint_t wchar = _wchar;

  if (s == NULL)
    return 0;
 
#ifdef __CYGWIN__
  if ((size_t)wchar >= 0x80)
#else
  if ((size_t)wchar >= 0x100)
#endif
    {
      r->_errno = EILSEQ;
      return -1;
    }

  *s = (char) wchar;
  return 1;
}

#ifdef _MB_CAPABLE
/* for some conversions, we use the __count field as a place to store a state value */
#define __state __count

int
_DEFUN (__utf8_wctomb, (r, s, wchar, charset, state),
        struct _reent *r       _AND 
        char          *s       _AND
        wchar_t        _wchar  _AND
	const char    *charset _AND
        mbstate_t     *state)
{
  wint_t wchar = _wchar;
  int ret = 0;

  if (s == NULL)
    return 0; /* UTF-8 encoding is not state-dependent */

  if (sizeof (wchar_t) == 2 && state->__count == -4
      && (wchar < 0xdc00 || wchar >= 0xdfff))
    {
      /* There's a leftover lone high surrogate.  Write out the CESU-8 value
	 of the surrogate and proceed to convert the given character.  Note
	 to return extra 3 bytes. */
      wchar_t tmp;
      tmp = (state->__value.__wchb[0] << 16 | state->__value.__wchb[1] << 8)
	    - (0x10000 >> 10 | 0xd80d);
      *s++ = 0xe0 | ((tmp & 0xf000) >> 12);
      *s++ = 0x80 | ((tmp &  0xfc0) >> 6);
      *s++ = 0x80 |  (tmp &   0x3f);
      state->__count = 0;
      ret = 3;
    }
  if (wchar <= 0x7f)
    {
      *s = wchar;
      return ret + 1;
    }
  if (wchar >= 0x80 && wchar <= 0x7ff)
    {
      *s++ = 0xc0 | ((wchar & 0x7c0) >> 6);
      *s   = 0x80 |  (wchar &  0x3f);
      return ret + 2;
    }
  if (wchar >= 0x800 && wchar <= 0xffff)
    {
      /* No UTF-16 surrogate handling in UCS-4 */
      if (sizeof (wchar_t) == 2 && wchar >= 0xd800 && wchar <= 0xdfff)
	{
	  wint_t tmp;
	  if (wchar <= 0xdbff)
	    {
	      /* First half of a surrogate pair.  Store the state and
	         return ret + 0. */
	      tmp = ((wchar & 0x3ff) << 10) + 0x10000;
	      state->__value.__wchb[0] = (tmp >> 16) & 0xff;
	      state->__value.__wchb[1] = (tmp >> 8) & 0xff;
	      state->__count = -4;
	      *s = (0xf0 | ((tmp & 0x1c0000) >> 18));
	      return ret;
	    }
	  if (state->__count == -4)
	    {
	      /* Second half of a surrogate pair.  Reconstruct the full
		 Unicode value and return the trailing three bytes of the
		 UTF-8 character. */
	      tmp = (state->__value.__wchb[0] << 16)
		    | (state->__value.__wchb[1] << 8)
		    | (wchar & 0x3ff);
	      state->__count = 0;
	      *s++ = 0xf0 | ((tmp & 0x1c0000) >> 18);
	      *s++ = 0x80 | ((tmp &  0x3f000) >> 12);
	      *s++ = 0x80 | ((tmp &    0xfc0) >> 6);
	      *s   = 0x80 |  (tmp &     0x3f);
	      return 4;
	    }
	  /* Otherwise translate into CESU-8 value. */
	}
      *s++ = 0xe0 | ((wchar & 0xf000) >> 12);
      *s++ = 0x80 | ((wchar &  0xfc0) >> 6);
      *s   = 0x80 |  (wchar &   0x3f);
      return ret + 3;
    }
  if (wchar >= 0x10000 && wchar <= 0x10ffff)
    {
      *s++ = 0xf0 | ((wchar & 0x1c0000) >> 18);
      *s++ = 0x80 | ((wchar &  0x3f000) >> 12);
      *s++ = 0x80 | ((wchar &    0xfc0) >> 6);
      *s   = 0x80 |  (wchar &     0x3f);
      return 4;
    }

  r->_errno = EILSEQ;
  return -1;
}

/* Cygwin defines its own doublebyte charset conversion functions 
   because the underlying OS requires wchar_t == UTF-16. */
#ifndef __CYGWIN__
int
_DEFUN (__sjis_wctomb, (r, s, wchar, charset, state),
        struct _reent *r       _AND 
        char          *s       _AND
        wchar_t        _wchar  _AND
	const char    *charset _AND
        mbstate_t     *state)
{
  wint_t wchar = _wchar;

  unsigned char char2 = (unsigned char)wchar;
  unsigned char char1 = (unsigned char)(wchar >> 8);

  if (s == NULL)
    return 0;  /* not state-dependent */

  if (char1 != 0x00)
    {
    /* first byte is non-zero..validate multi-byte char */
      if (_issjis1(char1) && _issjis2(char2)) 
	{
	  *s++ = (char)char1;
	  *s = (char)char2;
	  return 2;
	}
      else
	{
	  r->_errno = EILSEQ;
	  return -1;
	}
    }
  *s = (char) wchar;
  return 1;
}

int
_DEFUN (__eucjp_wctomb, (r, s, wchar, charset, state),
        struct _reent *r       _AND 
        char          *s       _AND
        wchar_t        _wchar  _AND
	const char    *charset _AND
        mbstate_t     *state)
{
  wint_t wchar = _wchar;
  unsigned char char2 = (unsigned char)wchar;
  unsigned char char1 = (unsigned char)(wchar >> 8);

  if (s == NULL)
    return 0;  /* not state-dependent */

  if (char1 != 0x00)
    {
    /* first byte is non-zero..validate multi-byte char */
      if (_iseucjp1 (char1) && _iseucjp2 (char2)) 
	{
	  *s++ = (char)char1;
	  *s = (char)char2;
	  return 2;
	}
      else if (_iseucjp2 (char1) && _iseucjp2 (char2 | 0x80))
	{
	  *s++ = (char)0x8f;
	  *s++ = (char)char1;
	  *s = (char)(char2 | 0x80);
	  return 3;
	}
      else
	{
	  r->_errno = EILSEQ;
	  return -1;
	}
    }
  *s = (char) wchar;
  return 1;
}

int
_DEFUN (__jis_wctomb, (r, s, wchar, charset, state),
        struct _reent *r       _AND 
        char          *s       _AND
        wchar_t        _wchar  _AND
	const char    *charset _AND
        mbstate_t     *state)
{
  wint_t wchar = _wchar;
  int cnt = 0; 
  unsigned char char2 = (unsigned char)wchar;
  unsigned char char1 = (unsigned char)(wchar >> 8);

  if (s == NULL)
    return 1;  /* state-dependent */

  if (char1 != 0x00)
    {
    /* first byte is non-zero..validate multi-byte char */
      if (_isjis (char1) && _isjis (char2)) 
	{
	  if (state->__state == 0)
	    {
	      /* must switch from ASCII to JIS state */
	      state->__state = 1;
	      *s++ = ESC_CHAR;
	      *s++ = '$';
	      *s++ = 'B';
	      cnt = 3;
	    }
	  *s++ = (char)char1;
	  *s = (char)char2;
	  return cnt + 2;
	}
      r->_errno = EILSEQ;
      return -1;
    }
  if (state->__state != 0)
    {
      /* must switch from JIS to ASCII state */
      state->__state = 0;
      *s++ = ESC_CHAR;
      *s++ = '(';
      *s++ = 'B';
      cnt = 3;
    }
  *s = (char)char2;
  return cnt + 1;
}
#endif /* !__CYGWIN__ */

#ifdef _MB_EXTENDED_CHARSETS_ISO
int
_DEFUN (__iso_wctomb, (r, s, wchar, charset, state),
        struct _reent *r       _AND 
        char          *s       _AND
        wchar_t        _wchar  _AND
	const char    *charset _AND
        mbstate_t     *state)
{
  wint_t wchar = _wchar;

  if (s == NULL)
    return 0;

  /* wchars <= 0x9f translate to all ISO charsets directly. */
  if (wchar >= 0xa0)
    {
      int iso_idx = __iso_8859_index (charset + 9);
      if (iso_idx >= 0)
	{
	  unsigned char mb;

	  if (s == NULL)
	    return 0;

	  for (mb = 0; mb < 0x60; ++mb)
	    if (__iso_8859_conv[iso_idx][mb] == wchar)
	      {
		*s = (char) (mb + 0xa0);
		return 1;
	      }
	  r->_errno = EILSEQ;
	  return -1;
	}
    }
 
  if ((size_t)wchar >= 0x100)
    {
      r->_errno = EILSEQ;
      return -1;
    }

  *s = (char) wchar;
  return 1;
}
#endif /* _MB_EXTENDED_CHARSETS_ISO */

#ifdef _MB_EXTENDED_CHARSETS_WINDOWS
int
_DEFUN (__cp_wctomb, (r, s, wchar, charset, state),
        struct _reent *r       _AND 
        char          *s       _AND
        wchar_t        _wchar  _AND
	const char    *charset _AND
        mbstate_t     *state)
{
  wint_t wchar = _wchar;

  if (s == NULL)
    return 0;

  if (wchar >= 0x80)
    {
      int cp_idx = __cp_index (charset + 2);
      if (cp_idx >= 0)
	{
	  unsigned char mb;

	  if (s == NULL)
	    return 0;

	  for (mb = 0; mb < 0x80; ++mb)
	    if (__cp_conv[cp_idx][mb] == wchar)
	      {
		*s = (char) (mb + 0x80);
		return 1;
	      }
	  r->_errno = EILSEQ;
	  return -1;
	}
    }

  if ((size_t)wchar >= 0x100)
    {
      r->_errno = EILSEQ;
      return -1;
    }

  *s = (char) wchar;
  return 1;
}
#endif /* _MB_EXTENDED_CHARSETS_WINDOWS */
#endif /* _MB_CAPABLE */
