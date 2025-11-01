// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::WorkerThreadError,
    syscalls::SyscallTable,
};
use ::alloc::collections::BTreeMap;
use ::sys::{
    error::Error,
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::syscall::message::{
    LinuxDaemonLongMessage,
    LinuxDaemonMessagePart,
    MessagePartitioner,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default)]
pub struct RequestAssembler {
    inflight: BTreeMap<ThreadIdentifier, RequestAssemblerType>,
}

impl RequestAssembler {
    pub fn process_message<S, T: RequestAssemblerTrait<S>>(
        &mut self,
        syscall_table: &SyscallTable<S>,
        source: ThreadIdentifier,
        part: LinuxDaemonMessagePart,
    ) -> Result<Option<Vec<Message>>, WorkerThreadError> {
        match self.process_message_internal::<S, T>(syscall_table, source, part) {
            Ok(messages) => Ok(messages),
            Err(WorkerThreadError::Interrupted) => Err(WorkerThreadError::Interrupted),
            Err(WorkerThreadError::Error(e)) => {
                self.inflight.remove(&source);
                Err(WorkerThreadError::Error(e))
            },
        }
    }

    fn process_message_internal<S, T: RequestAssemblerTrait<S>>(
        &mut self,
        syscall_table: &SyscallTable<S>,
        source: ThreadIdentifier,
        part: LinuxDaemonMessagePart,
    ) -> Result<Option<Vec<Message>>, WorkerThreadError> {
        let message_complete: bool = {
            match self.assemble_parts::<S, T>(source, part) {
                Ok(message_complete) => message_complete,
                Err(e) => {
                    return Err(e);
                },
            }
        };

        if !message_complete {
            return Ok(None);
        }

        match self.process_request::<S, T>(syscall_table, source) {
            Ok(messages) => Ok(Some(messages)),
            Err(e) => Err(e),
        }
    }

    fn assemble_parts<S, T: RequestAssemblerTrait<S>>(
        &mut self,
        source: ThreadIdentifier,
        part: LinuxDaemonMessagePart,
    ) -> Result<bool, WorkerThreadError> {
        let assembler: &mut RequestAssemblerType = self
            .inflight
            .entry(source)
            .or_insert_with(|| T::new_assembler());
        T::add_part(assembler, part)?;
        Ok(T::is_complete(assembler)?)
    }

    fn process_request<S, T: RequestAssemblerTrait<S>>(
        &mut self,
        syscall_table: &SyscallTable<S>,
        source: ThreadIdentifier,
    ) -> Result<Vec<Message>, WorkerThreadError> {
        let assembler: RequestAssemblerType = self
            .inflight
            .remove(&source)
            .expect("inflight request does exist");

        let parts: Vec<LinuxDaemonMessagePart> = T::take_parts(assembler);
        let request: T = T::from_parts(&parts)?;
        T::process_request(syscall_table, source, request)
    }
}

#[allow(clippy::enum_variant_names)]
pub enum RequestAssemblerType {
    FileStatAtRequest(LinuxDaemonLongMessage),
    SymbolicLinkAtRequest(LinuxDaemonLongMessage),
    LinkAtRequest(LinuxDaemonLongMessage),
    ReadLinkAtRequest(LinuxDaemonLongMessage),
    MakeDirectoryAtRequest(LinuxDaemonLongMessage),
    UpdateFileAccessTimeAtRequest(LinuxDaemonLongMessage),
    FileChownAtRequest(LinuxDaemonLongMessage),
    FileChmodAtRequest(LinuxDaemonLongMessage),
    OpenAtRequest(LinuxDaemonLongMessage),
    RenameAtRequest(LinuxDaemonLongMessage),
    UnlinkAtRequest(LinuxDaemonLongMessage),
    ChangeDirectoryRequest(LinuxDaemonLongMessage),
    FileAccessAtRequest(LinuxDaemonLongMessage),
    PollRequest(LinuxDaemonLongMessage),
}

pub trait RequestAssemblerTrait<T>
where
    Self: Sized,
    Self: MessagePartitioner,
{
    fn new_assembler() -> RequestAssemblerType;

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), WorkerThreadError>;

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error>;

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart>;

    fn process_request(
        syscall_table: &SyscallTable<T>,
        source: ThreadIdentifier,
        request: Self,
    ) -> Result<Vec<Message>, WorkerThreadError>;
}
