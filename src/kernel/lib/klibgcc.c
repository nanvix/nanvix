/*
 * Copyright(C) 2017-2018 Davidson Francis <davidsondfgl@gmail.com>
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

unsigned long
udivmodsi4(unsigned long num, unsigned long den, int modwanted)
{
	unsigned long bit = 1;
	unsigned long res = 0;

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

	if (modwanted) 
		return num;
	
	return res;
}

/**
 * @brief Calculates the quotient of the signed division of a and b.
 * 
 * @param a first number
 * @param b second number
 * 
 * @returns Returns the quotient of the signed division of a and b.
 */
long __divsi3 (long a, long b)
{
	int neg = 0;
	long res;

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

/**
 * @brief Calculates the remainder of the signed division of a and b.
 * 
 * @param a first number
 * @param b second number
 * 
 * @returns Returns the remainder of the signed division of a and b.
 */
long __modsi3 (long a, long b)
{
	int neg = 0;
	long res;

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

/**
 * @brief Calculates the quotient of the unsigned division of a and b.
 * 
 * @param a first number
 * @param b second number
 * 
 * @returns Returns the quotient of the unsigned division of a and b.
 */
long __udivsi3 (long a, long b)
{
	return udivmodsi4 (a, b, 0);
}

/**
 * @brief Calculates the remainder of the unsigned division of a and b.
 * 
 * @param a first number
 * @param b second number
 * 
 * @returns Returns the remainder of the unsigned division of a and b.
 */
long __umodsi3 (long a, long b)
{
	return udivmodsi4 (a, b, 1);
}
