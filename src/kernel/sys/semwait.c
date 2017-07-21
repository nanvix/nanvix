#include <sys/sem.h>
#include <errno.h>

/**
 * @brief Waits on a semaphore
 *		 
 * @param num The inode number of the semaphore
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
	int i;
	
	if (!SEM_IS_VALID(idx))
		return (-EINVAL);

	for (i = 0; i < PROC_MAX; i++)
	{
		if (semtable[idx].currprocs[i] == curr_proc->pid)
			break;
	}

	if (i == PROC_MAX)
		return -1;

	seminode = inode_get(semtable[idx].dev, semtable[idx].num);

	if (seminode == NULL)
		return (-EINVAL);

	inode_unlock(seminode);

	while (semtable[idx].value <= 0)
	{
		sleep(semtable[idx].semwaiters,curr_proc->priority);
		
		/* Awaken by a signal. */
		if (issig())
		{
			inode_put(seminode);
			return (-EINTR);
		}
	}

	semtable[idx].value--;
	inode_put(seminode);

	return (0);
}