#include <fcntl.h>
#include <stdarg.h>
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <errno.h>

/**
 *	@brief Set the semaphore of index idx
 *		   to corresponding value and semname
 *         in the global semaphore table
 */
int add_table(int value, const char* semname, int idx, struct inode* seminode)
{
	sem_path(semname, semtable[idx].name);
	semtable[idx].value = value;
	semtable[idx].currprocs[0] = curr_proc->pid;
	semtable[idx].num = seminode->num;
	semtable[idx].dev = seminode->dev;
	return 0;
}

/**
 * @brief Opens a semaphore
 *		 
 * @param	name 	Name of the semaphore
 *			oflag	Creation flags
 *			mode	User permissions
 *			value 	Semaphore value
 *
 * @returns the inode number of the semaphore in the
 */

/* TODO for error detection :
 *			ENOSPC : There is insufficient space on a storage device for the creation of the new named semaphore.
 *			EINTR : The sem_open() operation was interrupted by a signal.
 */
PUBLIC int sys_semopen(const char* name, int oflag, ...)
{
	mode_t mode;
	int value;
	va_list arg;				/* Variable argument */
	int i, freeslot, semid;
	struct inode *inode;
	freeslot = -1;

	/* Name invalid */
	if (namevalid(name) == (-1))
		return (ENAMETOOLONG);

	if (existence_semaphore(name) == (-1))	/* This semaphore does not exist */
	{
		if(oflag & O_CREAT)	
		{
			/* Both O_CREAT and O_EXCL flags set */
			if (oflag & O_EXCL)
				return (-EEXIST);
			
			/* Semaphore creation if it does not exist */
			va_start(arg, oflag);
			mode = va_arg(arg, mode_t);
			value = va_arg(arg, int);
			va_end(arg);

			/* Value greater than maximum */
			if (!SEM_VALID_VALUE(value))
				return (-EINVAL);

			/* Searching a free slot in the semaphore table */
			semid = -1;

			for (i = 0; i < SEM_OPEN_MAX; i++)
			{
				if (semtable[i].num == 0)
				{
					semid = i;
					break;
				}
			}

			/* Table if full */
			if (semid < 0) 
				return (-ENFILE);

			/* Creates the inode */
			if (!(inode = inode_semaphore(name, mode)))
			{
				/*  Access forbiden to the parent directory 
				 * 	or parent directory doesn't exist
				 */
				return (-EACCES);
			}
			/* Add the semaphore in the semaphore table */
			add_table(value, name, semid, inode);
		}
		/* O_CREAT not set and sem does not exist */
		else
			return (-ENOENT);
	}
	else	/* This semaphore already exists */
	{
		char sempath[MAX_SEM_NAME];
		sem_path(name,sempath);
		/* Searching the semaphore indice in the semaphore table */
		semid = search_semaphore (sempath);
		/* This opening will increment the inode counter */
		inode = inode_name(sempath);

		/* Checking if there is WRITE and READ permissions */
		if (	!permission(inode->mode, inode->uid, inode->gid, curr_proc, MAY_WRITE, 0) \
			 ||	!permission(inode->mode, inode->uid, inode->gid, curr_proc, MAY_READ, 0) )
		{
			inode_put(inode);
			inode_unlock(inode);
			return (-EACCES);
		}

		for (i = 0; i < PROC_MAX; i++)
		{
			/* 	
			 *	Searching for a free slot : a same pid can appear multiple
			 * 	times in order to allow a potential multi-threading
			 */
			if (semtable[semid].currprocs[i] == (-1) && freeslot < 0)
			{
				freeslot = i;
				break;
			}
		}

		if (freeslot == (-1))
		{
			inode_put(inode);
			inode_unlock(inode);
			return (-EMFILE);
		}

		semtable[semid].currprocs[freeslot] = curr_proc->pid;
	}

	inode_unlock(inode);

	return semid;
}
