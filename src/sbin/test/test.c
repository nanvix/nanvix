/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
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

#include <assert.h>
#include <nanvix/config.h>
#include <sys/times.h>
#include <sys/wait.h>
#include <sys/sem.h>
#include <stdio.h>
#include <signal.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>
#include <fcntl.h>
#include <limits.h>
#include <semaphore.h>
#include <errno.h>
#include <tmpthread.h>

/* Test flags. */
#define VERBOSE  (1 << 10)

/* Test flags. */
static unsigned flags = VERBOSE;

/*============================================================================*
 *                             Synthetic Works                                *
 *============================================================================*/

/**
 * @brief Performs some dummy CPU-intensive computation.
 */
static void work_cpu(void)
{
	int c;
	
	c = 0;
		
	/* Perform some computation. */
	for (int i = 0; i < 4096; i++)
	{
		int a = 1 + i;
		for (int b = 2; b < i; b++)
		{
			if ((i%b) == 0)
				a += b;
		}
		c += a;
	}
}

/**
 * @brief Performs some dummy FPU-intensive computation.
 */
static void work_fpu(void)
{
	const int n = 16; /* Matrix size.    */
	float a[16][16];  /* First operand.  */
	float b[16][16];  /* Second operand. */
	float c[16][16];  /* Result.         */
	
	/* Initialize matrices. */
	for (int i = 0; i < n; i++)
	{
		for (int j = 0; j < n; j++)
		{
			a[i][j] = 1.0;
			b[i][j] = 2.0;
			c[i][j] = 0.0;
		}
	}
	
	/* Perform matrix multiplication. */
	for (int i = 0; i < n; i++)
	{
		for (int j = 0; j < n; j++)
		{
			for (int k = 0; k < n; k++)
				c[i][j] += a[i][k]*b[k][i];
		}
	}
}

/**
 * @brief Performs some dummy IO-intensive computation.
 */
static void work_io(void)
{
	int fd;            /* File descriptor. */
	char buffer[2048]; /* Buffer.          */
	
	/* Open hdd. */
	fd = open("/dev/hdd", O_RDONLY);
	if (fd < 0)
		_exit(EXIT_FAILURE);
	
	/* Read data. */
	for (size_t i = 0; i < MEMORY_SIZE; i += sizeof(buffer))
	{
		if (read(fd, buffer, sizeof(buffer)) < 0)
			_exit(EXIT_FAILURE);
	}
	
	/* House keeping. */
	close(fd);
}

/*============================================================================*
 *                             Paging System Tests                            *
 *============================================================================*/

/**
 * @brief Demand zero test module.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
static int demand_zero_test(void)
{
	const size_t size = PROC_SIZE_MAX/2; /* Buffer size.        */
	char *buffer;                        /* Buffer.             */
	struct tms timing;                   /* Timing information. */
	clock_t t0, t1;                      /* Elapsed times.      */

	buffer = malloc(size);
	assert(buffer != NULL);

	t0 = times(&timing);
	
	/* Fill buffer. */
	for (size_t i = 0; i < size; i++)
		buffer[i] = 1;
	
	t1 = times(&timing);
	
	/* Print timing statistics. */
	if (flags & VERBOSE)
		printf("  Elapsed: %d\n", t1 - t0);
	
	/* House keeping. */
	free(buffer);

	return (0);
}

/**
 * @brief Dummy function used by stack_grow_test().
 *
 * @param i Dummy variable.
 *
 * @returns Dummy value.
 */
static int foobar(int i)
{
	return ((i == 0) ? 0 : foobar(i - 1) + 1);
}

/**
 * @brief Stack grow test module.
 *
 * @returns Zero if passed on test, and non-zero otherwise.
 */
static int stack_grow_test(void)
{
	struct tms timing;     /* Timing information. */
	clock_t t0, t1;        /* Elapsed times.      */
	const int size = 1024; /* Recursion size.     */

	t0 = times(&timing);
	
	foobar(size);
	
	t1 = times(&timing);
	
	/* Print timing statistics. */
	if (flags & VERBOSE)
		printf("  Elapsed: %d\n", t1 - t0);

	return (0);
}

/*============================================================================*
 *                                  io_test                                   *
 *============================================================================*/

/**
 * @brief I/O testing module.
 * 
 * @details Reads sequentially the contents of the hard disk
            to a in-memory buffer.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
static int io_test(void)
{
	int fd;            /* File descriptor.    */
	struct tms timing; /* Timing information. */
	clock_t t0, t1;    /* Elapsed times.      */
	char *buffer;      /* Buffer.             */
	
	/* Allocate buffer. */
	buffer = malloc(MEMORY_SIZE);
	if (buffer == NULL)
		exit(EXIT_FAILURE);
	
	/* Open hdd. */
	fd = open("/dev/hdd", O_RDONLY);
	if (fd < 0)
		exit(EXIT_FAILURE);
	
	t0 = times(&timing);
	
	/* Read hdd. */
	if (read(fd, buffer, MEMORY_SIZE) != MEMORY_SIZE)
		exit(EXIT_FAILURE);
	
	t1 = times(&timing);
	
	/* House keeping. */
	free(buffer);
	close(fd);
	
	/* Print timing statistics. */
	if (flags & VERBOSE)
		printf("  Elapsed: %d\n", t1 - t0);
	
	return (0);
}

/*============================================================================*
 *                                sched_test                                  *
 *============================================================================*/

/**
 * @brief Scheduling test 0.
 * 
 * @details Spawns two processes and tests wait() system call.
 * 
 * @return Zero if passed on test, and non-zero otherwise.
 */
static int sched_test0(void)
{
	pid_t pid;
	
	pid = fork();
	
	/* Failed to fork(). */
	if (pid < 0)
		return (-1);
	
	/* Child process. */
	else if (pid == 0)
	{
		work_cpu();
		_exit(EXIT_SUCCESS);
	}
	
	wait(NULL);
	
	return (0);
}

/**
 * @brief Scheduling test 1.
 * 
 * @details Forces the priority inversion problem, to check how well the system
 *          performs on dynamic priorities.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
static int sched_test1(void)
{
	pid_t pid;
		
	pid = fork();
	
	/* Failed to fork(). */
	if (pid < 0)
		return (-1);
	
	/* Parent process. */
	else if (pid > 0)
	{
		nice(-2*NZERO);
		work_cpu();
	}
	
	/* Child process. */
	else
	{
		nice(2*NZERO);
		work_io();
		_exit(EXIT_SUCCESS);
	}
		
	wait(NULL);
	
	return (0);
}

/**
 * @brief Scheduling test 1.
 * 
 * @details Spawns several processes and stresses the scheduler.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
static int sched_test2(void)
{
	pid_t pid[4];
	
	for (int i = 0; i < 4; i++)
	{
		pid[i] = fork();
	
		/* Failed to fork(). */
		if (pid[i] < 0)
			return (-1);
		
		/* Child process. */
		else if (pid[i] == 0)
		{
			if (i & 1)
			{
				nice(2*NZERO);
				work_cpu();
				_exit(EXIT_SUCCESS);
			}
			
			else
			{	
				nice(-2*NZERO);
				pause();
				_exit(EXIT_SUCCESS);
			}
		}
	}
	
	for (int i = 0; i < 4; i++)
	{
		if (i & 1)
			wait(NULL);
			
		else
		{	
			kill(pid[i], SIGCONT);
			wait(NULL);
		}
	}
	
	return (0);
}
/**
 * @brief Scheduling test 3.
 * 
 * @details Spawns several processes and stresses the scheduler.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
*/
static int sched_test3(void)
{
	pid_t child;
	pid_t father;

	father = getpid();

	fork();
	fork();
	fork();
	fork();

	/* Wait for children. */
	while ((child = wait(NULL)) >= 0)
		/* noop. */;

	/* Die. */
	if (getpid() != father)
		_exit(EXIT_SUCCESS);

	return (0);
}
/*============================================================================*
 *                             Semaphores Test                                *
 *============================================================================*/

/**
 * @brief Produces stuff.
 *
 * @param sem  Accounts for empty slots on a shared buffer.
 * @param fill Accounts for used slots on a shared buffer.
 */
static void consumer(sem_t *sem, sem_t *empty, int n)
{
	for (int j = 0; j < n; j++)
	{
		sem_wait(empty);
		work_cpu();
		sem_post(sem);
	}
}

/**
 * @brief Consumes stuff.
 *
 * @param sem  Accounts for empty slots on a shared buffer.
 * @param fill Accounts for used slots on a shared buffer.
 */
static void producer(sem_t *sem, sem_t *empty, int n)
{
	for (int j = 0; j < n; j++)
	{
		sem_wait(sem);
		work_cpu();
		sem_post(empty);
	}
}

/*  
 * @brief Producer consumer test.
 */
static int sem_producer_consumer_test(void)
{
	int n;               /* Number of items.       */
	mode_t mode;         /* Semaphore access mode. */
	sem_t *empty, *full; /* Semaphores.            */

	/* Named semaphores. */
	const char *sem1 = "/home/mysem/sem1";
	const char *sem2 = "/home/mysem/sem2";

	n = 10;
	mode = 0644;

	full = sem_open(sem2, O_CREAT, mode, 0);
	empty = sem_open(sem1, O_CREAT, mode, n);

	if ((empty == NULL) || (full == NULL))
	{	
		printf("cannot create semaphores\n");
		exit(EXIT_FAILURE);	
	}

	/* Child. */
	if (fork() == 0)
	{
		full = sem_open(sem2, O_RDWR);
		empty = sem_open(sem1, O_RDWR);

		if ((empty == NULL) || (full == NULL))
		{	
			printf("cannot open semaphores\n");
			exit(EXIT_FAILURE);	
		}

		producer(empty, full, n);

		sem_close(full);
		sem_close(empty);

		exit (EXIT_SUCCESS);
	}

	/* Father. */
	else
	{
		consumer(empty, full, n);

		sem_close(full);
		sem_close(empty);
		
		sem_unlink(sem2);
		sem_unlink(sem1);
	}

	return (0);
}

static int sem_test_open_close(void) 
{ 
	/* Child. */
	if (fork() == 0)
	{ 
		/* We don't unlink semc2 */ 
		sem_t *semc1, *semc3;
		semc1 = sem_open("/home/mysem/sem1",O_CREAT,0777,0);
		semc3 = sem_open("/home/mysem/sem3",O_CREAT,0777,0); 

		sem_wait(semc1);
		work_cpu();
		sem_post(semc3);

		sem_unlink("/home/mysem/sem1");
		sem_wait(semc1);

		sem_close(semc3);
		sem_close(semc1);
		sem_unlink("/home/mysem/sem3");

		/* Operations on invalid semaphores */
		sem_wait(semc1);
		sem_wait(semc3);
		sem_post(semc1);
		sem_post(semc3);

		exit (EXIT_SUCCESS);
	} 
	else
	{ 
		/* father */ 
		sem_t *semf1, *semf2, *semf3, *semf4; 
		semf1 = sem_open("/home/mysem/sem1",O_CREAT,0777,0);
		/* Opening the same semaphore : the same address should be returned */ 
		semf2 = sem_open("/home/mysem/sem1",O_CREAT,0777,0);
		semf3 = sem_open("/home/mysem/sem3",O_CREAT,0777,0); 
		semf4 = sem_open("/home/mysem/sem4",O_CREAT,0777,0); 

		sem_post(semf1);
		work_cpu();
		sem_wait(semf3);
		sem_unlink("/home/mysem/sem1");

		work_cpu();
		sem_post(semf1);
		sem_close(semf3);

		/* sem4 multiple closed */
		sem_close(semf4);
		sem_close(semf4); /* no effect */ 
		sem_close(semf4); /* no effect */
		
		sem_unlink("/home/mysem/sem4"); /* this will delete sem4 because ref count == 0 */
		
		/* opening a new semaphore called sem4 */
		semf4 = sem_open("/home/mysem/sem4",O_CREAT,0777,0); 

		/* Unlinking before closing */
		sem_unlink("/home/mysem/sem4"); /* this will delete sem4 because ref count == 0 */

		sem_close(semf4);

		semf4 = sem_open("/home/mysem/sem4",O_CREAT,0777,0); /* This will use another kernel slot : sem4 has been unlinked */

		sem_close(semf1);
		sem_close(semf2);
		sem_close(semf3);
		
		sem_unlink("/home/mysem/sem2");
		sem_unlink("/home/mysem/sem4");
	} 
 
	return (0); 
} 

/*============================================================================*
 *                                FPU test                                    *
 *============================================================================*/

/**
 * @brief FPU testing module.
 * 
 * @details Performs a floating point operation, while trying to
            mess up the stack from another process.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
int fpu_test(void)
{
#ifdef i386
	pid_t pid;     /* Child process ID.     */
	float a = 6.7; /* First dummy operand.  */
	float b = 1.2; /* Second dummy operand. */
	float result;  /* Result.               */
	
	/* Perform a/b */
	__asm__ volatile(
		"flds %1;"
		"flds %0;"
		"fdivrp %%st,%%st(1);"
		: /* noop. */
		: "m" (b), "m" (a)
	);

	pid = fork();
	
	/* Failed to fork(). */
	if (pid < 0)
		return (-1);
	
	/*
	 * Child process tries
	 * to mess up the stack.
	 */
	else if (pid == 0)
	{
		work_fpu();
		_exit(EXIT_SUCCESS);
	}
	
	wait(NULL);

	/* But it's only in your context,
	 * so nothing changed for father process.
	 */
	__asm__ volatile(
		"fstps %0;"
		: "=m" (result)
	);

	/* 0x40b2aaaa = 6.7/1.2 = 5.5833.. */
	return (result == 0x40b2aaaa);
#elif or1k
	((void)work_fpu);
	return (1);
#endif
}

/*============================================================================*
 *                           Memory Violation                                 *
 *============================================================================*/

/*  
 * @brief Forces a memory violation.
 */
static int test_mem0(void)
{
	/* Child. */
	if (fork() == 0)
	{
		int *i;
		i = NULL;
		printf("%d \n",*i);
	}
	/* Parent. */
	else
	{
		/* noop */;
		exit(EXIT_SUCCESS);
	}

	return (0);
}

/*============================================================================*
 *                           Thread Test                                      *
 *============================================================================*/

/*
 * @brief Thread test.
 */
static int thread_test(void)
{
	return (pthread_dummy());
}

/*============================================================================*
 *                                   main                                     *
 *============================================================================*/

/**
 * @brief Prints program usage and exits.
 * 
 * @details Prints the program usage and exists gracefully.
 */
static void usage(void)
{
	printf("Usage: test [options]\n\n");
	printf("Brief: Performs regression tests on Nanvix.\n\n");
	printf("Options:\n");
	printf("  fpu     Floating Point Unit Test\n");
	printf("  io      I/O Test\n");
	printf("  ipc     Interprocess Communication Test\n");
	printf("  paging  Paging System Test\n");
	printf("  stack   Stack growth Test\n");
	printf("  sched   Scheduling Test\n");
	printf("  sem	  Semaphore Tests\n");
	printf("  mem	  Memory Violation Tests\n");
	printf("  thread  Thread Tests\n");

	exit(EXIT_SUCCESS);
}

/**
 * @brief System testing utility.
 */
int main(int argc, char **argv)
{
	/* Missing arguments? */
	if (argc <= 1)
		usage();

	for (int i = 1; i < argc; i++)
	{
		/* I/O test. */
		if (!strcmp(argv[i], "io"))
		{
			printf("I/O Test\n");
			printf("  Result:             [%s]\n", 
				(!io_test()) ? "PASSED" : "FAILED");
		}
		
		/* Paging system test. */
		else if (!strcmp(argv[i], "paging"))
		{
			printf("Demand Zero Test\n");
			printf("  Result:             [%s]\n",
				(!demand_zero_test()) ? "PASSED" : "FAILED");
		}

		/* Stack growth test. */
		else if (!strcmp(argv[i], "stack"))
		{
			printf("Stack Grow Test\n");
			printf("  Result:             [%s]\n",
				(!stack_grow_test()) ? "PASSED" : "FAILED");
		}
		
		/* Scheduling test. */
		else if (!strcmp(argv[i], "sched"))
		{
			printf("Scheduling Tests\n");
			printf("  waiting for child  [%s]\n",
				(!sched_test0()) ? "PASSED" : "FAILED");
			printf("  dynamic priorities [%s]\n",
				(!sched_test1()) ? "PASSED" : "FAILED");
			printf("  scheduler stress   [%s]\n",
				(!sched_test2() && !sched_test3()) ? "PASSED" : "FAILED");	
		}

		/* FPU test. */
		else if (!strcmp(argv[i], "fpu"))
		{
			printf("Float Point Unit Test\n");
			printf("  Result [%s]\n",
				(!fpu_test()) ? "PASSED" : "FAILED");
		}

		/* Semaphore tests. */
		else if (!strcmp(argv[i], "sem"))
		{
			printf("Semaphore Tests\n");
			printf("  open and close    [%s]\n",
				(!sem_test_open_close()) ? "PASSED" : "FAILED");
			printf("  producer consumer [%s]\n",
				(!sem_producer_consumer_test()) ? "PASSED" : "FAILED");
		}

		/* Memory tests. */
		else if (!strcmp(argv[i], "mem"))
		{
			printf("Memory Violation Tests\n");
			printf("  null pointer      [%s]\n",
				(!test_mem0()) ? "PASSED" : "FAILED");
		}

		/* Thread tests. */
		else if (!strcmp(argv[i], "thread"))
		{
			printf("Thread Tests\n");
			printf("  Result [%s]\n",
				(!thread_test()) ? "PASSED" : "FAILED");
		}

		/* Wrong usage. */
		else
			usage();
	}
	
	return (EXIT_SUCCESS);
}
