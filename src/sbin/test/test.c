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
	int *a, *b, *c;
	
	a = malloc(1280*1280*sizeof(int));
	if (a == NULL)
		goto error0;
	b = malloc(1280*1280*sizeof(int));
	if (a == NULL)
		goto error1;
	c = malloc(1280*1280*sizeof(int));
	if (a == NULL)
		goto error2;
	
	/* Initialize matrices. */
	for (int i = 0; i < 1280*1280; i++)
	{
		a[i] = 1;
		b[i] = 1;
		c[i] = 0;
	}
	
	/* Multiply matrices. */
	for (int i = 0; i < 1280; i++)
	{
		for (int j = 0; j < 1280; j++)
		{
				
			for (int k = 0; k < 1280; k++)
				c[i*1280 + j] += a[i*1280 + k]*b[k*1280 + j];
		}
	}
	
	/* Check values. */
	for (int i = 0; i < 1280*1280; i++)
	{
		if (c[i] != 2)
			goto error3;
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
