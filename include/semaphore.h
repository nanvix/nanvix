#define SEM_FAILED -1

#ifndef _ASM_FILE_

	/* User semaphore */
	typedef struct sem_t
	{

		int idx;

	}sem_t;

	sem_t* sem_open(char* name, int oflag, ...);
	int    sem_close(sem_t* sem);
	int    sem_unlink(const char *name);

	/*
	int    sem_destroy(sem_t *);
	int    sem_getvalue(sem_t *restrict, int *restrict);
	int    sem_init(sem_t *, int, unsigned);
	int    sem_post(sem_t *);
	int    sem_timedwait(sem_t *restrict, const struct timespec *restrict);
	int    sem_trywait(sem_t *);
	int    sem_wait(sem_t *);
	*/
#endif


