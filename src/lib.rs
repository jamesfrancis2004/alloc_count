use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;

mod macros;

/// Detailed statistics about allocations performed on the current thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllocStats {
    pub alloc_calls: usize,
    pub dealloc_calls: usize,
    pub realloc_calls: usize,
    pub bytes_allocated: usize,
    pub bytes_deallocated: usize,
    pub bytes_reallocated: usize,
}

impl AllocStats {
    pub const fn new() -> Self {
        AllocStats {
            alloc_calls: 0,
            dealloc_calls: 0,
            realloc_calls: 0,
            bytes_allocated: 0,
            bytes_deallocated: 0,
            bytes_reallocated: 0,
        }
    }

    /// Subtracts the values of another `AllocStats` from this one, saturating at zero.
    /// Useful for computing the difference before and after a scope.
    pub fn saturating_sub(self, other: Self) -> Self {
        Self {
            alloc_calls: self.alloc_calls.saturating_sub(other.alloc_calls),
            dealloc_calls: self.dealloc_calls.saturating_sub(other.dealloc_calls),
            realloc_calls: self.realloc_calls.saturating_sub(other.realloc_calls),
            bytes_allocated: self.bytes_allocated.saturating_sub(other.bytes_allocated),
            bytes_deallocated: self.bytes_deallocated.saturating_sub(other.bytes_deallocated),
            bytes_reallocated: self.bytes_reallocated.saturating_sub(other.bytes_reallocated),
        }
    }
}

std::thread_local! {
    static ALLOC_STATS: Cell<AllocStats> = const { Cell::new(AllocStats::new()) };
    static IN_ALLOC: Cell<bool> = const { Cell::new(false) };
}

#[cfg(feature = "tokio")]
tokio::task_local! {
    #[doc(hidden)]
    pub static ASYNC_ALLOC_STATS: Cell<AllocStats>;
}

/// A generic, thread-local allocation counter wrapper around a standard `GlobalAlloc`.
/// It intercepts allocation calls, tallies them per-thread, and forwards them to the underlying allocator.
pub struct AllocCounter<A = std::alloc::System>(pub A);

unsafe impl<A> GlobalAlloc for AllocCounter<A> where A: GlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        IN_ALLOC.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                ALLOC_STATS.with(|stats| {
                    let mut current = stats.get();
                    current.alloc_calls += 1;
                    current.bytes_allocated += layout.size();
                    stats.set(current);
                });
                
                #[cfg(feature = "tokio")]
                let _ = ASYNC_ALLOC_STATS.try_with(|stats| {
                    let mut current = stats.get();
                    current.alloc_calls += 1;
                    current.bytes_allocated += layout.size();
                    stats.set(current);
                });
                
                in_alloc.set(false);
            }
        });
        self.0.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        IN_ALLOC.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                ALLOC_STATS.with(|stats| {
                    let mut current = stats.get();
                    current.dealloc_calls += 1;
                    current.bytes_deallocated += layout.size();
                    stats.set(current);
                });
                
                #[cfg(feature = "tokio")]
                let _ = ASYNC_ALLOC_STATS.try_with(|stats| {
                    let mut current = stats.get();
                    current.dealloc_calls += 1;
                    current.bytes_deallocated += layout.size();
                    stats.set(current);
                });

                in_alloc.set(false);
            }
        });
        self.0.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        IN_ALLOC.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                ALLOC_STATS.with(|stats| {
                    let mut current = stats.get();
                    current.alloc_calls += 1;
                    current.bytes_allocated += layout.size();
                    stats.set(current);
                });
                
                #[cfg(feature = "tokio")]
                let _ = ASYNC_ALLOC_STATS.try_with(|stats| {
                    let mut current = stats.get();
                    current.alloc_calls += 1;
                    current.bytes_allocated += layout.size();
                    stats.set(current);
                });

                in_alloc.set(false);
            }
        });
        self.0.alloc_zeroed(layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        IN_ALLOC.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                ALLOC_STATS.with(|stats| {
                    let mut current = stats.get();
                    current.realloc_calls += 1;
                    current.bytes_reallocated += new_size;
                    stats.set(current);
                });
                
                #[cfg(feature = "tokio")]
                let _ = ASYNC_ALLOC_STATS.try_with(|stats| {
                    let mut current = stats.get();
                    current.realloc_calls += 1;
                    current.bytes_reallocated += new_size;
                    stats.set(current);
                });

                in_alloc.set(false);
            }
        });
        self.0.realloc(ptr, layout, new_size)
    }
}

/// Returns the current thread's total recorded memory statistics.
pub fn stats() -> AllocStats {
    ALLOC_STATS.with(|s| s.get())
}
