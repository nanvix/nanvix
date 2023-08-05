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
 * Copyright (C) 1991-1996 Free Software Foundation, Inc.
 *
 * This file is part of the GNU C Library.
 *
 * The GNU C Library free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * The GNU C Library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
 * MA 02110-1301, USA.
 */

/**
 * @file
 *
 * @brief mbtowc() implementation.
 */

#include <stdlib.h>
#include <wchar.h>
#include <errno.h>

/**
 * @brief Number of bytes needed for the current character.
 */
static int count = 0;
static wint_t value = 0;

/**
 * @brief Internal mbtowc().
 */
static size_t _mbtowc(wchar_t *pwc, const char *s, size_t n)
{
	size_t used = 0;

	if (s == NULL)
    {
		pwc = NULL;
		s = "";
		n = 1;
	}

	if (n > 0)
	{
		if (count == 0)
		{
			unsigned char byte = (unsigned char) *s++;
			++used;

			/* We must look for a possible first byte of a UTF8 sequence.  */
			if (byte < 0x80)
			{
				/* One byte sequence.  */
				if (pwc != NULL)
					*pwc = (wchar_t) byte;
				return byte ? used : 0;
			}

			if ((byte & 0xc0) == 0x80 || (byte & 0xfe) == 0xfe)
			{
				/* Oh, oh.  An encoding error.  */
				errno = EILSEQ;
				return (size_t) -1;
			}

			if ((byte & 0xe0) == 0xc0)
			{
				/* We expect two bytes.  */
				count = 1;
				value = byte & 0x1f;
			}
			else if ((byte & 0xf0) == 0xe0)
			{
				/* We expect three bytes.  */
				count = 2;
				value = byte & 0x0f;
			}
			else if ((byte & 0xf8) == 0xf0)
			{
				/* We expect four bytes.  */
				count = 3;
				value = byte & 0x07;
			}
			else if ((byte & 0xfc) == 0xf8)
			{
				/* We expect five bytes.  */
				count = 4;
				value = byte & 0x03;
			}
			else
			{
				/* We expect six bytes.  */
				count = 5;
				value = byte & 0x01;
			}
		}

		/*
		 * We know we have to handle a multibyte character and there
		 * are some more bytes to read.
		 */
		while (used < n)
		{
			/* The second to sixths byte must be of the form 10xxxxxx.  */
			unsigned char byte = (unsigned char) *s++;
			++used;

			if ((byte & 0xc0) != 0x80)
			{
				/* Oh, oh.  An encoding error.  */
				errno = EILSEQ;
				return (size_t) -1;
			}

			value <<= 6;
			value |= byte & 0x3f;

			if (--count == 0)
			{
				/* The character is finished.  */
				if (pwc != NULL)
					*pwc = (wchar_t) value;
				return value ? used : 0;
			}
		}
	}

	return ((size_t) -2);
}

/**
 * @brief Converts a character to a wide-character code.
 *
 * @param pwc Wide-character code.
 * @param s   Wide-character.
 * @param n   Number of bytes to consider.
 *
 * @returns If @p s is a null pointer, mbtowc() returns a non-zero or 0 value,
 *          if character encodings, respectively, do or do not have state-
 *          dependent encodings. If @p s is not a null pointer, mbtowc() either
 *          returns 0 (if s points to the null byte), or returns the number of
 *          bytes that constitute the converted character (if the next @p n or
 *          fewer bytes form a valid character), or return -1 and sets errno to
 *          indicate the error (if they do not form a valid character).
 *
 *          In no case the value returned is greater than @p n or the value of
 *          the MB_CUR_MAX macro.
 *
 * @note The mbtowc() function is not thread-safe.
 */
int mbtowc(wchar_t *pwc, const char *s, size_t n)
{
	int result;

	/*
	 * If S is NULL the function has to return null or not null
	 * depending on the encoding having a state depending encoding or
	 * not.  This is nonsense because any multibyte encoding has a
	 * state.  The ISO C amendment 1 corrects this while introducing the
	 * restartable functions. We simply say here all encodings have a
	 * state.
	 */
	if (s == NULL)
		return (1);

	result = _mbtowc(pwc, s, n);

	/*
	 * The `mbrtowc' functions tell us more than we need.
	 * Fold the -1 and -2 result into -1.
	 */
	if (result < 0)
	result = -1;

	return (result);
}
