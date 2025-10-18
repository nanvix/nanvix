# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

ifneq ($(strip $(filter yes,$(BUILD_OPT))),)

include build/make/optional/openblas.mk
include build/make/optional/openssl.mk
include build/make/optional/python.mk
include build/make/optional/sqlite.mk
include build/make/optional/zlib.mk
include build/make/optional/quickjs.mk

all-opt: init all-openblas all-openssl all-python all-quickjs all-sqlite all-zlib

clean-opt: clean-openblas clean-openssl clean-python clean-quickjs clean-sqlite clean-zlib

init-opt: init-openblas init-openssl init-python init-quickjs init-sqlite init-zlib

else

all-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"
clean-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"
init-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"

endif
