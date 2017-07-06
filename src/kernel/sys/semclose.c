#include <sys/sem.h>
#include <errno.h>

/**
 * @brief close a semaphore for a given process
 *		 
 * @param num Semaphore inode number 
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 */
PUBLIC int sys_semclose(int idx)
{
	struct inode *seminode;
	int i;

	seminode = inode_name(semtable[idx].name);

	if (seminode == NULL)
		return (-EINVAL);

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

	if (i == PROC_MAX)
		return -1;

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
		return 1;
	}
	else
	{
		if (seminode->count == 1 && seminode->nlinks == 0)
		{		
			remove_semaphore(semtable[idx].name);
			freesem(&semtable[idx]);
		}	
		
		inode_put(seminode);
		inode_unlock(seminode);
		return 0;
	}
}
