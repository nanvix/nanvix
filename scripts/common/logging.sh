#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Logging functions.
#

#===================================================================================================
# Include Guard
#===================================================================================================

# Skip this file if already included.
if [[ -n "${__LOGGING_SH_INCLUDED:-}" ]]; then
    return
fi
readonly __LOGGING_SH_INCLUDED=1

#===================================================================================================
# Constants
#===================================================================================================

# Colors
readonly RED='\033[0;31m'    # Red
readonly GREEN='\033[0;32m'  # Green
readonly YELLOW='\033[0;33m' # Yellow
readonly NC='\033[0m'        # No Color

#==================================================================================================
# Functions
#==================================================================================================

#
# Description
#
#   Prints an error message on stderr.
#
# Arguments
#
#   $1 - The error message to print.
#
# Usage Example
#
#   print_error "Print an error message."
#
print_error() {
    echo -e "${RED}[ERROR] ${1}${NC}" >&2
}

#
# Description
#
#   Prints a success message on stdout.
#
# Arguments
#
#   $1 - The success message to print.
#
# Usage Example
#
#   print_success "Print a success message."
#
print_success() {
    echo -e "${GREEN}[SUCCESS] ${1}${NC}"
}

#
# Description
#
#   Prints a message on stdout.
#
# Arguments
#
#   $1 - The message to print.
#
# Usage Example
#
#   print_info "Print an informational message."
#
print_info() {
    echo -e "[INFO] ${1}"
}

#
# Description
#
#   Prints a warning message on stderr.
#
# Arguments
#
#   $1 - The warning message to print.
#
# Usage Example
#
#   print_warning "Print a warning message."
#
print_warning() {
    echo -e "${YELLOW}[WARN] ${1}${NC}" >&2
}
