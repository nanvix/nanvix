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
		return (-ENOENT);

	/* Checking WRITE permission */
	if (!permission(semtable[idx].state, semtable[idx].uid, semtable[idx].gid, curr_proc, MAY_WRITE, 0))
		return (-EACCES);

	if (semtable[idx].nbproc == 0)
		freesem(idx);

	semtable[idx].state|=UNLINKED;

	return 0;	/* Successful completion */
}
