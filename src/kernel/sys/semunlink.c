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
	struct inode *seminode;


	seminode = get_sem(name);

	if (seminode == NULL)
		return (-ENOENT);

	freesem(&sembuf);
	file_read(seminode, &sembuf, sizeof(struct ksem),0);
	
  	/* Checking WRITE permission */ 
  	if (!permission(seminode->mode, seminode->uid, seminode->gid, curr_proc, MAY_WRITE, 0)) 
    	return (-EACCES);

	if (sembuf.nbproc == 0)
	{
		seminode->count = 1;
		freesem(&sembuf);
		inode_put(seminode);
		return 0;
	}
	else
	{
		sembuf.state |= UNLINKED;
		file_write(seminode, &sembuf, sizeof(struct ksem),0);
	}

	return 0;	/* Successful completion */
}
