# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := echo-cpp

#===============================================================================
# Build Artifacts
#===============================================================================

# Libraries
export LIBRARIES := -Wl,--start-group $(LIBC) $(LIBCXX) $(LIBPOSIX) -Wl,--end-group

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
	$(CXX) $(LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)

clean:
	rm -rf $(OBJECTS)
	rm -rf $(BINARIES_DIR)/$(BINARY)

# Builds C++ source files.
%.o: %.cpp
	$(CXX) $(CXXFLAGS) $< -c -o $@
