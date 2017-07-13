#include <nanvix/syscall.h>
#include <errno.h>

/**
 * @brief	Unlinks a semaphore for deletion
 *		 
 * @param	name Semaphore name
 *
 * @returns returns 0 in case of successful completion
 *			returns SEM_FAILED otherwise
 */
int sem_unlink(const char *name)
{	
	int ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_semunlink),
		  "b" (name)
	);
	
	if (ret < 0)
	{
		errno = ret;
		return (-1);
	}

	return 0;
}