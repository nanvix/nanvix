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
PUBLIC int sys_semclose(ino_t num)
{
	int i;
	struct inode *seminode;

	seminode = inode_get(semdirectory->dev,num);
	inode_unlock(seminode);

	if (seminode == NULL)
		return (-EINVAL);

	freesem(&sembuf);
	file_read(seminode, &sembuf, sizeof(struct ksem),0);

	sembuf.nbproc--;

	/*
	 * The semaphore is no longer accessible when 0 process use it
	 * and only if it has been unlinked once 
	 */
	if (sembuf.nbproc == 0 && (sembuf.state&UNLINKED))
	{
		seminode->count = 1;
		kprintf("SUPRECAO FROM CLOSING");
		kprintf("Nombre de ref de l'inode : %d", seminode->nlinks);
		freesem(&sembuf);
		inode_put(seminode);
		return 0;
	}
	else
	{
		for (i = 0; i < PROC_MAX; i++)
		{
			if (sembuf.currprocs[i] == curr_proc->pid)
			{	
				sembuf.currprocs[i] = (-1);
				file_write(seminode, &sembuf, sizeof(struct ksem),0);
				return (0);
			}
		}
		/* Proc pid not found in the semaphore pid table. */
	}
return (-1);
}
