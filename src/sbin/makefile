# 
# Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com> 
#
# This file is part of Nanvix.
#
# Nanvix is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Nanvix is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Nanvix.  If not, see <http://www.gnu.org/licenses/>.
#

# Resolves conflicts
.PHONY: foobar
.PHONY: init
.PHONY: shutdown
.PHONY: test

# Builds everything.
all: foobar init shutdown test

# Builds foobar.
foobar:
	$(CC) $(CFLAGS) $(LDFLAGS) foobar/*.c -o $(SBINDIR)/foobar $(LIBDIR)/libc.a

# Builds init.
init:
	$(CC) $(CFLAGS) $(LDFLAGS) init/*.c -o $(SBINDIR)/init $(LIBDIR)/libc.a

# Builds shutdown.
shutdown:
	$(CC) $(CFLAGS) $(LDFLAGS) shutdown/*.c -o $(SBINDIR)/shutdown $(LIBDIR)/libc.a

# Builds test.
test:
	$(CC) $(CFLAGS) $(LDFLAGS) test/*.c -o $(SBINDIR)/test $(LIBDIR)/libc.a
	
# Cleans compilations files.
clean:
	@rm -f $(SBINDIR)/foobar
	@rm -f $(SBINDIR)/init
	@rm -f $(SBINDIR)/shutdown
	@rm -f $(SBINDIR)/test
	
