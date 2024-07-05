/*
 * Copyright(c) 2011-2024 The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==============================================================================
// Imports
//==============================================================================

use nanvix::{
    memory::{
        self,
        FrameNumber,
        PageInfo,
        VirtualAddress,
        VirtualMemory,
        VmCtrlRequest,
    },
    security::AccessMode,
};

//==============================================================================
// Private Standalone Functions
//==============================================================================

/// Checks if sizes are conformant.
fn check_sizes() -> bool {
    if core::mem::size_of::<AccessMode>() != 4 {
        nanvix::log!("unexpected size for AccessMode");
        return false;
    }
    if core::mem::size_of::<VirtualMemory>() != 4 {
        nanvix::log!("unexpected size for VirtualMemory");
        return false;
    }
    if core::mem::size_of::<VirtualAddress>() != 4 {
        nanvix::log!("unexpected size for VirtualAddress");
        return false;
    }
    if core::mem::size_of::<memory::PhysicalAddress>() != 4 {
        nanvix::log!("unexpected size for PhysicalAddress");
        return false;
    }
    if core::mem::size_of::<memory::FrameNumber>() != 4 {
        nanvix::log!("unexpected size for FrameNumber");
        return false;
    }
    if core::mem::size_of::<PageInfo>() != 8 {
        nanvix::log!("unexpected size for PageInfo");
        return false;
    }

    true
}

/// Attempts to allocate and release page frame.
fn alloc_free_frame() -> bool {
    // Attempt to allocate a page frame.
    let frame: u32 = memory::fralloc();

    // Check if we failed to allocate a page frame.
    if frame == memory::NULL_FRAME {
        nanvix::log!("failed to allocate a page frame");
        return false;
    }

    // Check if the page frame lies on a valid range.
    if (frame * memory::PAGE_SIZE) < memory::USER_BASE_ADDRESS {
        nanvix::log!("succeded to allocate an invalid page frame");
        return false;
    }

    // Attempt to release the page frame.
    let result: u32 = memory::frfree(frame);

    // Check if we failed to release the page frame.
    if result != 0 {
        nanvix::log!("failed to release a valid page frame");
        return false;
    }

    true
}

/// Attempts release the null page frame.
fn free_null_frame() -> bool {
    // Attempt to release the null page frame.
    let result: u32 = memory::frfree(memory::NULL_FRAME);

    // Check if we succeeded to release the null page frame.
    if result == 0 {
        nanvix::log!("succeded to release null page frame");
        return false;
    }

    true
}

/// Attempts release an invalid page frame.
fn free_invalid_frame() -> bool {
    // Attempt to release an invalid page frame.
    let frame_addr: u32 = memory::KERNEL_BASE_ADDRESS;
    let result: u32 = memory::frfree(frame_addr / memory::PAGE_SIZE);

    // Check if we succeeded to release an invalid page frame.
    if result == 0 {
        nanvix::log!("succeded to release an invalid page frame");
        return false;
    }

    true
}

/// Attempts to release a page frame twice.
fn double_free_frame() -> bool {
    // Attempt to allocate a page frame.
    let frame: u32 = memory::fralloc();

    // Check if we failed to allocate a page frame.
    if frame == memory::NULL_FRAME {
        nanvix::log!("failed to allocate a page frame");
        return false;
    }

    // Attempt to release the page frame.
    let result: u32 = memory::frfree(frame);

    // Check if we failed to release the page frame.
    if result != 0 {
        nanvix::log!("failed to release a valid page frame");
        return false;
    }

    // Attempt to release the page frame again.
    let result: u32 = memory::frfree(frame);

    // Check if we succeeded to release the page frame again.
    if result == 0 {
        nanvix::log!("succeded to release a page frame twice");
        return false;
    }

    true
}

/// Attempts to create and release a virtual memory space.
fn create_remove_vmem() -> bool {
    // Attempt to create a virtual memory space.
    let vmem: VirtualMemory = memory::vmcreate();

    // Check if we failed to create a virtual memory space.
    if vmem == memory::NULL_VMEM {
        nanvix::log!("failed to create a virtual memory space");
        return false;
    }

    // Attempt to remove the virtual memory space.
    let result: u32 = memory::vmremove(vmem);

    // Check if we failed to remove the virtual memory space.
    if result != 0 {
        nanvix::log!("failed to remove a valid virtual memory space");
        return false;
    }

    true
}

/// Attempts to remove the null virtual memory space.
fn remove_null_vmem() -> bool {
    // Attempt to remove the null virtual memory space.
    let result: u32 = memory::vmremove(memory::NULL_VMEM);

    // Check if we succeeded to remove the null virtual memory space.
    if result == 0 {
        nanvix::log!("succeded to remove null virtual memory space");
        return false;
    }

    true
}

/// Attempts to map and unmap a page frame to a virtual memory space.
fn map_unmap_vmem() -> bool {
    // Attempt to create a virtual memory space.
    let vmem: VirtualMemory = memory::vmcreate();

    // Check if we failed to create a virtual memory space.
    if vmem == memory::NULL_VMEM {
        nanvix::log!("failed to create a virtual memory space");
        return false;
    }

    // Attempt to allocate a page frame.
    let frame: FrameNumber = memory::fralloc();

    // Check if we failed to allocate a page frame.
    if frame == memory::NULL_FRAME {
        nanvix::log!("failed to allocate a page frame");
        return false;
    }

    // Attempt to map the page frame to the virtual memory space.
    nanvix::log!("vmmap vmem={:?} frame=0x{:x?}", vmem, frame);
    let result: u32 = memory::vmmap(vmem, memory::USER_BASE_ADDRESS, frame);

    // Check if we failed to map the page frame to the virtual memory space.
    if result != 0 {
        nanvix::log!("failed to map a page frame to a virtual memory space");
        return false;
    }

    // Attempt to unmap the page frame from the virtual memory space.
    // nanvix::log!(
    //     "vmunmap vmem={:?} frame=0x{:x?}",
    //     vmem,
    //     memory::USER_BASE_ADDRESS
    // );
    let result: u32 = memory::vmunmap(vmem, memory::USER_BASE_ADDRESS);

    // Check if we failed to unmap the page frame from the virtual memory space.
    if result != frame {
        nanvix::log!(
            "failed to unmap a page frame from a virtual memory space {:?}",
            frame
        );
        return false;
    }

    // Attempt to remove the virtual memory space.
    let result: u32 = memory::vmremove(vmem);

    // Check if we failed to remove the virtual memory space.
    if result != 0 {
        nanvix::log!("failed to remove a valid virtual memory space");
        return false;
    }

    true
}

/// Attempts to change access permissions on page.
fn change_page_permissions() -> bool {
    // Attempt to create a virtual memory space.
    let vmem: VirtualMemory = memory::vmcreate();

    // Check if we failed to create a virtual memory space.
    if vmem == memory::NULL_VMEM {
        nanvix::log!("failed to create a virtual memory space");
        return false;
    }

    // Attempt to allocate a page frame.
    let frame: u32 = memory::fralloc();

    // Check if we failed to allocate a page frame.
    if frame == memory::NULL_FRAME {
        nanvix::log!("failed to allocate a page frame");
        return false;
    }

    // Attempt to map the page frame to the virtual memory space.
    let result: u32 = memory::vmmap(vmem, memory::USER_BASE_ADDRESS, frame);

    // Check if we failed to map the page frame to the virtual memory space.
    if result != 0 {
        nanvix::log!("failed to map a page frame to a virtual memory space");
        return false;
    }

    // Get information on page.
    let mut pageinfo: PageInfo = PageInfo::default();

    let result: u32 =
        memory::vminfo(vmem, memory::USER_BASE_ADDRESS, &mut pageinfo);

    // Check if we failed to get information on page.
    if result != 0 {
        nanvix::log!("failed to get information on page");
        return false;
    }

    // Attempt to change access permissions on page.
    let mode: AccessMode = AccessMode::new(false, true, false);
    let request: VmCtrlRequest =
        VmCtrlRequest::ChangePermissions(memory::USER_BASE_ADDRESS, mode);
    let result: u32 = memory::vmctrl(vmem, request);

    // Check if we failed to change access permissions on page.
    if result != 0 {
        nanvix::log!("failed to change access permissions on page");
        return false;
    }

    // Get information on page.
    let mut pageinfo: PageInfo = PageInfo::default();

    let result: u32 =
        memory::vminfo(vmem, memory::USER_BASE_ADDRESS, &mut pageinfo);

    // Check if we failed to get information on page.
    if result != 0 {
        nanvix::log!("failed to get information on page");
        return false;
    }

    // Check if page has expected information.
    if pageinfo.frame != frame {
        nanvix::log!("page has unexpected frame number");
        return false;
    }
    if !pageinfo.mode.read() {
        nanvix::log!("page has unexpected read permission");
        return false;
    }
    if !pageinfo.mode.write() {
        nanvix::log!("page has unexpected write permission");
        return false;
    }
    if !pageinfo.mode.exec() {
        nanvix::log!("page has unexpected exec permission");
        return false;
    }

    // Attempt to unmap the page frame from the virtual memory space.
    let result: u32 = memory::vmunmap(vmem, memory::USER_BASE_ADDRESS);

    // Check if we failed to unmap the page frame from the virtual memory space.
    if result != frame {
        nanvix::log!(
            "failed to unmap a page frame from a virtual memory space"
        );
        return false;
    }

    // Attempt to remove the virtual memory space.
    let result: u32 = memory::vmremove(vmem);

    // Check if we failed to remove the virtual memory space.
    if result != 0 {
        nanvix::log!("failed to remove a valid virtual memory space");
        return false;
    }

    true
}

//==============================================================================
// Public Standalone Functions
//==============================================================================

///
/// **Description**
///
/// Tests kernel calls in the memory management facility.
///
pub fn test() {
    crate::test!(check_sizes());
    crate::test!(alloc_free_frame());
    crate::test!(free_null_frame());
    crate::test!(free_invalid_frame());
    crate::test!(double_free_frame());
    crate::test!(create_remove_vmem());
    crate::test!(remove_null_vmem());
    crate::test!(map_unmap_vmem());
    crate::test!(change_page_permissions());
}
