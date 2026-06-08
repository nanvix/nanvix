// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::WorkerThreadError,
    linux::{
        fcntl,
        unistd,
    },
    message::{
        RequestAssemblerTrait,
        RequestAssemblerType,
    },
    syscalls::SyscallTable,
};
use ::anyhow::Result;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::syscall::{
    fcntl::message::{
        OpenAtRequest,
        RenameAtRequest,
        UnlinkAtRequest,
    },
    message::{
        SystemCallLongMessage,
        SystemCallMessagePart,
    },
    poll::message::PollRequest,
    sys::stat::message::{
        FileChmodAtRequest,
        FileStatAtRequest,
        MakeDirectoryAtRequest,
        UpdateFileAccessTimeAtRequest,
    },
    unistd::message::{
        ChangeDirectoryRequest,
        FileAccessAtRequest,
        FileChownAtRequest,
        LinkAtRequest,
        ReadLinkAtRequest,
        SymbolicLinkAtRequest,
    },
};

//==================================================================================================
// Implementations
//==================================================================================================

impl<T> RequestAssemblerTrait<T> for FileStatAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileStatAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_fstat_at(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for SymbolicLinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::SymbolicLinkAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_symlinkat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for LinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::LinkAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        unistd::do_linkat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for ReadLinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::ReadLinkAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_readlinkat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for MakeDirectoryAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::MakeDirectoryAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => {
                Ok(assembler.add_part(part)?)
            },
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_mkdirat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for UpdateFileAccessTimeAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::UpdateFileAccessTimeAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::UpdateFileAccessTimeAtRequest(assembler) => {
                Ok(assembler.add_part(part)?)
            },
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::UpdateFileAccessTimeAtRequest(assembler) => {
                Ok(assembler.is_complete())
            },
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::UpdateFileAccessTimeAtRequest(assembler) => {
                assembler.take_parts()
            },
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_utimensat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for FileChownAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileChownAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_fchownat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for FileChmodAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileChmodAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_fchmodat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for OpenAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::OpenAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::OpenAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::OpenAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::OpenAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_openat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for RenameAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::RenameAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::RenameAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::RenameAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::RenameAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_renameat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for UnlinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::UnlinkAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::UnlinkAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::UnlinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::UnlinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        fcntl::do_unlinkat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for ChangeDirectoryRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::ChangeDirectoryRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::ChangeDirectoryRequest(assembler) => {
                Ok(assembler.add_part(part)?)
            },
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::ChangeDirectoryRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::ChangeDirectoryRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        unistd::do_chdir(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for FileAccessAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileAccessAtRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::FileAccessAtRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileAccessAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::FileAccessAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        unistd::do_faccessat(syscall_table, source, request)
    }
}

impl<T> RequestAssemblerTrait<T> for PollRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::PollRequest(
            SystemCallLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: SystemCallMessagePart,
    ) -> Result<(), WorkerThreadError> {
        match assembler {
            RequestAssemblerType::PollRequest(assembler) => Ok(assembler.add_part(part)?),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type").into()),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::PollRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<SystemCallMessagePart> {
        match assembler {
            RequestAssemblerType::PollRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        crate::linux::poll::do_poll(syscall_table, source, request)
    }
}
