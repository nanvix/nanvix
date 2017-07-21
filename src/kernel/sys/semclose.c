#include <sys/sem.h>
#include <errno.h>

/**
 * @brief Calling process closes the semaphore
 *		 
 * @param idx The semaphore index in semtable
 *
 * @returns Returns 0 in case of successful completion
 *					corresponding error code otherwise
 */
PUBLIC int sys_semclose(int idx)
{
	struct inode *seminode;
	int i;

	if (!SEM_IS_VALID(idx))
		return (-EINVAL);

	seminode = inode_get(semtable[idx].dev, semtable[idx].num);

	if (seminode == NULL)
		return 0;			/* 0 is returned because, it would mean the semaphore is in user table but the inode doesn't exist anymore */

	inode_put(seminode);

	for (i = 0; i < PROC_MAX; i++)
	{
		/* Removing the proc pid in the semaphore procs table */
		if (semtable[idx].currprocs[i] == curr_proc->pid)
		{
			semtable[idx].currprocs[i] = 0;
			break;
		}
	}

	/*  
	 *  The semaphore is in user table but the PID isn't on 
	 *  the semaphore -> sem should be freed. It can happen
	 *  if the inode is removed outside of system calls
	 */
	if (i == PROC_MAX)
		return 0;			

	char found;
	found = 0;
	i++;

	/* Searching for multiple openings */
	while (i < PROC_MAX)
	{
		if (semtable[idx].currprocs[i] == curr_proc->pid)
		{
			found = 1;
			break;
		}
		i++;
	}

	if (found)
	{
		inode_put(seminode);
		inode_unlock(seminode);
		
		/* 
		 *  1 is returned to userland so it is known that the process
		 *  has opened the semaphore multiple times.
		 */
		return 1;
	}
	else
	{
		if (seminode->count == 1 && seminode->nlinks == 0)
			freesem(&semtable[idx]);
		
		inode_put(seminode);
		inode_unlock(seminode);
		return 0;
	}
}
