// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores the information about the execution context of a thread.
///
#[derive(Default)]
#[repr(C, packed)]
pub struct ContextInformation {
    esp0: u32,
    cr3: u32,
    gs: u32,
    fs: u32,
    es: u32,
    ds: u32,
    edi: u32,
    esi: u32,
    ebp: u32,
    edx: u32,
    ecx: u32,
    ebx: u32,
    eax: u32,
    err: u32,
    eip: u32,
    cs: u32,
    eflags: u32,
    esp: u32,
    ss: u32,
}

// `Context` must be 72 bytes long. This must match low-level assembly dispatcher code.
static_assert_size!(ContextInformation, 76);

//==================================================================================================
// Implementations
//==================================================================================================

impl ContextInformation {
    pub fn new(cr3: u32, esp: u32, esp0: u32) -> Self {
        Self {
            esp0,
            cr3,
            esp,
            ..Default::default()
        }
    }

    pub unsafe fn switch(from: *mut ContextInformation, to: *mut ContextInformation) {
        extern "C" {
            pub fn __context_switch(from: *mut ContextInformation, to: *mut ContextInformation);
        }

        __context_switch(from, to)
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl core::fmt::Debug for ContextInformation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Copy fields to local variables.
        let esp0: u32 = self.esp0;
        let cr3: u32 = self.cr3;
        let gs: u32 = self.gs;
        let fs: u32 = self.fs;
        let es: u32 = self.es;
        let ds: u32 = self.ds;
        let edi: u32 = self.edi;
        let esi: u32 = self.esi;
        let ebp: u32 = self.ebp;
        let edx: u32 = self.edx;
        let ecx: u32 = self.ecx;
        let ebx: u32 = self.ebx;
        let eax: u32 = self.eax;
        let err: u32 = self.err;
        let eip: u32 = self.eip;
        let cs: u32 = self.cs;
        let eflags: u32 = self.eflags;
        let esp: u32 = self.esp;
        let ss: u32 = self.ss;

        write!(
            f,
            "esp0={:#010x}, cr3={:#010x}, gs={:#010x}, fs={:#010x}, es={:#010x}, ds={:#010x}, \
             edi={:#010x}, esi={:#010x}, ebp={:#010x}, edx={:#010x}, ecx={:#010x}, ebx={:#010x}, \
             eax={:#010x}, err={:#010x}, eip={:#010x}, cs={:#010x}, eflags={:#010x}, \
             esp={:#010x}, ss={:#010x}",
            esp0,
            cr3,
            gs,
            fs,
            es,
            ds,
            edi,
            esi,
            ebp,
            edx,
            ecx,
            ebx,
            eax,
            err,
            eip,
            cs,
            eflags,
            esp,
            ss
        )
    }
}
