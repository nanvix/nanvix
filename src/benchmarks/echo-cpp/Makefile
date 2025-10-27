# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := echo-cpp

#===============================================================================
# Build Artifacts
#===============================================================================

# Libraries
export LIBRARIES := -Wl,--start-group $(LIBPOSIX) $(LIBC) $(LIBCXX) -Wl,--end-group

# Source Files
export SOURCES := $(wildcard *.cpp)

# Object Files
export OBJECTS = $(SOURCES:.cpp=.o)

# Binary
export BINARY := $(PROGRAM_NAME).$(EXEC_FORMAT)

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJECTS)
	$(NANVIX_CXX) $(NANVIX_LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)

clean:
	rm -rf $(OBJECTS)
	rm -rf $(BINARIES_DIR)/$(BINARY)

# Builds C++ source files.
%.o: %.cpp
	$(NANVIX_CXX) $(NANVIX_CXXFLAGS) $< -c -o $@
