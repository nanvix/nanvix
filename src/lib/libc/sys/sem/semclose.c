#include <nanvix/syscall.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>

/**
 * 	@brief closes a semaphore for calling process
 *
 *	@param sem The semaphore to close
 *
 *	@returns 0 in case of successful completion
 *			 SEM_FAILED otherwise
 */
int sem_close(sem_t* sem)
{	
	int ret, i;

	if (sem == NULL)
		return (-1);

	for (i = 0; i < OPEN_MAX; i++)
		if (usem[i] == sem)
			break;

	if (i == OPEN_MAX)
		return (-1);

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_semclose),
		  "b" (sem->semid)
	);

	if (ret == (-1))
	{
		errno = ret;
		return (-1);
	}

	if (ret == 0)
	{
		usem[sem->semid] = NULL;
		free(sem);
	}

	/* if ret == 1 : do not free because multiple opening */

	return (0);
}	