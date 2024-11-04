// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::BTreeMap;
use ::linuxd::message::{
    LinuxDaemonLongMessage,
    LinuxDaemonMessagePart,
    MessagePartitioner,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::Error,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default)]
pub struct RequestAssembler {
    inflight: BTreeMap<ProcessIdentifier, RequestAssemblerType>,
}

impl RequestAssembler {
    pub fn process_message<T: RequestAssemblerTrait>(
        &mut self,
        source: ProcessIdentifier,
        part: LinuxDaemonMessagePart,
    ) -> Result<Option<Vec<Message>>, Error> {
        match self.process_message_internal::<T>(source, part) {
            Ok(messages) => Ok(messages),
            Err(e) => {
                self.inflight.remove(&source);
                Err(e)
            },
        }
    }

    fn process_message_internal<T: RequestAssemblerTrait>(
        &mut self,
        source: ProcessIdentifier,
        part: LinuxDaemonMessagePart,
    ) -> Result<Option<Vec<Message>>, Error> {
        let message_complete: bool = {
            match self.assemble_parts::<T>(source, part) {
                Ok(message_complete) => message_complete,
                Err(e) => {
                    return Err(e);
                },
            }
        };

        if !message_complete {
            return Ok(None);
        }

        match self.process_request::<T>(source) {
            Ok(messages) => Ok(Some(messages)),
            Err(e) => Err(e),
        }
    }

    fn assemble_parts<T: RequestAssemblerTrait>(
        &mut self,
        source: ProcessIdentifier,
        part: LinuxDaemonMessagePart,
    ) -> Result<bool, Error> {
        let assembler: &mut RequestAssemblerType = self
            .inflight
            .entry(source)
            .or_insert_with(|| T::new_assembler());
        T::add_part(assembler, part)?;
        T::is_complete(assembler)
    }

    fn process_request<T: RequestAssemblerTrait>(
        &mut self,
        source: ProcessIdentifier,
    ) -> Result<Vec<Message>, Error> {
        let assembler: RequestAssemblerType = self
            .inflight
            .remove(&source)
            .expect("inflight request does exist");

        let parts: Vec<LinuxDaemonMessagePart> = T::take_parts(assembler);
        let request: T = T::from_parts(&parts)?;
        Ok(T::process_request(source, request))
    }
}

#[allow(clippy::enum_variant_names)]
pub enum RequestAssemblerType {
    FileStatAtRequest(LinuxDaemonLongMessage),
    SymbolicLinkAtRequest(LinuxDaemonLongMessage),
    LinkAtRequest(LinuxDaemonLongMessage),
}

pub trait RequestAssemblerTrait
where
    Self: Sized,
    Self: MessagePartitioner,
{
    fn new_assembler() -> RequestAssemblerType;

    fn add_part(
        assembler: &mut RequestAssemblerType,
        part: LinuxDaemonMessagePart,
    ) -> Result<(), Error>;

    fn is_complete(assembler: &RequestAssemblerType) -> Result<bool, Error>;

    fn take_parts(assembler: RequestAssemblerType) -> Vec<LinuxDaemonMessagePart>;

    fn process_request(source: ProcessIdentifier, request: Self) -> Vec<Message>;
}
