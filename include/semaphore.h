#include <sys/types.h>








#ifndef _ASM_FILE_

/* users are manipulating a fake semaphore */

	/* user semaphore */
	typedef struct sem_t{

		int idx;

	}sem_t;



sem_t* sem_open(char* name, int oflag, ...);


// int    sem_close(sem_t *);
// int    sem_destroy(sem_t *);
// int    sem_getvalue(sem_t *restrict, int *restrict);
// int    sem_init(sem_t *, int, unsigned);
// int    sem_post(sem_t *);
// int    sem_timedwait(sem_t *restrict, const struct timespec *restrict);
// int    sem_trywait(sem_t *);
// int    sem_unlink(const char *);
// int    sem_wait(sem_t *);

#endif