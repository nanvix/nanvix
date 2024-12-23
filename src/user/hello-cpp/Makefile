# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := hello-cpp

#===============================================================================
# Build Artifacts
#===============================================================================

# Libraries
export LIBC := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libc.a
export LIBCXX := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libstdc++.a
export LIBPOSIX := $(LIBRARIES_DIR)/libposix.a
export LIBRARIES := -Wl,--start-group $(LIBC) $(LIBCXX) $(LIBPOSIX) -Wl,--end-group

# Source Files
export SOURCES := $(wildcard *.cpp)

# Object Files
export OBJECTS = $(SOURCES:.cpp=.o)

# Binary
export BINARY := $(PROGRAM_NAME).$(EXEC_FORMAT)

# C Toolchain
export CXX := $(TOOLCHAIN_DIR)/bin/i686-nanvix-g++

# Compiler Options
export CXXFLAGS += -march=pentiumpro -Wstack-usage=4096

# Linker Options
export LDFLAGS := -z noexecstack -T $(BUILD_DIR)/user/linker/$(TARGET)/user.ld

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJECTS)
ifeq ($(VERBOSE), no)
	@echo [LD] $(BINARY)
	@$(CXX) $(LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)
else
	$(CXX) $(LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)
endif

clean:
	rm -rf $(OBJECTS)
	rm -rf $(BINARIES_DIR)/$(BINARY)

# Builds C++ source files.
%.o: %.cpp
ifeq ($(VERBOSE), no)
	@echo [CXX] $@
	@$(CXX) $(CXXFLAGS) $< -c -o $@
else
	$(CXX) $(CXXFLAGS) $< -c -o $@
endif
