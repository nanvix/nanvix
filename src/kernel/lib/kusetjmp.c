
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>

PUBLIC void kusetjmp(kjmp_buf *kenv)
{
	UNUSED(kenv);
	
	curr_proc->flags |= PROC_JMPSET;
}
