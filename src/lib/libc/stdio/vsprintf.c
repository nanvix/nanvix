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

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Converts an integer to a string.
 */
static int itoa(char *string, unsigned num, int base)
{
	char tmp;          /* Temporary variable. */
	char *s;           /* Working substring.  */
	char *p, *p1, *p2; /* Working characters. */
	unsigned divisor;  /* Base divisor.       */

	s = string;

	if (strchr("ud", base) != NULL)
		divisor = 10;

	else
	{
		*s++ = '0'; *s++ = 'x';
		divisor = 16;
	}

	p = s;

	/* Convert number. */
	do
	{
		int remainder = num % divisor;

		*p++ = (remainder < 10) ? remainder + '0' :
		                          remainder + 'a' - 10;
	} while (num /= divisor);

	/* Fill up with zeros. */
	if (divisor == 16)
		while ((p - s) < 8)
			*p++ = '0';

	/* Reverse BUF. */
	p1 = s; p2 = p - 1;
	while (p1 < p2)
	{
		tmp = *p1;
		*p1++ = *p2;
		*p2-- = tmp;
	}

	return(p - string);
}

/*
 * Writes format output of a stdarg argument list to a string.
 */
int vsprintf(char *string, const char *format, va_list ap)
{
	char *s;    /* Working string. */
	char *base; /* Base string.    */

	base = string;

    /* Format string. */
    while (*format != '\0')
    {
        /* No conversion needed. */
        if (*format != '%')
            *string++ = *format;

        /* Parse format. */
        else
        {
            switch (*(++format))
            {
				/* Character. */
                case 'c':
					*string++ = va_arg(ap, char);
					break;

				/* Decimal. */
				case 'd':
				case 'u':
					string += itoa(string, va_arg(ap, unsigned int), *format);
					break;

				/* Hexadecimal. */
				case 'X':
				case 'x':
					string += itoa(string, va_arg(ap, unsigned int), *format);
					break;

				/* String. */
                case 's':
					s = va_arg(ap, char*);
                    while (*s != '\0')
						*string++ = *s++;
					break;

                /* Ignore. */
                default:
                    break;
            }
		}

        format++;
    }

    *string++ = '\0';

    return (string - base);
}
