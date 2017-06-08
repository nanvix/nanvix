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
PUBLIC int sys_sempost(int idx)
{

	if(!SEM_IS_VALID(idx))
	{
		/* Not a valid semaphore */
		curr_proc->errno = EINVAL;
		return SEM_FAILED;
	}
	else
	{
		semtable[idx].value++;

		if (semtable[idx].value>0)
		{
			wakeup(semwaiters);
		}
	}

	kprintf("%s Value : %d\n",semtable[idx].name,semtable[idx].value);

	return 0;	/* successful completion */
}