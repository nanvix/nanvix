#define SEM_FAILED -1

#ifndef _ASM_FILE_

	/* User semaphore */
	typedef struct sem_t
	{
		
		int idx;

	}sem_t;

	sem_t* sem_open(const char* name, int oflag, ...);
	int    sem_close(sem_t* sem);
	int    sem_unlink(const char *name);
	int    sem_post(sem_t *sem);
	int    sem_wait(sem_t *sem);

#endif


