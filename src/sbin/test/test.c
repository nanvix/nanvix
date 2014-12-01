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

/* Test flags. */
#define EXTENDED (1 << 0)
#define FULL     (1 << 1)

/* Test flags. */
static unsigned flags = FULL;

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
	#define N 1280
	int *a, *b, *c;
	
	/* Allocate matrices. */
	if ((a = malloc(N*N*sizeof(int))) == NULL)
		goto error0;
	if ((b = malloc(N*N*sizeof(int))) == NULL)
		goto error1;
	if ((c = malloc(N*N*sizeof(int))) == NULL)
		goto error2;
	
	/* Initialize matrices. */
	for (int i = 0; i < N*N; i++)
	{
		a[i] = 1;
		b[i] = 1;
		c[i] = 0;
	}
	
	/* Multiply matrices. */
	if (flags & (EXTENDED | FULL))
	{	
		for (int i = 0; i < N; i++)
		{
			for (int j = 0; j < N; j++)
			{
					
				for (int k = 0; k < N; k++)
					c[i*N + j] += a[i*N + k]*b[k*N + j];
			}
		}
	}
	
	/* Check values. */
	if (flags & FULL)
	{
		for (int i = 0; i < N*N; i++)
		{
			if (c[i] != N)
				goto error3;
		}
	}
	
	/* House keeping. */
	free(a);
	free(b);
	free(c);
	
	return (0);

error3:
	free(c);
error2:
	free(b);
error1:
	free(a);
error0:
	return (-1);
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
