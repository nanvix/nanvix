/*
 * Copyright(C) 2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/config.h>
#include <stdio.h>
#include <stdlib.h>

/**
 * @brief Swapping test module.
 * 
 * @details Forces swapping algorithms to be activated by performing a large
 *          matrix multiplication operation that does not fit on memory.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
int swap_test(void)
{
	int ret;        /* Return value. */
	int *a, *b, *c; /* Matrices.     */
	
	ret = 0;
	
	a = malloc(2048*2048*sizeof(int));
	b = malloc(2048*2048*sizeof(int));
	c = malloc(2048*2048*sizeof(int));
	
	/* Initialize matrices. */
	for (int i = 0; i < 2048*2048; i++)
	{
		a[i] = 1;
		b[i] = 1;
		c[i] = 0;
	}
	
	/* Multiply matrices. */
	for (int i = 0; i < 2048; i++)
	{
		for (int j = 0; j < 2048; j++)
		{
			for (int k = 0; k < 2048; k++)
				c[i*2048 + j] += a[i*2048 + k]*b[k*2048 + j];
		}
	}
	
	/* Check values. */
	for (int i = 0; i < 2048*2048; i++)
	{
		if (c[i] != 2)
		{
			ret = -1;
			break;
		}
	}
	
	/* House keeping. */
	free(a);
	free(b);
	free(c);
	
	return (ret);
}

/**
 * @brief System testing utility.
 */
int main(int argc, char **argv)
{
	((void)argc);
	((void)argv);
	
	printf("Swap Test [%s]\n", (!swap_test()) ? "PASSED" : "FAILED");
	
	return (EXIT_SUCCESS);
}
