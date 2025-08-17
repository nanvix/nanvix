// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pal::fs::{
        OpenOptions,
        Path,
    },
    wasi::{
        types::{
            Errno,
            Fd,
            FdFlags,
            LookupFlags,
            OpenFlags,
            Rights,
        },
        WasiCtxInner,
    },
};

//==================================================================================================
// Implementations
//==================================================================================================

impl WasiCtxInner {
    /// Opens a file or a directory.
    #[allow(clippy::too_many_arguments)]
    pub fn path_open(
        &mut self,
        dirfd: Fd,
        dirflags: &LookupFlags,
        pathname: &str,
        oflags: OpenFlags,
        fs_rights_base: Rights,
        fs_rights_inheriting: Rights,
        fdflags: FdFlags,
    ) -> Result<Fd, Errno> {
        let mut flags: OpenOptions = OpenOptions::new();

        match self.get_file(dirfd) {
            Some(dirfd) => {
                // Ensure that we have the right to invoke this operation.
                if !dirfd.rights_base().path_open {
                    ::syslog::error!("path_open(): access denied");
                    return Err(Errno::AccessDenied);
                }

                // Set creation flag.
                if oflags.creat {
                    flags.create(true);
                }

                // Set directory flag.
                if oflags.directory {
                    unimplemented!("path_open(): directory flag not supported");
                }

                // Set exclusive flag.
                if oflags.excl {
                    flags.create_new(true);
                }

                // Set truncation flag.
                if oflags.trunc {
                    flags.truncate(true);
                }

                // Set append mode.
                if fdflags.append {
                    flags.append(true);
                }

                // Set dsync mode.
                if fdflags.dsync {
                    ::syslog::error!("path_open(): dsync mode not supported");
                }

                // Set non-blocking mode.
                if fdflags.nonblock {
                    ::syslog::error!("path_open(): non-blocking mode not supported");
                }

                // Set rsync mode.
                if fdflags.rsync {
                    ::syslog::error!("path_open(): rsync mode not supported");
                }

                // Set sync mode.
                if fdflags.sync {
                    ::syslog::error!("path_open(): sync mode not supported");
                }

                // Set symlink follow flag.
                if !dirflags.symlink_follow {
                    ::syslog::error!("path_open(): symlink follow flag not supported");
                }

                // Set read mode.
                if fs_rights_base.fd_read {
                    flags.read(true);
                }

                // Set write mode.
                if fs_rights_base.fd_write {
                    flags.write(true);
                }

                match flags.openat(Some(dirfd.file()), &Path::new(pathname)) {
                    Ok(file) => {
                        let fd: Fd = self.insert_file(file, fs_rights_base, fs_rights_inheriting);
                        Ok(fd)
                    },
                    Err(e) => Err(e.value().into()),
                }
            },
            None => {
                ::syslog::error!("path_open(): invalid file descriptor (fd={:?})", dirfd);
                Err(Errno::Badf)
            },
        }
    }
}
