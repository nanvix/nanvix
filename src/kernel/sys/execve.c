/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/execve.c - execve() system call implementation.
 */

#include <asm/util.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <elf.h>
#include <errno.h>

/*
 * Asserts if a file is a ELF executable.
 */
PRIVATE int is_elf(struct elf32_fhdr *header)
{
	/* Check ELF magic value. */
    if ((header->e_ident[0] != ELFMAG0) || (header->e_ident[1] != ELFMAG1) ||
		(header->e_ident[2] != ELFMAG2) || (header->e_ident[3] != ELFMAG3))
		return (0);
	
	return (1);
}

/*
 * Loads an ELF 32 executable.
 */
PRIVATE addr_t load_elf32(struct inode *inode)
{
	int i;                  /* Loop index.                    */
	addr_t addr;            /* Region address.                */
	addr_t entry;           /* Program entry point.           */
	struct elf32_fhdr *elf; /* ELF file header.               */
	struct elf32_phdr *seg; /* ELF Program header.            */
	block_t blk;            /* Working block number.          */
	struct buffer *header;  /* File headers block buffer.     */
	struct region *reg;     /* Working memory region.         */
	struct pregion *preg;   /* Working process memory region. */
	
	blk = block_map(inode, 0, 0);
	
	/* Empty file. */
	if (blk == BLOCK_NULL)
	{
		curr_proc->errno = -ENOEXEC;
		return (0);
	}
	
	/* Read ELF file header. */
	header = bread(inode->dev, blk);
	elf = header->data;
	
	/* Bad ELF file. */
	if (!is_elf(elf))
	{
		brelse(header);
		curr_proc->errno = -ENOEXEC;
		return (0);
	}
	
	/* Bad ELF file. */
	if (elf->e_phoff + elf->e_phnum*elf->e_phentsize > BLOCK_SIZE)
	{
		brelse(header);
		curr_proc->errno = -ENOEXEC;
		return (0);
	}
	
	seg = (struct elf32_phdr *)((char*)header->data + elf->e_phoff);
	
	/* Load segments. */
	for (i = 0; i < elf->e_phnum; i++)
	{
		/* Not loadable. */
		if (seg[i].p_type != PT_LOAD)
			continue;
		
		/* Broken executable. */
		if (seg[i].p_filesz > seg[i].p_memsz)
		{
			kprintf("broken executable");
			
			brelse(header);
			curr_proc->errno = -ENOEXEC;
			return (0);
		}
		
		addr = ALIGN(seg[i].p_vaddr, seg[i].p_align);
		
		/* Text section. */
		if (!(seg[i].p_flags ^ (PF_R | PF_X)))
		{
			preg = TEXT(curr_proc);
			reg = allocreg(S_IRUSR | S_IXUSR, seg[i].p_memsz, 0);
		}
		
		/* Data section. */
		else
		{
			preg = DATA(curr_proc);
			reg = allocreg(S_IRUSR | S_IWUSR, seg[i].p_memsz, 0);
		}
		
		/* Failed to allocate region. */
		if (reg == NULL)
		{
			brelse(header);
			curr_proc->errno = -ENOMEM;
			return (0);
		}
		
		/* Attach memory region. */
		if (attachreg(curr_proc, preg, addr, reg))
		{
			freereg(reg);
			brelse(header);
			curr_proc->errno = -ENOMEM;
			return (0);
		}
		
		loadreg(inode, reg, seg[i].p_offset, seg[i].p_filesz);
		
		unlockreg(reg);	
	}
	
	entry = elf->e_entry;
	
	brelse(header);
	
	return (entry);
}

/*
 * Returns the number of strings of a null terminated vector of strings.
 */
PRIVATE int count(const char **str)
{
	int s;          /* Working string. */
	const char **r; /* Read pointer.   */
	int c;          /* String couynt.  */
	
	/* Count the number of strings. */
	for (c = 0, r = str; (s = fudword(r)) != 0; r++)
	{				
		/* Bad string vector. */
		if (s == -1)
		{
			curr_proc->errno = -EFAULT;
			return (-1);
		}
			
		c++;
	}
	
	return (c);
}

/*
 * Copy strings of a vector of strings to somewhere.
 */
PRIVATE int copy_strings(int count, const char **strings, char *where, int p, const int bypass)
{
	int ch;          /* Working character.     */
	const char *str; /* Working string.        */
	int length;      /* Working string length. */
	
	/* Copy strings. */
	while (count-- > 0)
	{
		str = (bypass) ? *strings : (const char *)fudword(strings + count);
		length = 1;
		
		/* Get working string length. */
		while ((ch = ((bypass) ? *str : fubyte(str))) != '\0')
		{			
			/* Bad string. */
			if (ch < 0)
			{
				curr_proc->errno = -EFAULT;
				return (-1);
			}
			
			length++;
			str++;
		}
		
		
		/* Copy working string. */
		while (length-- > 0)
		{
			where[p] = (char) fubyte(str);
		
			p--;
			str--;
			
			/* Strings too long. */
			if (p < 0)
			{
				curr_proc->errno = -E2BIG;
				return (-1);
			}
		}
	}
	
	return (p);
}

/*
 * Create string tables for argv and envp.
 */
PRIVATE addr_t create_tables(char *stack, size_t size, int p, int argc, int envc)
{
	int argv; /* Argument variables table.    */
	int envp; /* Environment variables table. */
	int sp;   /* Stack pointer.               */
	
	sp = p & ~(DWORD_SIZE - 1);
	sp -= (envc + 1)*DWORD_SIZE;
	envp = sp;
	sp -= (argc + 1)*DWORD_SIZE;
	argv = sp;
	
	/* Arguments too long. */
	if (sp < (4*DWORD_SIZE))
	{
		curr_proc->errno = -E2BIG;
		return (0);
	}
	
	sp -= DWORD_SIZE; (*((dword_t *)(stack + sp))) = USTACK_ADDR - size + envp;
	sp -= DWORD_SIZE; (*((dword_t *)(stack + sp))) = USTACK_ADDR - size + argv;
	sp -= DWORD_SIZE; (*((dword_t *)(stack + sp))) = argc;
	sp -= DWORD_SIZE;
	
	/* 
	 * Build argv table.
	 * p is the first byte available in the stack
	 * so we need to increment it before checking strings.
	 */
	while (argc-- > 0)
	{
		(*((dword_t *)(stack + argv))) = USTACK_ADDR - size + p + 1;
		while (stack[++p] != '\0') /* nothing. */ ;
		argv += DWORD_SIZE;
	}
	(*((dword_t *)(stack + argv))) = 0;
	
	/* Build envp table. */
	while (envc-- > 0)
	{
		(*((dword_t *)(stack + envp))) = USTACK_ADDR - size + p + 1;
		while (stack[++p] != '\0') /* nothing. */ ;
		envp += DWORD_SIZE;
	}
	(*((dword_t *)(stack + envp))) = 0;
	
	
	return (USTACK_ADDR - size + sp);
}


/*
 * Builds arguments.
 */
PRIVATE addr_t buildargs
(void *stack, size_t size, const char **argv, const char **envp)
{
	int p;        /* Stack pointer. */
	int argc;     /* argv length.   */
	int envc;     /* envc length.   */
	
	/* Get argv count. */
	if ((argc = count(argv)) < 0)
		return (0);
		
	/* Get envp count. */
	if ((envc = count(envp)) < 0)
		return (0);
		
	/* Copy argv and envp to stack. */
	if ((p = copy_strings(envc, envp, stack, size - 1, 0)) < 0)
		return (0);
	if ((p = copy_strings(argc, argv, stack, p, 0)) < 0)
		return (0);
	if ((p = create_tables(stack, size, p, argc, envc)) == 0)
		return (0);
		
	return (p);
}

/*
 * Executes a program.
 */
PUBLIC int sys_execve(const char *filename, const char **argv, const char **envp)
{
	int i;                /* Loop index.          */
	struct inode *inode;  /* File inode.          */
	struct region *reg;   /* Process region.      */
	addr_t entry;         /* Program entry point. */
	addr_t sp;            /* User stack pointer.  */
	char *name;           /* File name.           */
	char stack[ARG_MAX];  /* Stack size.          */

	/* Get file name. */
	if ((name = getname(filename)) == NULL)
		return (curr_proc->errno);

	/* Build arguments before freeing user memory. */
	kmemset(stack, 0, ARG_MAX);
	if (!(sp = buildargs(stack, ARG_MAX, argv, envp)))
	{
		putname(name);
		return (curr_proc->errno);
	}

	/* Get file's inode. */
	if ((inode = inode_name(name)) == NULL)
	{
		putname(name);
		return (curr_proc->errno);
	}

	/* Not a regular file. */
	if (!S_ISREG(inode->mode))
	{
		putname(name);
		inode_put(inode);
		return (-EACCES);
	}

	/* Not allowed. */
	if (!permission(inode->mode, inode->uid, inode->gid, curr_proc, MAY_EXEC, 0))
	{
		putname(name);
		inode_put(inode);
		return (-EACCES);
	}

	/* Close file descriptors. */
	for (i = 0; i < OPEN_MAX; i++)
	{
		if (curr_proc->close & (1 << i))
			do_close(i);
	}

	/* Detach process memory regions. */
	for (i = 0; i < NR_PREGIONS; i++)
		detachreg(curr_proc, &curr_proc->pregs[i]);
	
	/* Load executable. */
	if (!(entry = load_elf32(inode)))
		goto die0;

	/* Attach stack region. */
	if ((reg = allocreg(S_IRUSR | S_IWUSR, PAGE_SIZE, REGION_DOWNWARDS)) == NULL)
		goto die0;
	if (attachreg(curr_proc, STACK(curr_proc), USTACK_ADDR - 1, reg))
		goto die1;
	unlockreg(reg);

	/* Attach heap region. */
	if ((reg = allocreg(S_IRUSR | S_IWUSR, PAGE_SIZE, REGION_UPWARDS)) == NULL)
		goto die0;
	if (attachreg(curr_proc, HEAP(curr_proc), UHEAP_ADDR, reg))
		goto die1;
	unlockreg(reg);
	
	inode_put(inode);
	putname(name);

	kmemcpy((void *)(USTACK_ADDR - ARG_MAX), stack, ARG_MAX);
	
	user_mode(entry, sp);
	
	/* Will not return. */
	return (0);

die1:
	unlockreg(reg);
	freereg(reg);
die0:
	inode_put(inode);
	putname(name);
	die(((SIGSEGV & 0xff) << 16) | (1 << 9));
	return (-1);
}
