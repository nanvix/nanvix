/*
 * Copyleft (C) 2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * <sys/ps.c> - ps() system call implementation.
 */

#include <nanvix/klib.h>
#include <nanvix/pm.h>

void reverse(char* s)
{
	int i, j;
	char c;

	for (i = 0, j = kstrlen(s)-1; i<j; i++, j--)
	{
		c = s[i];
		s[i] = s[j];
		s[j] = c;
	}
}

void itoa(int n, char* s)
{
	int i, sign;

	if ((sign = n) < 0) 
		n = -n;

	i = 0;
	do
	{
		s[i++] = n % 10 + '0';
	} while ((n /= 10) > 0);

	if (sign < 0)
		s[i++] = '-';
	
	s[i] = '\0';
	reverse(s);
}

void prepareValue(int value, char* s, int padding)
{
	int i,len,size;

	itoa(value, s);
	len = padding - kstrlen(s);

	size = kstrlen(s);
	for(i=size; i<len+size; i++)
		*(s+i) = ' ';

	*(s+i) = '\0';
}

/*
 * Gets process information and print on the screen
 */

PUBLIC int sys_ps()
{
	struct process *p;

	kprintf("------------------------------- Process Status"
			" -------------------------------\n"
		    "NAME               PID   COUNTER   PRIORITY   NICE"
		    "   UTIME   KTIME     STATUS");


	char name    [26];
	char pid     [26];
	char counter [26];
	char priority[26];
	char nice    [26];
	char utime   [26];
	char ktime   [26];

	const char *states[7];
	states[0] = "DEAD";
	states[1] = "ZOMBIE";
	states[2] = "RUNNING";
	states[3] = "READY";
	states[4] = "WAITING";
	states[5] = "SLEEPING";
	states[6] = "STOPPED";

	unsigned len = 0;
	unsigned i;
	int size;

	for (p = IDLE; p <= LAST_PROC; p++)
	{
		/* Skip invalid processes. */
		if (!IS_VALID(p))
			continue;

		/* Name */
		size = kstrlen(p->name);
		kstrcpy(name, p->name);
		len = 20 - size;

		for(i=size; i<len+size-1; i++)
			*(name+i) = ' ';

		*(name+i) = '\0';

		/* Pid */
		prepareValue(p->pid, pid, 6);

		/* Remaining Quantum */
		prepareValue(p->counter, counter, 10);

		/* Priority */
		prepareValue(p->priority, priority, 11);

		/* Nice */
		prepareValue(p->nice, nice, 7);

		/* Utime */
		prepareValue(p->utime, utime, 8);

		/* Ktime */
		prepareValue(p->ktime, ktime, 10);
		
		kprintf("%s%s%s%s%s%s%s%s",name, pid, 
			counter, priority, nice, utime, ktime, states[(int)p->state] );
	}

	kprintf("\nLast process: %s, pid: %d\n",last_proc->name, last_proc->pid);
	return 0;
}