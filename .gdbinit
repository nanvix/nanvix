# Copyright(c) 2011-2023 The Maintainers of Nanvix.
# Licensed under the MIT License.

layout split
target remote tcp::1234
file bin/kernel
symbol-file bin/kernel
handle SIGSEGV nostop noprint nopass
set confirm off
focus cmd
set detach-on-fork
b kmain

define hook-stop
	if $_isvoid ($_exitcode) != 1
		quit
	end

	focus cmd
end
