#include <sys/sem.h>
#include <errno.h>

#include <nanvix/klib.h>
/**
 * @brief Closes a semaphore for a given process
 *		 
 * @param idx Semaphore index (in semaphore
 *			  table to close
 *
 * @returns 0 in case of successful completion
 *			error code otherwise
 */
PUBLIC int sys_sempost(int idx)
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
		return (-1);

	semtable[idx].value++;
	
	if (semtable[idx].value>0)
		wakeup(semtable[idx].semwaiters);

	inode_put(seminode);
	inode_unlock(seminode);

	return (0);
}
