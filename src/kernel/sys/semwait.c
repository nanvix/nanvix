#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>

/**
 * @brief	Waits on a semaphore
 *		 
 * @param	idx The index of the semaphore in the
 *			semtable
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 */
PUBLIC int sys_semwait(int idx)
{

	if (!SEM_IS_VALID(idx))
	{
		curr_proc->errno = EINVAL;
		return SEM_FAILED;
	}


	while (semtable[idx].value<=0)
	{
		sleep(semwaiters,curr_proc->priority);
	}

	semtable[idx].value--;

	return 0;	/* Successful completion */
}