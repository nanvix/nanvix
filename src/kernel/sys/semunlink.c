#include <fcntl.h>
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>

/**
 * @brief	Unlinks a semaphore for future deletion
 *		 
 * @param	name Semaphore name
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 */
PUBLIC int sys_semunlink(const char *name)
{
	int idx;

	idx = existance(name);

	if (idx==-1)
	{
		/* Semaphore doesn't exists */
		curr_proc->errno = ENOENT;
		return SEM_FAILED;
	}
	else
	{
		/* Checking WRITE permission */
		if (!permission(semtable[idx].state, semtable[idx].uid, semtable[idx].gid, curr_proc, MAY_WRITE, 0))
		{
			curr_proc->errno = EACCES;
			return SEM_FAILED;
		}

		kprintf("Unlinking %s",semtable[idx].name);
		semtable[idx].state|=UNLINKED;
	}

	return 0;	/* Successful completion */
}