#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>

/**
 * @brief	Waits on a semaphore
 *		 
 * @param	num The inode number of the semaphore
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 *
 * TODO for error detection :
 *			EDEADLK : A deadlock condition was detected.
 *			EINTR : A signal interrupted this function.
 */
PUBLIC int sys_semwait(ino_t num)
{
	struct inode *seminode;

	seminode = inode_get(semdirectory->dev,num);

	if (seminode == NULL)
		return (-EINVAL);

	inode_unlock(seminode);

	freesem(&sembuf);
	file_read(seminode, &sembuf, sizeof(struct ksem),0);

	kprintf("wait : %d, sem %d value : %d",curr_proc->pid,num,sembuf.value);

	while (sembuf.value <= 0)
	{
		sleep(semwaiters,curr_proc->priority);
		freesem(&sembuf);
		file_read(seminode, &sembuf, sizeof(struct ksem),0);
	}

	sembuf.value--;
	file_write(seminode, &sembuf, sizeof(struct ksem),0);

	return (0);
}