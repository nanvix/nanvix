#include <nanvix/fs.h>
#include <nanvix/smp.h>
#include <sys/sem.h>
#include <errno.h>

/**
 * @brief Wait action : consume a ressource if available
 *						sleeps otherwise.
 *		 
 * @param idx The semaphore index in semtable
 *
 * @returns 0 in case of successful completion
 *			Corresponding error code otherwise.
 *
 * TODO for error detection :
 *			EDEADLK : A deadlock condition was detected.
 *					  Seems not implemented on other OS
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

	/* Semaphore not opened by the process */
	if (i == PROC_MAX)
		return (-EINVAL);

	seminode = inode_get(semtable[idx].dev, semtable[idx].num);

	if (seminode == NULL)
		return (-EINVAL);

	inode_unlock(seminode);

	while (semtable[idx].value <= 0)
	{
		sleep(semtable[idx].semwaiters, cpus[curr_core].curr_thread->priority);
		
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