#include <nanvix/syscall.h>

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

	if (sem == NULL)
		return (-1);

	for (int i = 0; i < OPEN_MAX; i++)
		if (usem[i] == sem)
			break;	

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_semwait),
		  "b" (sem->semid)
	);

	return (ret);
}