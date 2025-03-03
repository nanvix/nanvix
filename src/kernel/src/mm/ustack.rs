// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::PageAligned;
use ::alloc::{
    fmt,
    rc::Rc,
};
use ::bitmap::Bitmap;
use ::config::memory_layout::{
    NUM_USER_STACK_ENTRIES,
    USER_BASE_RAW,
    USER_END_RAW,
    USER_STACK_BASE_RAW,
    USER_STACK_SIZE,
};
use ::core::cell::RefCell;
use ::sys::{
    error::Error,
    mm::{
        Address,
        VirtualAddress,
    },
};
use core::cell::RefMut;

//==================================================================================================
// User Stack
//==================================================================================================

///
/// # Description
///
/// A structure that represents a user stack.
///
pub struct UserStack {
    /// Base address.
    base: PageAligned<VirtualAddress>,
    /// Handle to stack allocator.
    allocator: Rc<RefCell<UserStackAllocatorInner>>,
}

impl UserStack {
    fn new(
        allocator: Rc<RefCell<UserStackAllocatorInner>>,
        base: PageAligned<VirtualAddress>,
    ) -> Result<Self, Error> {
        Ok(Self { base, allocator })
    }

    ///
    /// # Description
    ///
    /// Returns the size of the target stack.
    ///
    /// # Returns
    ///
    /// The size of the target stack.
    ///
    pub fn size(&self) -> usize {
        USER_STACK_SIZE
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the target stack.
    ///
    /// # Returns
    ///
    /// The base address of the target stack.
    ///
    /// # Notes
    ///
    /// As sacks grow downwards, the base address is the highest address of the stack.
    ///
    pub fn base(&self) -> PageAligned<VirtualAddress> {
        self.base
    }

    ///
    /// # Description
    ///
    /// Returns the top address of the target stack.
    ///
    /// # Returns
    ///
    /// The top address of the target stack.
    ///
    /// # Notes
    ///
    /// As stacks grow downwards, the top address is the lowest address of the stack.
    ///
    pub fn top(&self) -> PageAligned<VirtualAddress> {
        PageAligned::from_raw_value(self.base.into_raw_value() + self.size()).unwrap()
    }
}

impl fmt::Debug for UserStack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UserStack {{ base: {:?}, top: {:?}, size={:?} }}",
            self.base,
            self.top(),
            self.size()
        )
    }
}

impl Drop for UserStack {
    fn drop(&mut self) {
        debug!("drop(): {:?}", &self);
        let allocator = self.allocator.clone();
        if let Err(err) = allocator.borrow_mut().free(self) {
            error!("failed to free user stack: {:?}", err);
        };
    }
}

//==================================================================================================
// User Stack Allocator
//==================================================================================================

///
/// # Description
///
/// Inner state of a user stack allocator.
///
struct UserStackAllocatorInner {
    /// Base address.
    base: PageAligned<VirtualAddress>,
    /// Bitmap.
    bitmap: Bitmap,
}

impl UserStackAllocatorInner {
    ///
    /// # Description
    ///
    /// Allocates a user stack.
    ///
    /// # Returns
    ///
    /// On success, the newly allocated user stack. Otherwise, an error.
    ///
    /// # Notes
    ///
    /// After allocation, it is up to the caller to map the stack into the target virtual address
    /// space.
    ///
    pub fn alloc(&mut self) -> Result<PageAligned<VirtualAddress>, Error> {
        let index: usize = self.bitmap.alloc()?;
        PageAligned::from_address(self.base.into_inner() + (index * USER_STACK_SIZE))
    }

    ///
    /// # Description
    ///
    /// Frees a user stack.
    ///
    /// # Parameters
    ///
    /// - `user_stack`: The user stack to free.
    ///
    /// # Returns
    ///
    /// On success, the user stack is freed. Otherwise, an error.
    ///
    /// # Notes
    ///
    /// After freeing, it is up to the caller to unmap the stack from the target virtual address
    /// space.
    ///
    fn free(&mut self, user_stack: &mut UserStack) -> Result<(), Error> {
        let index: usize =
            (user_stack.base.into_raw_value() - self.base.into_raw_value()) / USER_STACK_SIZE;
        self.bitmap.clear(index)
    }
}

///
/// # Description
///
/// A structure that represents a user stack allocator.
///
pub struct UserStackAllocator {
    inner: Rc<RefCell<UserStackAllocatorInner>>,
}

impl UserStackAllocator {
    ///
    /// # Description
    ///
    /// Initializes a new user stack allocator.
    ///
    /// # Returns
    ///
    /// On success, the newly initialized user stack allocator. Otherwise, an error.
    ///
    pub fn new() -> Result<Self, Error> {
        const USER_STACK_TOP_RAW: usize =
            USER_STACK_BASE_RAW - USER_STACK_SIZE * NUM_USER_STACK_ENTRIES;

        ::sys::static_assert!(
            ::config::memory_layout::NUM_USER_STACK_ENTRIES % u8::BITS as usize == 0
        );
        ::sys::static_assert!(USER_STACK_BASE_RAW <= USER_END_RAW);
        ::sys::static_assert!(USER_STACK_TOP_RAW >= USER_BASE_RAW);

        let len: usize = ::config::memory_layout::NUM_USER_STACK_ENTRIES / u8::BITS as usize;
        let bitmap: Bitmap = Bitmap::new(len)?;

        Ok(Self {
            inner: Rc::new(RefCell::new(UserStackAllocatorInner {
                base: PageAligned::from_raw_value(USER_STACK_TOP_RAW)?,
                bitmap,
            })),
        })
    }

    ///
    /// # Description
    ///
    /// Allocates a user stack.
    ///
    /// # Returns
    ///
    /// On success, the newly allocated user stack. Otherwise, an error.
    ///
    pub fn alloc(&self) -> Result<UserStack, Error> {
        let mut inner: RefMut<'_, UserStackAllocatorInner> = self.inner.borrow_mut();
        let base: PageAligned<VirtualAddress> = inner.alloc()?;
        UserStack::new(self.inner.clone(), base)
    }
}
