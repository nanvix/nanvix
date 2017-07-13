#include <nanvix/syscall.h>
#include <errno.h>

/**
 * @brief	Waits on a semaphore
 *		 
 * @param	sem The blocking semaphore
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 */
int sem_wait(sem_t *sem)
{	
	int ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_semwait),
		  "b" (sem->semid)
	);

	if (ret < 0)
	{
		errno = ret;
		return (-1);
	}

	return (0);
}