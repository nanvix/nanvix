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
#include <sys/times.h>
#include <sys/wait.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

/* Test flags. */
#define EXTENDED (1 << 0)
#define FULL     (1 << 1)
#define VERBOSE  (1 << 2)

/* Test flags. */
static unsigned flags = VERBOSE;

/*============================================================================*
 *                               swap_test                                    *
 *============================================================================*/

/**
 * @brief Swapping test module.
 * 
 * @details Forces swapping algorithms to be activated by performing a large
 *          matrix multiplication operation that does not fit on memory.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
static int swap_test(void)
{
	#define N 1280
	int *a, *b, *c;
	struct tms timing;

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
	
	times(&timing);
	
	/* Print timing statistics. */
	if (flags & VERBOSE)
	{
		printf("  User:    %d\n", timing.tms_utime);
		printf("  System:  %d\n", timing.tms_stime);
	}
	
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

/*============================================================================*
 *                                sched_test                                  *
 *============================================================================*/

/**
 * @brief Scheduling testing module.
 * 
 * @details Forces the priority inversion problem, to check how well the system
 *          performs on dynamic priorities.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
static int sched_test(void)
{
	int i;             /* Loop index.                 */
	int a, b, c;       /* Used for dummy computation. */
	int fd;            /* File descriptor.            */
	pid_t pid;         /* Child process ID.           */
	struct tms timing; /* Timing information.         */
	clock_t t0, t1;    /* Elapsed times.              */
	char *buffer;      /* Buffer.                     */
	
	t0 = times(&timing);
		
	pid = fork();
	
	/* Failed to fork(). */
	if (pid < 0)
		return (-1);
	
	/* Parent process. */
	else if (pid > 0)
	{
		nice(0);
		
		/* Perform some computation. */
		c = 0;
		for (i = 0; i < 1024; i++)
		{
			a = 1 + i;
			for (b = 2; b < i; b++)
			{
				if ((i%b) == 0)
					a += b;
			}
			c += a;
		}
		
		wait(NULL);
	}
	
	/* Child process. */
	else
	{
		nice(2*NZERO);
		
		/* Allocate buffer. */
		buffer = malloc(HDD_SIZE);
		if (buffer == NULL)
			exit(EXIT_FAILURE);
		
		/* Open hdd. */
		fd = open("/dev/hdd", O_RDONLY);
		if (fd < 0)
			exit(EXIT_FAILURE);
		
		/* Read hdd. */
		if (read(fd, buffer, HDD_SIZE) != HDD_SIZE)
			exit(EXIT_FAILURE);
		
		/* House keeping. */
		free(buffer);
		close(fd);
		
		exit(EXIT_SUCCESS);
	}
	
	t1 = times(&timing);
	
	/* Print timing statistics. */
	if (flags & VERBOSE)
		printf("  Elapsed: %d\n", t1 - t0);
	
	return (0);
}

/*============================================================================*
 *                                   main                                     *
 *============================================================================*/

/**
 * @brief System testing utility.
 */
int main(int argc, char **argv)
{
	((void)argc);
	((void)argv);

	printf("Scheduling Test\n");
	printf("  Result:  [%s]\n", (!sched_test()) ? "PASSED" : "FAILED");

	printf("Swapping Test\n");
	printf("  Result:  [%s]\n", (!swap_test()) ? "PASSED" : "FAILED");

	return (EXIT_SUCCESS);
}
