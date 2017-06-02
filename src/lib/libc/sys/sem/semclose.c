#include <nanvix/syscall.h>

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

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_semclose),
		  "b" (sem->idx)
	);

	/* 	
	 *	making the semaphore unavailable for
	 * 	the calling process by setting
	 *	error index : -1
	 */
	sem->idx=-1;
	
	return (ret);
}