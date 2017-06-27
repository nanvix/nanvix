#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>

/**
 * @brief	Waits on a semaphore
 *		 
 * @param	num The inode number of the semaphore
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 *
 * TODO for error detection :
 *			EDEADLK : A deadlock condition was detected.
 *			EINTR : A signal interrupted this function.
 */
PUBLIC int sys_semwait(int idx)
{
	struct inode *seminode;

	seminode = semtable[idx].seminode;

	if (seminode == NULL)
		return (-EINVAL);

	while (semtable[idx].value <= 0)
	{
		sleep(semtable[idx].semwaiters,curr_proc->priority);
	}

	semtable[idx].value--;

	return (0);
}