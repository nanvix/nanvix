# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

layout split
target remote tcp::1234
file bin/kernel.elf
symbol-file bin/kernel.elf
handle SIGSEGV nostop noprint nopass
set confirm off
focus cmd
set detach-on-fork
b _do_start
b kmain

define hook-stop
	if $_isvoid ($_exitcode) != 1
		quit
	end

	focus cmd
end
