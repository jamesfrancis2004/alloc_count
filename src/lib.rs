use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

mod macros;

std::thread_local! {
    static ALLOC_COUNT: Cell<usize> = const { Cell::new(0) };
    static IN_ALLOC: Cell<bool> = const { Cell::new(false) };
}

/// A lightweight, thread-local allocation counter wrapper around the standard `System` allocator.
pub struct AllocCounter;

unsafe impl GlobalAlloc for AllocCounter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        IN_ALLOC.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                // We only count allocations directly initialized from the user's thread, ignoring
                // recursion (which shouldn't happen during `in_alloc.set` on modern rust versions,
                // but we safeguard it just in case our TLS accesses cause allocations on some OSes).
                ALLOC_COUNT.with(|count| count.set(count.get() + 1));
                in_alloc.set(false);
            }
        });
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        IN_ALLOC.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                ALLOC_COUNT.with(|count| count.set(count.get() + 1));
                in_alloc.set(false);
            }
        });
        System.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        IN_ALLOC.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                ALLOC_COUNT.with(|count| count.set(count.get() + 1));
                in_alloc.set(false);
            }
        });
        System.realloc(ptr, layout, new_size)
    }
}

/// Returns the current thread's total number of recorded allocations.
pub fn count() -> usize {
    ALLOC_COUNT.with(|c| c.get())
}


