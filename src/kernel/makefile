# 
# Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>.
#
# makefile - builds nanvix kernel.
#

# C source files.
C_SRC = $(wildcard boot/*.c)        \
        $(wildcard core/*.c)        \
        $(wildcard dev/*.c)         \
        $(wildcard dev/ramdisk/*.c) \
        $(wildcard dev/tty/*.c)     \
        $(wildcard fs/*.c)          \
        $(wildcard init/*.c)        \
        $(wildcard lib/*.c)         \
        $(wildcard mm/*.c)          \
        $(wildcard pm/*.c)          \
        $(wildcard sys/*.c)         \

# Assembly source files.
ASM_SRC = $(wildcard boot/*.S) \
          $(wildcard core/*.S) \

# Object files.
OBJ = $(ASM_SRC:.S=.o) \
	  $(C_SRC:.c=.o)   \

# Executable file.clear
EXEC = kernel

# Compiler options.
CFLAGS += -c -I $(INCDIR)
CFLAGS += -ansi -pedantic
CFLAGS += -nostdlib -nostdinc -fno-builtin -fno-stack-protector
CFLAGS += -Wall -Wextra -Werror

# Assembler options
ASMFLAGS += -Wa,--divide,--warn

# Linker options.
LDFLAGS = -Tlink.ld

# Builds object file from C source file.
%.o: %.c
	$(CC) $< $(CFLAGS) -o $@

# Builds object file from assembly source file.
%.o: %.S
	$(CC) $< $(CFLAGS) $(ASMFLAGS) -o $@

# Builds Nanvix kernel.
all: $(OBJ)
	$(LD) $(LDFLAGS) $(OBJ)  -o $(BINDIR)/$(EXEC)

# Cleans object and executable files.
clean:
	@rm -f $(OBJ) 
	@rm -f $(BINDIR)/$(EXEC)
