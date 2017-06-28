#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>

/**
 * @brief close a semaphore for a given process
 *		 
 * @param	num		semaphore inode number 
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 */
PUBLIC int sys_semclose(int idx)
{
	struct inode *seminode;
	int i;

	if (semtable[idx].name[0] == '\0')
		return (-EINVAL);

	for (i = 0; i < PROC_MAX; i++)
	{
		/* Removing the proc pid in the semaphore procs table */
		if (semtable[idx].currprocs[i] == curr_proc->pid)
		{
			semtable[idx].currprocs[i] = -1;
			break;
		}
	}

	if (i > PROC_MAX)
	{
		kprintf("Attempting to close a non-opened semaphore");
		return -1;
	}

	semtable[idx].nbproc--;

	if (semtable[idx].nbproc == 0 && (semtable[idx].state & UNLINKED)>0)
	{		
		remove_semaphore (semtable[idx].name);
		semtable[idx].name[0] = '\0';
	}	

	seminode = semtable[idx].seminode;
	inode_put(seminode);

	return 0;
}
