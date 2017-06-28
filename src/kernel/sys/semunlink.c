#include <fcntl.h>
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>
#include <nanvix/syscall.h>

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
	struct inode* seminode = inode_name(name);

	if (seminode == NULL)
	{
		/* The semaphore descriptor doesn't exist */
		return (-ENOENT);
	}
	
	if (!permission(seminode->mode, seminode->uid, seminode->gid, curr_proc, MAY_WRITE, 0))
	{
		inode_put(seminode);
		inode_unlock(seminode);
		return -(EACCES);
	}

 	idx = search_semaphore(name);
	
	if (semtable[idx].nbproc == 0)
	{
		semtable[idx].name[0] = '\0';
		remove_semaphore (name);
		// dir_remove(semdirectory, name);
	}
	else
	{
		seminode->nlinks = 0;
	}

	inode_put(seminode);
	inode_unlock(seminode);

	return 0;	/* Successful completion */
}
