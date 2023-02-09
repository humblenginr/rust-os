use core::{alloc::GlobalAlloc, ptr};

use super::{align_up, Locked};

/// A Bump Allocator is a very simple allocator that only allows the heap to grow linearly.
/// `next` will always point to the boundary between used and unused memory.
///
///       next->
/// ----------------------------
/// |xxxxx|    unused space    |
/// ----------------------------
/// ^heap_start                ^heap_end
///
/// ----------------------------
/// |xxxxx|xxxxx| unused space |
/// ----------------------------
///             ^next->
///
/// The next pointer always moves in a single direction and hence guarantees that the used memory
/// region will not be allocated twice.

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    // This function is defined as a const fn so that it can be used for initializing the static
    // ALLOCATOR
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    // This function is unsafe because the caller needs to guarantee that the memory region given
    // by the heap_start and heap_size bound is a valid one.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    // we can only get immutable reference to self in this trait function because we are defining
    // the allocator as a static variable and static variables are immutable.
    // To get around this problem, we wrap our BumpAllocator type in a Locked type.
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut bmp_allocator = self.lock();

        let alloc_start = align_up(bmp_allocator.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return ptr::null_mut(),
        };

        if alloc_end > bmp_allocator.heap_end {
            ptr::null_mut() // out of memory
        } else {
            bmp_allocator.next = alloc_end;
            bmp_allocator.allocations += 1;
            alloc_start as *mut u8
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut bump = self.lock(); // get a mutable reference

        bump.allocations -= 1;
        if bump.allocations == 0 {
            bump.next = bump.heap_start;
        }
    }
}
