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
	seminode = inode_name(semtable[idx].name);

	if (seminode == NULL)
		return (-EINVAL);

	for (i = 0; i < PROC_MAX; i++)
	{
		/* Removing the proc pid in the semaphore procs table */
		if (semtable[idx].currprocs[i] == curr_proc->pid)
			break;
	}

	if (i == PROC_MAX)
		return -1;

	inode_unlock(seminode);

	while (semtable[idx].value <= 0)
		sleep(semtable[idx].semwaiters,curr_proc->priority);

	semtable[idx].value--;
	inode_put(seminode);

	return (0);
}