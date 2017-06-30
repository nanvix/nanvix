#include <nanvix/syscall.h>
#include <stdlib.h>

/**
 * 	@brief closes a semaphore for calling process
 *
 *	@param sem The semaphore to close
 *
 *	@returns 0 in case of successful completion
 *			 SEM_FAILED otherwise
 */
int sem_post(sem_t* sem)
{	
	int ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_sempost),
		  "b" (sem->semid)
	);

	return (ret);
}