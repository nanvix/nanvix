/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * kvsprintf.c - Kernel vsprintf()
 */

#include <nanvix/const.h>
#include <stdarg.h>

/*
 * Converts integer to string.
 */
PRIVATE int itoa(char *str, unsigned num, int base)
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

/*
 * Writes formatted data from variable argument list to string. 
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
