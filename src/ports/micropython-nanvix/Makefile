include ../../py/mkenv.mk

# Cross-compilation for NanVix (i686 bare-metal via NanVix toolchain).
CROSS = 1
CROSS_COMPILE ?= i686-nanvix-

# qstr definitions (must come before including py.mk).
QSTR_DEFS = qstrdefsport.h

# MicroPython feature configurations.
MICROPY_ROM_TEXT_COMPRESSION ?= 0

# Include MicroPython core make definitions.
include $(TOP)/py/py.mk

# Compiler: use i686-nanvix-gcc from the NanVix toolchain.
CC = $(CROSS_COMPILE)gcc
LD = $(CROSS_COMPILE)gcc
SIZE = $(CROSS_COMPILE)size
OBJCOPY = $(CROSS_COMPILE)objcopy

# Include paths.
INC += -I.
INC += -I$(TOP)
INC += -I$(BUILD)

# Compiler flags for NanVix target.
CFLAGS += $(INC) -Wall -std=c99 $(COPT)
CFLAGS += -m32 -march=pentiumpro
CFLAGS += -fdata-sections -ffunction-sections

# NanVix-specific paths (set by Docker or caller).
NANVIX_ROOT ?= /mnt
NANVIX_LIB_DIR ?= $(NANVIX_ROOT)/lib
TOOLCHAIN_DIR ?= /opt/nanvix

# Linker flags: use NanVix linker script and libraries.
LDFLAGS += -z noexecstack
LDFLAGS += -T $(NANVIX_ROOT)/build/user/linker/x86/user.ld
LDFLAGS += -Wl,--gc-sections
LDFLAGS += -Wl,-Map=$@.map,--cref

# Libraries: NanVix POSIX + libc + libm.
LIBS += -Wl,--start-group
LIBS += $(NANVIX_LIB_DIR)/libposix.a
LIBS += $(TOOLCHAIN_DIR)/i686-nanvix/lib/libc.a
LIBS += $(TOOLCHAIN_DIR)/i686-nanvix/lib/libm.a
LIBS += -Wl,--end-group

# Optimisation.
CSUPEROPT = -Os

ifeq ($(DEBUG), 1)
CFLAGS += -O0 -g
else
CFLAGS += -Os -DNDEBUG
endif

# Source files.
SRC_C = \
	main.c \
	uart_core.c \
	shared/libc/printf.c \
	shared/readline/readline.c \
	shared/runtime/pyexec.c \
	shared/runtime/stdout_helpers.c \
	$(BUILD)/_frozen_mpy.c \

SRC_QSTR += shared/readline/readline.c shared/runtime/pyexec.c

# Object files.
OBJ += $(PY_CORE_O)
OBJ += $(addprefix $(BUILD)/, $(SRC_C:.c=.o))

# Build target.
all: $(BUILD)/micropython.elf

$(BUILD)/_frozen_mpy.c: $(TOP)/tests/frozen/frozentest.mpy $(BUILD)/genhdr/qstrdefs.generated.h
	$(ECHO) "MISC freezing bytecode"
	$(Q)$(TOP)/tools/mpy-tool.py -f -q $(BUILD)/genhdr/qstrdefs.preprocessed.h -mlongint-impl=none $< > $@

$(BUILD)/micropython.elf: $(OBJ)
	$(ECHO) "LINK $@"
	$(Q)$(LD) $(LDFLAGS) -o $@ $^ $(LIBS)
	$(Q)$(SIZE) $@

include $(TOP)/py/mkrules.mk
