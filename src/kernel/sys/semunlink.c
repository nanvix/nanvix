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
	int idx = -1;
	struct inode* seminode = get_sem(name);

	if (seminode == NULL)
	{
		/* The semaphore descriptor doesn't exist */
		return (-ENOENT);
	}
	else
	{
		if (!permission(seminode->mode, seminode->uid, seminode->gid, curr_proc, MAY_WRITE, 0))
		{
			return -(EACCES);
		}

	 	idx = search_semaphore(name);
		
		if (semtable[idx].nbproc == 0)
		{
			semtable[idx].num = 0;
			dir_remove(semdirectory, name);
		}
		else
		{
			semtable[idx].state |= UNLINKED;
		}

		inode_put(seminode);

		return 0;	/* Successful completion */
	}
}
