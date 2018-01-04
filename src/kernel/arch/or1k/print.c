/*
 * Copyright(C) 2017-2017 Davidson Francis <davidsondfgl@gmail.com>
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

/* Copyright (C) 2000-2015 Free Software Foundation, Inc.

This file is part of GCC.

GCC is free software; you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free
Software Foundation; either version 3, or (at your option) any later
version.

GCC is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
for more details.

Under Section 7 of GPL version 3, you are granted additional
permissions described in the GCC Runtime Library Exception, version
3.1, as published by the Free Software Foundation.

You should have received a copy of the GNU General Public License and
a copy of the GCC Runtime Library Exception along with this program;
see the files COPYING3 and COPYING.RUNTIME respectively.  If not, see
<http://www.gnu.org/licenses/>.  */

extern void uart_write(char);

unsigned udivmodsi4(unsigned num, unsigned den, int modwanted)
{
	unsigned bit = 1;
	unsigned res = 0;

	while (den < num && bit && !(den & (1L<<31)))
	{
		den <<=1;
		bit <<=1;
	}
	while (bit)
	{
		if (num >= den)
		{
			num -= den;
			res |= bit;
		}
		bit >>=1;
		den >>=1;
	}
	if (modwanted) return num;
	return res;
}

int
__divsi3 (int a, int b)
{
	int neg = 0;
	int res;

	if (a < 0)
	{
		a = -a;
		neg = !neg;
	}

	if (b < 0)
	{
		b = -b;
		neg = !neg;
	}

	res = udivmodsi4 (a, b, 0);

	if (neg)
		res = -res;

	return res;
}

int
__modsi3 (int a, int b)
{
	int neg = 0;
	int res;

	if (a < 0)
	{
		a = -a;
		neg = 1;
	}

	if (b < 0)
		b = -b;

	res = udivmodsi4 (a, b, 1);

	if (neg)
		res = -res;

	return res;
}

void print(int n, int b)
{
	char buff[10];
	int count;

	if (n < 0 && b == 10)
	{
		n = -n;
		uart_write('-');
	}
	
	int r;
	count = 0;
	uart_write('0');
	uart_write('x');
	do
	{
		r = n % b;
		buff[count++] = (r < 10) ? r + '0' : r + 'A' - 10;

	} while (n /= b);

	for (int i = count - 1; i >= 0; i--)
		uart_write(buff[i]);

	uart_write('\n');
}

void printline(char *c)
{
	while(*c)
		uart_write(*c++);
	
	uart_write('\n');
}
