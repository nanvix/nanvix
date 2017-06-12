#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>

/**
 * @brief close a semaphore for a given process
 *		 
 * @param	idx		semaphore index (in semaphore
 *					table to close
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 */
PUBLIC int sys_sempost(int idx)
{
	/* Not a valid semaphore */
	if (!SEM_IS_VALID(idx))
		return (-EINVAL);

	semtable[idx].value++;

	if (semtable[idx].value>0)
	{
		wakeup(semwaiters);
	}
	return (0);
}
