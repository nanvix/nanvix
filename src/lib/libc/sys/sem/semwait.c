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