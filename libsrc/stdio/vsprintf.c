/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/vsprintf - vsprintf() implementation.
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
