#include <fcntl.h>
#include <stdarg.h>
#include <sys/sem.h>
#include <nanvix/klib.h>
#include <semaphore.h>
#include <errno.h>
#include <nanvix/fs.h>

/**
 *	@brief Add the semaphore to the sem table
 *
 *	If it does not exist, the semaphore is added to the table
 * 	
 *	@returns the index of the named semaphore in the semaphore table
 *			 in case of successful completion, -1 otherwise
 *			
 */
int free_sem_entry()
{
	int idx;
	for (idx = 0; idx < SEM_OPEN_MAX; idx++)
	{
		if(semtable[idx].name[0] == '\0')
		{
			return idx;
		}
	}
	/* Semaphore table full */
	return -1;
}


int add_table(struct inode *inode, int value, int idx, const char* semname)
{
	kstrcpy(semtable[idx].name, semname);
	semtable[idx].value = value;
	semtable[idx].currprocs[0] = curr_proc->pid;
	semtable[idx].nbproc = 0;
	semtable[idx].state = 0;
	semtable[idx].seminode = inode;
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
 *			EMFILE : Too many semaphore descriptors or file descriptors are currently in use by this process.
 *			EACESS : Check creation permission if semaphore does not exist
 */
PUBLIC int sys_semopen(const char* name, int oflag, ...)
{
	mode_t mode;
	int value;
	va_list arg;				/* Variable argument */
	int i, freeslot, semid;		/* Index of the opened semaphore */
	struct inode *inode;
	freeslot = -1;

	/* Name invalid */
	if (namevalid(name) == (-1))
		return (-EINVAL);

	/*  
	 *	inode == corresponding semaphore inode if it exists
	 *  NULL otherwise
	 */
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
				if (semtable[i].name[0] == '\0')
				{
					semid = i;
					break;
				}
			}

			/* Table if full */
			if (semid < 0) 
				return (-ENFILE);

			/* Creates the inode */
			inode = inode_semaphore(name, mode);
			/* Add the semaphore in the semaphore table */
			add_table(inode, value, semid, name);
		}
		/* O_CREAT not set and sem does not exist */
		else
			return (-ENOENT);
	}
	else	/* This semaphore already exists */
	{
		/* Searching the semaphore indice in the semaphore table */
		semid = search_semaphore (name);
		/* This opening will increment the inode counter */
		inode = inode_name(name);
		semtable[semid].seminode = inode;
		inode_unlock(inode);

		/* Checking if there is WRITE and READ permissions */
		if (	!permission(inode->mode, inode->uid, inode->gid, curr_proc, MAY_WRITE, 0) \
			 ||	!permission(inode->mode, inode->uid, inode->gid, curr_proc, MAY_READ, 0) )
		{
			inode_put(inode);
			return (EACCES);
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

			/* Not allowing multiple open
				if (semtable[semid].currprocs[i] == curr_proc->pid)
				{
					return ERROR;
				}
			 */
		}

		if (freeslot == (-1))
		{
			inode_put(inode);
			return (-EMFILE);
		}

		semtable[semid].currprocs[freeslot] = curr_proc->pid;
	}

	semtable[semid].nbproc++;
	return semid;
}
