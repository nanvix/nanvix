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
PUBLIC int sys_semclose(int idx)
{

	if(!SEM_IS_VALID(idx))
	{
		/* Not a valid semaphore */
		curr_proc->errno = EINVAL;
		return SEM_FAILED;
	}
	else
	{
		semtable[idx].nbproc--;
		
		/*
		 * 	The semaphore is no longer accessible when 0 process use it
		 * 	and only if it has been unlinked once 
		 */
		if(semtable[idx].nbproc==0 && (semtable[idx].state&UNLINKED) )
		{
			kprintf("deleting semaphore in table\n");
			freesem(idx);
		}

	}

	return 0;	/* successful completion */

}