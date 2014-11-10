# 
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com> 
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

# Conflicts.
.PHONY: cat
.PHONY: chgrp
.PHONY: chmod
.PHONY: chown
.PHONY: cp
.PHONY: echo
.PHONY: kill
.PHONY: ln
.PHONY: login
.PHONY: ls
.PHONY: mkdir
.PHONY: mv
.PHONY: nice
.PHONY: pwd
.PHONY: rm
.PHONY: rmdir
.PHONY: stat
.PHONY: sync
.PHONY: tsh

# Builds everything.
all: ubin documentation

# Builds user utilities.
ubin: cat chgrp chmod chown cp echo kill ln login ls mv nice pwd rm stat sync \
	  tsh

# Builds user utilities documentation.
documentation: cat-doc chgrp-doc chmod-doc chown-doc cp-doc echo-doc kill-doc  \
               ln-doc login-doc ls-doc mv-doc nice-doc pwd-doc rm-doc stat-doc \
               sync-doc tsh-doc

# Builds cat.
cat: 
	$(CC) $(CFLAGS) $(LDFLAGS) cat/*.c -o $(UBINDIR)/cat $(LIBDIR)/libc.a
	
# Builds cat documentation.
cat-doc:
	@cp -f cat/*.1 $(DOCDIR)/man/1

# Builds chgrp.
chgrp: 
	$(CC) $(CFLAGS) $(LDFLAGS) chgrp/*.c -o $(UBINDIR)/chgrp $(LIBDIR)/libc.a
	
# Builds chgrp documentation.
chgrp-doc:
	@cp -f chgrp/*.1 $(DOCDIR)/man/1
	
# Builds chmod.
chmod: 
	$(CC) $(CFLAGS) $(LDFLAGS) chmod/*.c -o $(UBINDIR)/chmod $(LIBDIR)/libc.a
	
# Builds chmod documentation.
chmod-doc:
	@cp -f chmod/*.1 $(DOCDIR)/man/1
	
# Builds chown.
chown: 
	$(CC) $(CFLAGS) $(LDFLAGS) chown/*.c -o $(UBINDIR)/chown $(LIBDIR)/libc.a
	
# Builds chown documentation.
chown-doc:
	@cp -f chown/*.1 $(DOCDIR)/man/1
	
# Builds cp.
cp: 
	$(CC) $(CFLAGS) $(LDFLAGS) cp/*.c -o $(UBINDIR)/cp $(LIBDIR)/libc.a
	
# Builds cp documentation.
cp-doc:
	@cp -f cp/*.1 $(DOCDIR)/man/1
	
# Builds echo.
echo: 
	$(CC) $(CFLAGS) $(LDFLAGS) echo/*.c -o $(UBINDIR)/echo $(LIBDIR)/libc.a
	
# Builds echo documentation.
echo-doc:
	@cp -f echo/*.1 $(DOCDIR)/man/1
	
# Builds kill.
kill: 
	$(CC) $(CFLAGS) $(LDFLAGS) kill/*.c -o $(UBINDIR)/kill $(LIBDIR)/libc.a
	
# Builds kill documentation.
kill-doc:
	@cp -f kill/*.1 $(DOCDIR)/man/1
	
# Builds ln.
ln: 
	$(CC) $(CFLAGS) $(LDFLAGS) ln/*.c -o $(UBINDIR)/ln $(LIBDIR)/libc.a
	
# Builds ln documentation.
ln-doc:
	@cp -f ln/*.1 $(DOCDIR)/man/1
	
# Builds login.
login: 
	$(CC) $(CFLAGS) $(LDFLAGS) login/*.c -o $(UBINDIR)/login $(LIBDIR)/libc.a
	
# Builds login documentation.
login-doc:
	
# Builds ls.
ls: 
	$(CC) $(CFLAGS) $(LDFLAGS) ls/*.c -o $(UBINDIR)/ls $(LIBDIR)/libc.a
	
# Builds ls documentation.
ls-doc:
	@cp -f ls/*.1 $(DOCDIR)/man/1
	
# Builds mkdir.
mkdir: 
	$(CC) $(CFLAGS) $(LDFLAGS) mkdir/*.c -o $(UBINDIR)/mkdir $(LIBDIR)/libc.a
	
# Builds mkdir documentation.
mkdir-doc:
	@cp -f mkdir/*.1 $(DOCDIR)/man/1
	
# Builds mv.
mv: 
	$(CC) $(CFLAGS) $(LDFLAGS) mv/*.c -o $(UBINDIR)/mv $(LIBDIR)/libc.a
	
# Builds mv documentation.
mv-doc:
	@cp -f mv/*.1 $(DOCDIR)/man/1
	
# Builds nice.
nice: 
	$(CC) $(CFLAGS) $(LDFLAGS) nice/*.c -o $(UBINDIR)/nice $(LIBDIR)/libc.a
	
# Builds nice documentation.
nice-doc:
	@cp -f nice/*.1 $(DOCDIR)/man/1
	
# Builds pwd.
pwd: 
	$(CC) $(CFLAGS) $(LDFLAGS) pwd/*.c -o $(UBINDIR)/pwd $(LIBDIR)/libc.a
	
# Builds pwd documentation.
pwd-doc:
	@cp -f pwd/*.1 $(DOCDIR)/man/1
	
# Builds rm.
rm: 
	$(CC) $(CFLAGS) $(LDFLAGS) rm/*.c -o $(UBINDIR)/rm $(LIBDIR)/libc.a
	
# Builds rm documentation.
rm-doc:
	@cp -f rm/*.1 $(DOCDIR)/man/1
	
# Builds rmdir.
rmdir: 
	$(CC) $(CFLAGS) $(LDFLAGS) rmdir/*.c -o $(UBINDIR)/rmdir $(LIBDIR)/libc.a
	
# Builds rmdir documentation.
rmdir-doc:
	@cp -f rmdir/*.1 $(DOCDIR)/man/1
	
# Builds stat.
stat: 
	$(CC) $(CFLAGS) $(LDFLAGS) stat/*.c -o $(UBINDIR)/stat $(LIBDIR)/libc.a
	
# Builds stat documentation.
stat-doc:
	@cp -f stat/*.1 $(DOCDIR)/man/1
	
# Builds sync.
sync: 
	$(CC) $(CFLAGS) $(LDFLAGS) sync/*.c -o $(UBINDIR)/sync $(LIBDIR)/libc.a
	
# Builds sync documentation.
sync-doc:
	@cp -f sync/*.1 $(DOCDIR)/man/1
	
# Builds tsh.
tsh: 
	$(CC) $(CFLAGS) $(LDFLAGS) tsh/*.c -o $(UBINDIR)/tsh $(LIBDIR)/libc.a
	
# Builds tsh documentation.
tsh-doc:
	@cp -f tsh/*.1 $(DOCDIR)/man/1

# Clean compilation files.
clean:
	@rm -f $(UBINDIR)/cat
	@rm -f $(UBINDIR)/chgrp
	@rm -f $(UBINDIR)/chmod
	@rm -f $(UBINDIR)/chown
	@rm -f $(UBINDIR)/cp
	@rm -f $(UBINDIR)/echo
	@rm -f $(UBINDIR)/kill
	@rm -f $(UBINDIR)/ln
	@rm -f $(UBINDIR)/login
	@rm -f $(UBINDIR)/ls
	@rm -f $(UBINDIR)/mkdir
	@rm -f $(UBINDIR)/mv
	@rm -f $(UBINDIR)/nice
	@rm -f $(UBINDIR)/pwd
	@rm -f $(UBINDIR)/rm
	@rm -f $(UBINDIR)/rmdir
	@rm -f $(UBINDIR)/stat
	@rm -f $(UBINDIR)/sync
	@rm -f $(UBINDIR)/tsh
