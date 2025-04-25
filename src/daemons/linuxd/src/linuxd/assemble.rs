// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl,
    message::{
        RequestAssemblerTrait,
        RequestAssemblerType,
    },
    unistd,
};
use ::anyhow::Result;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};
use ::posix::{
    fcntl::message::{
        FileChmodAtRequest,
        FileChownAtRequest,
        OpenAtRequest,
        RenameAtRequest,
        UnlinkAtRequest,
    },
    message::{
        LinuxDaemonLongMessage,
        LinuxDaemonMessagePart,
    },
    sys::stat::message::{
        FileStatAtRequest,
        MakeDirectoryAtRequest,
        UpdateFileAccessTimeAtRequest,
    },
    unistd::message::{
        LinkAtRequest,
        ReadLinkAtRequest,
        SymbolicLinkAtRequest,
    },
};

//==================================================================================================
// Implementations
//==================================================================================================

impl RequestAssemblerTrait for FileStatAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileStatAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::FileStatAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_fstat_at(source, request)
    }
}

impl RequestAssemblerTrait for SymbolicLinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::SymbolicLinkAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::SymbolicLinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_symlinkat(source, request)
    }
}

impl RequestAssemblerTrait for LinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        debug!("creating linkat request assembler");
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::LinkAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        debug!("adding part to linkat request");
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        debug!("checking if linkat request is complete");
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        debug!("taking parts from linkat request");
        match assembler {
            RequestAssemblerType::LinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        unistd::do_linkat(source, request)
    }
}

impl RequestAssemblerTrait for ReadLinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::ReadLinkAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::ReadLinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_readlinkat(source, request)
    }
}

impl RequestAssemblerTrait for MakeDirectoryAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::MakeDirectoryAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::MakeDirectoryAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_mkdirat(source, request)
    }
}

impl RequestAssemblerTrait for UpdateFileAccessTimeAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::UpdateFileAccessTimeAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::UpdateFileAccessTimeAtRequest(assembler) => {
                assembler.add_part(part)
            },
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
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

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::UpdateFileAccessTimeAtRequest(assembler) => {
                assembler.take_parts()
            },
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_utimensat(source, request)
    }
}

impl RequestAssemblerTrait for FileChownAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileChownAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::FileChownAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_fchownat(source, request)
    }
}

impl RequestAssemblerTrait for FileChmodAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::FileChmodAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::FileChmodAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_fchmodat(source, request)
    }
}

impl RequestAssemblerTrait for OpenAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::OpenAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::OpenAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::OpenAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::OpenAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_openat(source, request)
    }
}

impl RequestAssemblerTrait for RenameAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::RenameAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::RenameAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::RenameAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::RenameAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_renameat(source, request)
    }
}

impl RequestAssemblerTrait for UnlinkAtRequest {
    fn new_assembler() -> RequestAssemblerType {
        let capacity: usize = Self::MAX_SIZE.div_ceil(LinuxDaemonMessagePart::PAYLOAD_SIZE);
        RequestAssemblerType::UnlinkAtRequest(
            LinuxDaemonLongMessage::new(capacity).expect("capacity is set to a valid value"),
        )
    }

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error> {
        match assembler {
            RequestAssemblerType::UnlinkAtRequest(assembler) => assembler.add_part(part),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error> {
        match assembler {
            RequestAssemblerType::UnlinkAtRequest(assembler) => Ok(assembler.is_complete()),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid assembler type")),
        }
    }

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart> {
        match assembler {
            RequestAssemblerType::UnlinkAtRequest(assembler) => assembler.take_parts(),
            _ => unreachable!("invalid assembler type"),
        }
    }

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message> {
        fcntl::do_unlinkat(source, request)
    }
}
