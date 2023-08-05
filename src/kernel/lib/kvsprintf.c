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

#include <nanvix/const.h>
#include <stdarg.h>

/**
 * @brief Converts an integer to a string.
 *
 * @param str  Output string.
 * @param num  Number to be converted.
 * @param base Base to use.
 *
 * @returns The length of the output string.
 */
PUBLIC int itoa(char *str, unsigned num, int base)
{
	char *b = str;
	char *p, *p1, *p2;
	unsigned divisor;

	if (base == 'd')
		divisor = 10;

	else
	{
		*b++ = '0'; *b++ = 'x';
		divisor = 16;
	}

	p = b;

	/* Convert number. */
	do
	{
		int remainder = num % divisor;

		*p++ = (remainder < 10) ? remainder + '0' :
		                          remainder + 'a' - 10;
	} while (num /= divisor);

	/* Fill up with zeros. */
	if (divisor == 16)
		while ((p - b) < 8)
			*p++ = '0';

	/* Reverse BUF. */
	p1 = b; p2 = p - 1;
	while (p1 < p2)
	{
		char tmp = *p1;
		*p1++ = *p2;
		*p2-- = tmp;
	}

	return(p - str);
}

/**
 * @brief Writes formatted data from variable argument list to a string.
 *
 * @param str  Output string.
 * @param fmt  Formated string.
 * @param args Variable arguments list.
 *
 * @returns The length of the output string.
 */
PUBLIC int kvsprintf(char *str, const char *fmt, va_list args)
{
	char *base = str;
	char *s;

    /* Format string. */
    while (*fmt != '\0')
    {
        /* No conversion needed. */
        if (*fmt != '%')
            *str++ = *fmt;

        /* Parse format. */
        else
            switch (*(++fmt))
            {
				/* Character. */
                case 'c':
					*str++ = va_arg(args, char);
					break;

				/* Number. */
				case 'd':
				case 'x':
					str += itoa(str, va_arg(args, unsigned int), *fmt);
					break;

				/* String. */
                case 's':
					s = va_arg(args, char*);
                    while (*s != '\0')
						*str++ = *s++;
					break;

                /* Ignore. */
                default:
                    break;
            }
        fmt++;
    }

    return (str - base);
}
