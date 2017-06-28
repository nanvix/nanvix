#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>

/**
 * @brief Close a semaphore for a given process
 *		 
 * @param	idx	Semaphore index (in semaphore
 *				table to close
 *
 * @returns 0 in case of successful completion
 *			SEM_FAILED otherwise
 */
PUBLIC int sys_sempost(int idx)
{
	struct inode *seminode;
	seminode = inode_name(semtable[idx].name);

	if (seminode == NULL)
		return (-EINVAL);

	semtable[idx].value++;

	inode_unlock(seminode);

	if (semtable[idx].value>0)
		wakeup(semtable[idx].semwaiters);

	inode_put(seminode);

	return (0);
}
