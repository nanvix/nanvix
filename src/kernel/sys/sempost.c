#include <sys/sem.h>
#include <errno.h>

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
	seminode = inode_name(semtable[idx].name);

	if (seminode == NULL)
		return (-EINVAL);

	semtable[idx].value++;
	

	if (semtable[idx].value>0)
		wakeup(semtable[idx].semwaiters);

	inode_put(seminode);
	inode_unlock(seminode);

	return (0);
}
