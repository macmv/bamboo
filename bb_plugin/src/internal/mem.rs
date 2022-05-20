//! Malloc and free, exposed to `bb_server` for various reasons.

/// Allocates with the given size and alignment. The returned pointer will be
/// dangling if the size is zero (`align` will simply be returned).
///
/// A null pointer means the allocation failed, and this plugin has run out of
/// memory. Depending on the allocator, the plugin may simple abort instead of
/// returning null.
#[no_mangle]
unsafe extern "C" fn malloc(size: u32, align: u32) -> *mut u8 {
  use std::alloc::{alloc, Layout};
  if size == 0 {
    align as _
  } else {
    alloc(Layout::from_size_align(size as usize, align as usize).unwrap())
  }
}

/// Deallocates the given pointer, using the size and align. This will do
/// nothing if the size is zero, as calling `alloc` will never be called for a
/// zero sized type.
///
/// # Safety
///
/// According to the docs of [`dealloc`](std::alloc::dealloc):
///
/// This function is unsafe because undefined behavior can result if the caller
/// does not ensure all of the following:
///
/// * `ptr` must denote a block of memory currently allocated via this
///   allocator,
///
/// * `layout` must be the same layout that was used to allocate that block of
///   memory.
///
/// All of this remains true, except for a `size` of zero. In this case,
/// `dealloc` will not be called.
#[no_mangle]
unsafe extern "C" fn free(ptr: *mut u8, size: u32, align: u32) {
  use std::alloc::{dealloc, Layout};
  if size != 0 {
    dealloc(ptr, Layout::from_size_align(size as usize, align as usize).unwrap())
  }
}
