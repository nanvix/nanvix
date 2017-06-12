#include <nanvix/syscall.h>
#include <stdlib.h>
#include <stdio.h>

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
	int ret;

	if(sem == NULL)
		return (-1);

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_semclose),
		  "b" (sem->idx)
	);

	if(ret == 0)
		free(sem);
	
	return (ret);
}	