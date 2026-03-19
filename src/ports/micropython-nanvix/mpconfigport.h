// MicroPython configuration for NanVix port
// Based on ports/minimal/mpconfigport.h

#include <stdint.h>

// Use the minimum starting configuration.
#define MICROPY_CONFIG_ROM_LEVEL (MICROPY_CONFIG_ROM_LEVEL_MINIMUM)

// Enable the compiler and REPL.
#define MICROPY_ENABLE_COMPILER     (1)

// Enable garbage collection.
#define MICROPY_ENABLE_GC           (1)

// Enable REPL helpers (tab completion, history).
#define MICROPY_HELPER_REPL         (1)

// Enable external imports (for frozen modules).
#define MICROPY_ENABLE_EXTERNAL_IMPORT (1)

// Frozen bytecode support.
#define MICROPY_QSTR_EXTRA_POOL     mp_qstr_frozen_const_pool
#define MICROPY_MODULE_FROZEN_MPY   (1)

// Use stdout for I/O (NanVix provides POSIX read/write).
#define MICROPY_MIN_USE_STDOUT      (1)

// Heap size: 256KB (NanVix VMs have plenty of memory).
#define MICROPY_HEAP_SIZE           (256 * 1024)

// Path allocation limit.
#define MICROPY_ALLOC_PATH_MAX      (256)

// Minimum chunk size for parser.
#define MICROPY_ALLOC_PARSE_CHUNK_INIT (16)

// Type definitions for i686 (32-bit).
typedef intptr_t mp_int_t;
typedef uintptr_t mp_uint_t;
typedef long mp_off_t;

// alloca() declaration.
#include <alloca.h>

// Board identification.
#define MICROPY_HW_BOARD_NAME       "NanVix"
#define MICROPY_HW_MCU_NAME         "i686"

#define MP_STATE_PORT               MP_STATE_VM
