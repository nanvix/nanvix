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



/* Test flags. */
#define VERBOSE  (1 << 10)

/* Test flags. */
static unsigned flags = VERBOSE;

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

/*============================================================================*
 *                             Semaphores Test                                *
 *============================================================================*/


/**
 * @brief Puts an item in a buffer.
 */
#define PUT_ITEM(a, b)                                \
{                                                     \
	assert(lseek((a), 0, SEEK_SET) != -1);            \
	assert(write((a), &(b), sizeof(b)) == sizeof(b)); \
}                                                     \

/**
 * @brief Gets an item from a buffer.
 */
#define GET_ITEM(a, b)                               \
{                                                    \
	assert(lseek((a), 0, SEEK_SET) != -1);           \
	assert(read((a), &(b), sizeof(b)) == sizeof(b)); \
}                  									 \
	
/* Simulate a bit of work -> small wait : ~2.5 seconds */
void work(void)
{
	unsigned long i;
	float x;

	x=1.3232;

	for( i = 0 ; i<99999999;i++)
	{
		x=x*0.451;
	}

	if(x)
	{
		printf("\n");
	}
}


void producer(int nbprod, int limit)
{
	sem_t* sem, *semlim;
	sem = sem_open("/home/mysem/ressources", O_CREAT, 0777,0);
	semlim = sem_open("/home/mysem/limite", O_CREAT, 0777,limit);

	for (int j = 0; j < nbprod; j++)
	{
		sem_wait(semlim);
		printf("producer : start producing\n");
		work();
		printf("producer : has produced\n");
		sem_post(sem);
	}
}

void consumer(int nbcons, int limit)
{
	sem_t *sem, *semlim;
	sem = sem_open("/home/mysem/ressources", O_CREAT, 0777,0);
	semlim = sem_open("/home/mysem/limite", O_CREAT, 0777,limit);

	for (int j = 0; j < nbcons; j++)
	{
		printf("cons : waiting for ressource\n");
		sem_wait(sem);
		printf("cons : ressources has been produced\n");
		work();
		printf("cons : ressources consumed\n");
		sem_post(semlim);
	}
}

/*  
 *	Producer consumer
 *  The buffer has a size of 3
 *  The producer will produce 10 items
 */
static int sem_test(void)
{
	if (fork() == 0)
	{
		/* child */
		printf("child \n");
		producer(10,3);
	}
	else
	{
		/* father */
		printf("father \n");
		consumer(10,3);
	}

	return (0);
}

static int sem_test_open_close(void) 
{ 
	if (fork() == 0)
	{ 
		/* We don't unlink semc2 */ 
		/* child */ 
		printf("Child\n"); 
		sem_t *semc1, *semc3;
		semc1 = sem_open("/home/mysem/sem1",O_CREAT,0777,0);
		// semc2 = sem_open("/home/mysem/sem2",O_CREAT,0777,0); 
		semc3 = sem_open("/home/mysem/sem3",O_CREAT,0777,0); 

		printf("sem1 id : %d\n",semc1->semid);
		printf("sem3 id : %d\n",semc3->semid);

		printf("Child blocked on sem1\n");
		/* Locking child */
		sem_wait(semc1);

		/* Waiting */
		work();

		printf("Child unlocking sem3\n");
		/* Unlocking father */
		sem_post(semc3);

		/* Unlinking sem1 : it shoud still be usable but not be in file system */
		if (sem_unlink("/home/mysem/sem1"))
		{
			printf("Child : sem1 has already been unlinked\n");
		}
		else
		{
			printf("Child : sem1 has been unlinked\n");
		}


		printf("Child blocked on sem1\n");
		sem_wait(semc1);

		printf("%d %d\n",semc3,semc1);

		sem_close(semc3);
		sem_close(semc1);

		printf("%d %d\n",semc3,semc1);
		sem_unlink("/home/mysem/sem3");

		/* Operations on invalid semaphores */
		sem_wait(semc1);
		sem_wait(semc3);
		sem_post(semc1);
		sem_post(semc3);
	} 
	else
	{ 
		/* father */ 
		printf("Father \n"); 
		sem_t *semf1, *semf2, *semf3, *semf4; 
		semf1 = sem_open("/home/mysem/sem1",O_CREAT,0777,0);
		/* Opening the same semaphore : the same address should be returned */ 
		semf2 = sem_open("/home/mysem/sem1",O_CREAT,0777,0);
		semf3 = sem_open("/home/mysem/sem3",O_CREAT,0777,0); 
		semf4 = sem_open("/home/mysem/sem4",O_CREAT,0777,0); 

		if(semf1 == semf2)
		{
			printf("semf1 == semf2\n");
		}

		printf("sem1 id : %d\n",semf1->semid);
		printf("sem3 id : %d\n",semf3->semid);
		printf("sem4 id : %d\n",semf4->semid);

		printf("Father unlocking sem1\n");
		/* Unlocking child */
		sem_post(semf1);
		
		/* Waiting */
		work();

		printf("Father blocked on sem3\n");
		/* Unlocking father */
		sem_wait(semf3);

		/*  Unlinking sem1 : it shoud still be usable but not be in file system 
		 *  unlinked both in child and father -> so it's ensured to be unlinked 
		 *  the second unlink should have no effect because the semaphore doesn't
		 *  exist in the file system anymore
		 */
		if (sem_unlink("/home/mysem/sem1"))
		{
			printf("Father : sem1 has already been unlinked\n");
		}
		else
		{
			printf("Father : sem1 has been unlinked\n");
		}

		work();

		printf("Father unlocking sem1\n");
		/* Unlocking child */
		sem_post(semf1);

		/* sem1 has not been closed. */

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
		/* Opening sem4 and letting it in alive */
		printf("sem4 id : %d\n",semf4->semid);

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
			a[i][j] = 2.0;
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
 * @brief FPU testing module.
 * 
 * @details Performs a floating point operation, while trying to
            mess up the stack from another process.
 * 
 * @returns Zero if passed on test, and non-zero otherwise.
 */
int fpu_test(void)
{
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
	printf("  fpu   		Floating Point Unit Test\n");
	printf("  io    		I/O Test\n");
	printf("  ipc    		Interprocess Communication Test\n");
	printf("  Paging 		Paging System Test\n");
	printf("  stack  		Stack growth Test\n");
	printf("  sched  		Scheduling Test\n");
	printf("  se 			Open/Close Semaphore Test\n");
	printf("  prodcons		Producer/Consumer Semaphore Test\n");


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
				(!sched_test2()) ? "PASSED" : "FAILED");
		}
		

		/* FPU test. */
		else if (!strcmp(argv[i], "fpu"))
		{
			printf("Float Point Unit Test\n");
			printf("  Result [%s]\n",
				(!fpu_test()) ? "PASSED" : "FAILED");
		}

		/* Semaphore test. */
		else if (!strcmp(argv[i], "se"))
		{
			printf("Semaphore open/close Test\n");
			printf("  Result [%s]\n",
				(!sem_test_open_close2()) ? "PASSED" : "FAILED");
		}

		/* Semaphore test. */
		else if (!strcmp(argv[i], "prodcons"))
		{
			printf("Producer consummer Test\n");
			printf("  Result [%s]\n",
				(!sem_test()) ? "PASSED" : "FAILED");
		}
			/* Semaphore test. */
		else if (!strcmp(argv[i], "prodcons33"))
		{
			printf("Producer consummer Test\n");
			printf("  Result [%s]\n",
				(!sem_test_open_close()) ? "PASSED" : "FAILED");
		}
	
		/* Wrong usage. */
		else
			usage();
	}
	
	return (EXIT_SUCCESS);
}
