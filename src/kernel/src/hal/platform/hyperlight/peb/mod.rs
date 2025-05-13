// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(non_snake_case)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//==================================================================================================
// Modules
//==================================================================================================

mod input;
mod output;

//==================================================================================================
// Imports
//==================================================================================================

use self::{
    input::InputData,
    output::OutputData,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::hyperlight_common::flatbuffer_wrappers::{
    function_call::{
        FunctionCall,
        FunctionCallType,
    },
    function_types::{
        ParameterValue,
        ReturnType,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

pub enum OutBAction {
    _Log = 99,
    CallFunction = 101,
    _Abort = 102,
    Magic = 103,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct HostFunctionDefinitions {
    pub fbHostFunctionDetailsSize: u64,
    pub fbHostFunctionDetails: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct HostException {
    pub hostExceptionSize: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct GuestErrorData {
    pub guestErrorSize: u64,
    pub guestErrorBuffer: u64,
}

#[repr(u64)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum RunMode {
    _None = 0,
    _Hypervisor = 1,
    _InProcessWindows = 2,
    _InProcessLinux = 3,
    _Invalid = 4,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct GuestHeapData {
    pub guestHeapSize: u64,
    pub guestHeapBuffer: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct GuestStackData {
    pub minUserStackAddress: u64,
    pub userStackAddress: u64,
    pub kernelStackAddress: u64,
    pub bootStackAddress: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct GuestPanicContextData {
    pub guestPanicContextDataSize: u64,
    pub guestPanicContextDataBuffer: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct HyperlightPEB {
    security_cookie_seed: u64,
    guest_function_dispatch_ptr: u64,
    hostFunctionDefinitions: HostFunctionDefinitions,
    hostException: HostException,
    guestErrorData: GuestErrorData,
    pCode: u64,
    pOutb: u64,
    pOutbContext: u64,
    runMode: RunMode,
    inputdata: InputData,
    outputdata: OutputData,
    guestPanicContextData: GuestPanicContextData,
    guestheapData: GuestHeapData,
    gueststackData: GuestStackData,
}

pub struct ProcessEnvironmentBlock(pub *mut HyperlightPEB);

//==================================================================================================
// Global Variables
//==================================================================================================

static mut PEB_PTR: Option<*mut HyperlightPEB> = None;

//==================================================================================================
// Implementations
//==================================================================================================

impl core::fmt::Debug for ProcessEnvironmentBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "ProcessEnvironmentBlock({:#x?})", unsafe { &*self.0 })
    }
}

impl ProcessEnvironmentBlock {
    pub unsafe fn init(peb_base: *mut HyperlightPEB) -> Result<(), Error> {
        if PEB_PTR.is_some() {
            Err(Error::new(ErrorCode::ResourceBusy, "peb already initialized"))
        } else {
            PEB_PTR = Some(peb_base);
            Ok(())
        }
    }

    pub unsafe fn set_guest_function_dispatch_ptr(ptr: u64) -> Result<(), Error> {
        match PEB_PTR {
            Some(peb_ptr) => {
                (*peb_ptr).guest_function_dispatch_ptr = ptr;
                Ok(())
            },
            None => Err(Error::new(ErrorCode::NoSuchDevice, "peb not initialized")),
        }
    }

    pub unsafe fn puts(message: &str) -> Result<(), Error> {
        match PEB_PTR {
            Some(peb_ptr) => {
                let mut peb = Self(peb_ptr);
                peb.print(message)
            },
            None => Err(Error::new(ErrorCode::NoSuchDevice, "peb not initialized")),
        }
    }

    pub unsafe fn vmbus_write(data: &[u8]) -> Result<(), Error> {
        match PEB_PTR {
            Some(peb_ptr) => {
                let mut peb = Self(peb_ptr);
                peb.do_vmbus_write_with_host_fn(data)
            },
            None => Err(Error::new(ErrorCode::NoSuchDevice, "peb not initialized")),
        }
    }

    pub unsafe fn vmbus_read() -> Result<Vec<u8>, Error> {
        match PEB_PTR {
            Some(peb_ptr) => {
                let mut peb = Self(peb_ptr);
                peb.do_vmbus_read_with_host_fn()
            },
            None => Err(Error::new(ErrorCode::NoSuchDevice, "peb not initialized")),
        }
    }

    fn print(&mut self, message: &str) -> Result<(), Error> {
        self.print_with_magic_port(message)
        // self.print_with_host_fn(message)
    }

    #[allow(dead_code)]
    fn print_with_magic_port(&mut self, message: &str) -> Result<(), Error> {
        for byte in message.bytes() {
            unsafe {
                ::arch::io::out8(OutBAction::Magic as u16, byte);
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn print_with_host_fn(&mut self, message: &str) -> Result<(), Error> {
        self.call_host_function(
            "HostPrint",
            Some(Vec::from(&[ParameterValue::String(message.to_string())])),
            ReturnType::Int,
        )?;

        let message_len = message.len();
        let count = self.get_host_value_return_as_int()?;
        if count != message_len as i32 {
            Err(Error::new(ErrorCode::IoErr, "failed to print message"))
        } else {
            Ok(())
        }
    }

    fn do_vmbus_write_with_host_fn(&mut self, data: &[u8]) -> Result<(), Error> {
        self.call_host_function(
            "VmbusWrite",
            Some(Vec::from(&[ParameterValue::VecBytes(Vec::from(data))])),
            ReturnType::Int,
        )?;

        let count = self.get_host_value_return_as_int()?;
        if count != data.len() as i32 {
            Err(Error::new(ErrorCode::IoErr, "failed to write data"))
        } else {
            Ok(())
        }
    }

    fn do_vmbus_read_with_host_fn(&mut self) -> Result<Vec<u8>, Error> {
        self.call_host_function("VmbusRead", None, ReturnType::VecBytes)?;

        let data = self.pop_shared_input_data()?;
        Ok(data)
    }

    fn push_shared_output_data(&mut self, data: Vec<u8>) -> Result<(), Error> {
        unsafe { &mut (*self.0).outputdata }.write(data)
    }

    fn get_host_value_return_as_int(&self) -> Result<i32, Error> {
        unsafe { &(*self.0).inputdata }.get_host_value_return_as_int()
    }

    fn pop_shared_input_data(&mut self) -> Result<Vec<u8>, Error> {
        unsafe { &mut (*self.0).inputdata }.get_host_value_return_as_vecbytes()
    }

    fn call_host_function(
        &mut self,
        function_name: &str,
        parameters: Option<Vec<ParameterValue>>,
        return_type: ReturnType,
    ) -> Result<(), Error> {
        let host_function_call = FunctionCall::new(
            function_name.to_string(),
            parameters,
            FunctionCallType::Host,
            return_type,
        );

        // validate_host_function_call(&host_function_call)?;

        let host_function_call_buffer: Vec<u8> = match host_function_call.try_into() {
            Ok(buffer) => buffer,
            Err(_) => {
                return Err(Error::new(ErrorCode::IoErr, "unable to serialize host function call"));
            },
        };

        self.push_shared_output_data(host_function_call_buffer)?;

        unsafe {
            ::arch::io::out8(OutBAction::CallFunction as u16, 0);
        }

        Ok(())
    }
}
