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
 * Closes a file.
 */
EXTERN void do_close(int fd);

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
	header = block_read(inode->dev, blk);
	elf = header->data;
	
	/* Bad ELF file. */
	if (!is_elf(elf))
	{
		block_put(header);
		curr_proc->errno = -ENOEXEC;
		return (0);
	}
	
	/* Bad ELF file. */
	if (elf->e_phoff + elf->e_phnum*elf->e_phentsize > BLOCK_SIZE)
	{
		block_put(header);
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
		if (seg[i].p_filesz < seg[i].p_memsz)
		{
			block_put(header);
			curr_proc->errno = -ENOEXEC;
			return (0);
		}
		
		addr = ALIGN(seg[i].p_vaddr, seg[i].p_align);
		
		/* Text section. */
		if (seg[i].p_flags & (PF_R | PF_X))
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
			block_put(header);
			curr_proc->errno = -ENOMEM;
			return (0);
		}
		
		/* Attach memory region. */
		if (attachreg(curr_proc, preg, addr, reg))
		{
			freereg(reg);
			block_put(header);
			curr_proc->errno = -ENOMEM;
			return (0);
		}
		
		loadreg(inode, reg, seg[i].p_offset, seg[i].p_filesz);	
		
		unlockreg(reg);	
	}
	
	entry = elf->e_entry;
	
	block_put(header);
	
	return (entry);
}

/*
 * Executes a program.
 */
PUBLIC int sys_execve(const char *filename, const char **argv, const char **envp)
{
	int i;                /* Loop index.             */
	struct inode *inode;  /* File inode.             */
	struct region *reg;   /* Process region.         */
	addr_t entry;         /* Program entry point.    */
	
	UNUSED(argv);
	UNUSED(envp);
	
	inode = inode_name(filename);
	
	/* Failed to get inode. */
	if (inode == NULL)
		return (curr_proc->errno);
	
	/* Not a regular file. */
	if (!S_ISREG(inode->mode))
	{
		inode_put(inode);
		return (-EACCES);
	}
	
	/* Not allowed. */
	if (!permission(inode->mode, inode->uid, inode->gid, curr_proc, MAY_EXEC, 0))
	{
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
	if ((reg = allocreg(S_IRUSR | S_IWUSR, 2*PAGE_SIZE, REGION_DOWNWARDS)) == NULL)
		goto die0;
	if (attachreg(curr_proc, STACK(curr_proc), USTACK_ADDR, reg))
		goto die1;
	unlockreg(reg);
	
	/* Attach heap region. */
	if ((reg = allocreg(S_IRUSR | S_IWUSR, 2*PAGE_SIZE, REGION_UPWARDS)) == NULL)
		goto die0;
	if (attachreg(curr_proc, HEAP(curr_proc), UHEAP_ADDR, reg))
		goto die1;
	unlockreg(reg);
	
	inode_put(inode);
	
	user_mode(entry);
	
	/* Will not return. */
	return (0);

die1:
	unlockreg(reg);
	freereg(reg);
die0:
	inode_put(inode);
	die();
	return (-1);
}
