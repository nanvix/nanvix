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
PUBLIC int sys_sempost(ino_t num)
{
	struct inode *seminode;

	seminode = inode_get(semdirectory->dev,num);
	inode_unlock(seminode);
	
	if (seminode == NULL)
		return (-EINVAL);

	freesem(&sembuf);
	file_read(seminode, &sembuf, sizeof(struct ksem),0);

	sembuf.value++;

	file_write(seminode, &sembuf, sizeof(struct ksem),0);

	if (sembuf.value>0)
		wakeup(semwaiters);

	return (0);
}
